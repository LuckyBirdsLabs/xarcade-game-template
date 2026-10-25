//! Tests of the template game on its own. The hub is played by a plain account (`HUB`): the game only checks
//! that the caller is the address it was deployed with, so no real hub is needed here.
//!
//! Run them with:  sc-meta all build  &&  cargo test
//! (build first: the tests run the compiled contract in `output/`).
//!
//! When you change the rules, change these tests with them. The ones named `control_hub_rules_*` are what the
//! xArcade hub REQUIRES of every game: keep them passing.

use multiversx_sc_scenario::imports::*;
use xarcade_game_template::types::GameResolveResult;

const CODE: MxscPath = MxscPath::new("output/xarcade-game-template.mxsc.json");

const OWNER: TestAddress = TestAddress::new("owner");
const HUB: TestAddress = TestAddress::new("hub");
const PLAYER: TestAddress = TestAddress::new("player");
const OTHER: TestAddress = TestAddress::new("other");
const GAME: TestSCAddress = TestSCAddress::new("game");

const ONE: u64 = 1_000_000_000_000_000_000;
const USER_ERROR: u64 = 4;
const PVE: u8 = 0;
const PVP: u8 = 1;
/// Must match `PAYOUT_MULTIPLIER` in `src/play.rs`.
const PAYOUT_MULTIPLIER: u64 = 5;

type Api = StaticApi;

fn egld(n: u64) -> BigUint<Api> {
    BigUint::<Api>::from(n) * ONE
}

fn world() -> ScenarioWorld {
    let mut w = ScenarioWorld::new();
    w.register_contract(CODE, xarcade_game_template::ContractBuilder);
    w.account(OWNER).nonce(1).balance(egld(10));
    w.account(HUB).nonce(1).balance(egld(10));
    w.account(PLAYER).nonce(1).balance(egld(10));
    w.account(OTHER).nonce(1).balance(egld(10));
    w.tx().from(OWNER).raw_deploy().code(CODE).argument(&HUB.to_address()).new_address(GAME).run();
    w
}

/// Pin the randomness the next calls run with: `resolve` reads the block's random seed.
fn set_seed(w: &mut ScenarioWorld, nonce: u64, seed_byte: u8) {
    let mut s = [0u8; 48];
    s[0] = seed_byte;
    w.current_block().block_nonce(nonce).block_random_seed(ManagedBuffer::<Api>::from(&s[..]));
}

/// What the hub sends when a player starts a game. `status`: the user error the call must fail with, if any.
#[allow(clippy::too_many_arguments)]
fn join(
    w: &mut ScenarioWorld,
    caller: TestAddress,
    game_id: u64,
    is_new: bool,
    nonce: u64,
    wager: BigUint<Api>,
    mode: u8,
    data: &[u8],
    status: Option<u64>,
) -> Vec<EgldOrEsdtTokenPayment<Api>> {
    let tx = w
        .tx()
        .from(caller)
        .to(GAME)
        .raw_call("joinGame")
        .argument(&game_id)
        .argument(&is_new)
        .argument(&PLAYER.to_address())
        .argument(&EgldOrEsdtTokenIdentifier::<Api>::egld())
        .argument(&nonce)
        .argument(&wager)
        .argument(&mode)
        .argument(&ManagedBuffer::<Api>::from(data))
        .original_result::<ManagedVec<Api, EgldOrEsdtTokenPayment<Api>>>();
    match status {
        Some(code) => {
            tx.returns(ExpectStatus(code)).run();
            vec![]
        }
        None => tx.returns(ReturnsResult).run().into_iter().collect(),
    }
}

fn join_ok(w: &mut ScenarioWorld, game_id: u64, wager: BigUint<Api>, pick: u8) -> Vec<EgldOrEsdtTokenPayment<Api>> {
    join(w, HUB, game_id, true, 0, wager, PVE, &[pick], None)
}

fn resolve(w: &mut ScenarioWorld, caller: TestAddress, game_id: u64) -> GameResolveResult<Api> {
    w.tx()
        .from(caller)
        .to(GAME)
        .raw_call("resolve")
        .argument(&game_id)
        .original_result::<GameResolveResult<Api>>()
        .returns(ReturnsResult)
        .run()
}

fn open_games(w: &mut ScenarioWorld) -> usize {
    w.query()
        .to(GAME)
        .raw_call("getGames")
        .original_result::<MultiValueEncoded<Api, MultiValue2<u64, xarcade_game_template::types::Game<Api>>>>()
        .returns(ReturnsResult)
        .run()
        .into_iter()
        .count()
}

fn is_zero(address: &ManagedAddress<Api>) -> bool {
    address.is_zero()
}

// ===========================================================================================================
// What the hub requires of every game.
// ===========================================================================================================

#[test]
fn control_hub_rules_only_the_hub_can_call_every_endpoint() {
    let mut w = world();
    join_ok(&mut w, 1, egld(1), 3);

    for caller in [PLAYER, OTHER, OWNER] {
        join(&mut w, caller, 2, true, 0, egld(1), PVE, &[3], Some(USER_ERROR));
        w.tx().from(caller).to(GAME).raw_call("resolve").argument(&1u64).returns(ExpectStatus(USER_ERROR)).run();
        w.tx().from(caller).to(GAME).raw_call("leaveGame").argument(&1u64).returns(ExpectStatus(USER_ERROR)).run();
        w.tx().from(caller).to(GAME).raw_call("cancelGame").argument(&1u64).returns(ExpectStatus(USER_ERROR)).run();
    }
    assert_eq!(open_games(&mut w), 1, "nothing a stranger did changed the game");
}

#[test]
fn control_hub_rules_the_reservation_is_the_most_it_can_pay() {
    let mut w = world();
    let reserved = join_ok(&mut w, 1, egld(2), 4);

    assert_eq!(reserved.len(), 1, "one reservation: what a win would pay");
    assert!(reserved[0].token_identifier.is_egld());
    assert_eq!(reserved[0].token_nonce, 0);
    assert_eq!(reserved[0].amount, egld(2) * PAYOUT_MULTIPLIER);
}

#[test]
fn control_hub_rules_a_payout_never_exceeds_the_reservation_and_the_house_has_no_address() {
    let mut w = world();
    let (mut wins, mut losses) = (0, 0);
    for i in 0..60u64 {
        set_seed(&mut w, i + 1, i as u8);
        let reserved = join_ok(&mut w, i + 1, egld(1), 1 + (i % 6) as u8);
        let result = resolve(&mut w, HUB, i + 1);

        assert!(result.payout.amount <= reserved[0].amount, "payout above what was reserved");
        assert!(is_zero(&result.loser), "PvE has no loser");
        if is_zero(&result.winner) {
            losses += 1; // the house won: the zero address
        } else {
            wins += 1;
            assert_eq!(result.winner, PLAYER.to_managed_address(), "only the player can win");
            assert_eq!(result.payout.amount, egld(1) * PAYOUT_MULTIPLIER);
        }
    }
    assert!(wins > 0 && losses > 0, "over 60 different seeds both outcomes happen");
    assert!(wins < losses, "a 1 in 6 chance wins less often than it loses");
    assert_eq!(open_games(&mut w), 0, "every resolved game is deleted");
}

#[test]
fn control_hub_rules_a_game_settles_once_and_cancel_never_fails() {
    let mut w = world();
    join_ok(&mut w, 1, egld(1), 2);
    resolve(&mut w, HUB, 1);
    w.tx().from(HUB).to(GAME).raw_call("resolve").argument(&1u64).returns(ExpectStatus(USER_ERROR)).run();

    // `cancelGame` on a game that is gone (already settled, or already cancelled) is fine, any number of times.
    join_ok(&mut w, 2, egld(1), 2);
    for _ in 0..3 {
        w.tx().from(HUB).to(GAME).raw_call("cancelGame").argument(&2u64).run();
    }
    assert_eq!(open_games(&mut w), 0);
    w.tx().from(HUB).to(GAME).raw_call("resolve").argument(&2u64).returns(ExpectStatus(USER_ERROR)).run();
}

// ===========================================================================================================
// The rules of the example game. YOUR GAME: replace these with tests of yours.
// ===========================================================================================================

#[test]
fn control_a_pick_must_be_a_face_of_the_die() {
    let mut w = world();
    for bad in [0u8, 7, 255] {
        join(&mut w, HUB, 1, true, 0, egld(1), PVE, &[bad], Some(USER_ERROR));
    }
    join(&mut w, HUB, 1, true, 0, egld(1), PVE, &[], Some(USER_ERROR));
    join(&mut w, HUB, 1, true, 0, egld(1), PVE, &[1, 2], Some(USER_ERROR));
    for good in 1u8..=6 {
        join_ok(&mut w, 10 + u64::from(good), egld(1), good);
    }
    assert_eq!(open_games(&mut w), 6);
}

#[test]
fn control_it_refuses_what_it_cannot_play() {
    let mut w = world();
    join(&mut w, HUB, 1, true, 0, egld(1), PVP, &[3], Some(USER_ERROR)); // PvE only
    join(&mut w, HUB, 1, false, 0, egld(1), PVE, &[3], Some(USER_ERROR)); // nothing to join
    join(&mut w, HUB, 1, true, 7, egld(1), PVE, &[3], Some(USER_ERROR)); // an NFT cannot pay 5x
    join(&mut w, HUB, 1, true, 0, BigUint::<Api>::zero(), PVE, &[3], Some(USER_ERROR)); // nothing at stake
    join_ok(&mut w, 1, egld(1), 3);
    join(&mut w, HUB, 1, true, 0, egld(1), PVE, &[3], Some(USER_ERROR)); // the id is taken
    assert_eq!(open_games(&mut w), 1);
}

#[test]
fn control_leaving_is_refused_because_there_are_no_lobbies() {
    let mut w = world();
    join_ok(&mut w, 1, egld(1), 3);
    w.tx().from(HUB).to(GAME).raw_call("leaveGame").argument(&1u64).returns(ExpectStatus(USER_ERROR)).run();
    assert_eq!(open_games(&mut w), 1, "the game is untouched");
}
