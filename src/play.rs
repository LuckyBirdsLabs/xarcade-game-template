multiversx_sc::imports!();

use crate::types::{Game, GameMode, GameResolveResult};

// ===========================================================================================================
// YOUR GAME: THE RULES. This example is "Lucky Number": the player bets on one face of a six-sided die and the
// contract rolls it. Change the numbers below, or replace the rules in `join_game` and `resolve`.
// ===========================================================================================================

/// A right guess pays this many times the wager. With a fair six-sided die the player wins 1 time in 6, so
/// the house keeps an edge of 1 - 5/6 (about 17%). Pick a payout that leaves your house an edge, and that you
/// can fund: the hub locks `wager x PAYOUT_MULTIPLIER` of your bankroll for every game that is open.
const PAYOUT_MULTIPLIER: u32 = 5;

/// How many faces the die has. The player picks a number from 1 to this.
const DIE_FACES: u64 = 6;

#[multiversx_sc::module]
pub trait PlayModule: crate::storage::StorageModule {
    /// Called by the hub when a player starts a game. `data` is whatever the player sent along with the bet:
    /// here one byte, the face they pick (1 to 6).
    ///
    /// The return value is the most this game could ever have to PAY OUT for this player. The hub locks that
    /// much of the bankroll, and in `resolve` it refuses a payout above it. So: payout <= reservation, always.
    ///
    /// The hub has already checked the bet is inside the limits it set for this game and has taken the stake.
    #[endpoint(joinGame)]
    fn join_game(
        &self,
        game_id: u64,
        is_new: bool,
        player: ManagedAddress,
        token: EgldOrEsdtTokenIdentifier,
        token_nonce: u64,
        wager: BigUint,
        mode: GameMode,
        data: ManagedBuffer,
    ) -> ManagedVec<Self::Api, EgldOrEsdtTokenPayment<Self::Api>> {
        self.require_hub();

        // YOUR GAME: this example is player-versus-house only. For a PvP game, see `xarcade-sc-coinflip`.
        require!(mode == GameMode::PvE, "This game is PvE only");
        require!(is_new, "This game has nothing to join");
        // A win pays several times the wager, which cannot exist for an NFT or SFT.
        require!(token_nonce == 0, "Only fungible tokens can be wagered");
        require!(wager > 0u32, "Wager must be above zero");

        require!(data.len() == 1, "Invalid data");
        let mut pick_byte = [0u8; 1];
        data.load_slice(0, &mut pick_byte);
        let pick = pick_byte[0];
        require!(pick >= 1 && u64::from(pick) <= DIE_FACES, "Pick a number from 1 to 6");

        require!(!self.games().contains_key(&game_id), "Game ID already exists");
        let reservation = EgldOrEsdtTokenPayment::new(token.clone(), token_nonce, &wager * PAYOUT_MULTIPLIER);
        self.games().insert(game_id, Game { player: player.clone(), pick, wager, token, token_nonce });
        self.game_created_event(game_id, &player, pick);

        let mut reserved = ManagedVec::new();
        reserved.push(reservation);
        reserved
    }

    /// Called by the hub to settle a game: decides who won, and deletes the game.
    ///
    /// Only the hub can call it, and the hub only lets its trusted resolver do so, so a player cannot settle
    /// their own game over and over until they win.
    ///
    /// RANDOMNESS: the roll comes from the block's random seed, mixed with the game id. Only use what a player
    /// cannot choose or predict: never a value the player sent, the caller's address, or the block timestamp.
    /// The resolver does choose WHEN a game settles, and so which block's seed is used: that is an accepted
    /// assumption of the platform, which your code cannot remove.
    #[endpoint(resolve)]
    fn resolve(&self, game_id: u64) -> GameResolveResult<Self::Api> {
        self.require_hub();
        let game = self.games().get(&game_id).unwrap_or_else(|| sc_panic!("Game does not exist"));

        let rolled = self.random_below(game_id, DIE_FACES) + 1;
        let player_wins = rolled == u64::from(game.pick);

        // PvE: the house has no address, so a house win is the zero address.
        let winner = if player_wins { game.player.clone() } else { ManagedAddress::zero() };
        // Exactly what was reserved at join, so it can never be above it.
        let payout = EgldOrEsdtTokenPayment::new(game.token.clone(), game.token_nonce, &game.wager * PAYOUT_MULTIPLIER);

        self.games().remove(&game_id);
        self.game_resolved_event(game_id, &winner, rolled);
        GameResolveResult { winner, loser: ManagedAddress::zero(), payout }
    }

    /// Called by the hub when a PvP lobby creator leaves it. This example has no PvP lobbies, so the hub never
    /// reaches it; it must exist all the same.
    #[endpoint(leaveGame)]
    fn leave_game(&self, _game_id: u64) {
        self.require_hub();
        sc_panic!("This game has no lobbies to leave");
    }

    /// Called by the hub when a game was never settled and the player took their stake back. Drop everything
    /// you keep about the game. It must not fail if the game is already gone, and it must not depend on anything
    /// that could stop it: a game that refuses here can leave a player's stake stuck, and that is the first thing
    /// the audit looks for.
    #[endpoint(cancelGame)]
    fn cancel_game(&self, game_id: u64) {
        self.require_hub();
        self.games().remove(&game_id);
    }

    /// A number from 0 to `modulus` - 1, from the seed of the block this call runs in and the game id.
    fn random_below(&self, game_id: u64, modulus: u64) -> u64 {
        let mut input = ManagedBuffer::new();
        input.append(self.blockchain().get_block_random_seed().as_managed_buffer());
        input.append(&ManagedBuffer::from(&game_id.to_be_bytes()[..]));
        let digest = self.crypto().sha256(&input).to_byte_array();
        let number = u64::from_be_bytes([digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]]);
        number % modulus
    }

    /// Every endpoint above starts with this. It is what keeps players from calling the game directly.
    fn require_hub(&self) {
        require!(self.blockchain().get_caller() == self.hub_address().get(), "Only hub can call");
    }

    /// Events are optional, but they are how the platform and your players can follow a game.
    #[event("game_created")]
    fn game_created_event(&self, #[indexed] game_id: u64, #[indexed] player: &ManagedAddress, #[indexed] pick: u8);

    #[event("game_resolved")]
    fn game_resolved_event(&self, #[indexed] game_id: u64, #[indexed] winner: &ManagedAddress, #[indexed] rolled: u64);
}
