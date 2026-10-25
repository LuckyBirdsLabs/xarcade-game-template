multiversx_sc::imports!();
multiversx_sc::derive_imports!();

/// How the hub tells a game what kind of game this is. The ORDER of the variants is part of the interface
/// with the hub (PvE is 0, PvP is 1): do not reorder or add variants.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum GameMode {
    PvE,
    PvP,
}

/// What a game keeps about one open game.
///
/// YOUR GAME: add the fields your rules need (a pick, a difficulty, a chosen side...). Keep `player`, `wager`,
/// `token` and `token_nonce`: `resolve` needs them to pay out.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Debug)]
pub struct Game<M: ManagedTypeApi> {
    pub player: ManagedAddress<M>,
    /// The face of the die the player bet on (1 to 6).
    pub pick: u8,
    pub wager: BigUint<M>,
    pub token: EgldOrEsdtTokenIdentifier<M>,
    pub token_nonce: u64,
}

/// What `resolve` hands back to the hub. The FIELD ORDER is part of the interface with the hub: do not change it.
///
/// PvE: `winner` is the player, or the zero address when the house wins; `loser` is the zero address;
///      `payout` is what the player receives (never more than was reserved when they joined).
/// PvP: `winner` and `loser` are the two players; `payout` is ignored (the winner takes the whole pot).
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Debug)]
pub struct GameResolveResult<M: ManagedTypeApi> {
    pub winner: ManagedAddress<M>,
    pub loser: ManagedAddress<M>,
    pub payout: EgldOrEsdtTokenPayment<M>,
}
