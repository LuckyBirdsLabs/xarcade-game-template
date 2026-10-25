multiversx_sc::imports!();

use crate::types::Game;

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(getHubAddress)]
    #[storage_mapper("hubAddress")]
    fn hub_address(&self) -> SingleValueMapper<ManagedAddress>;

    /// The games that are open right now, by game id. A game is removed when it is resolved or cancelled.
    #[view(getGames)]
    #[storage_mapper("games")]
    fn games(&self) -> MapMapper<u64, Game<Self::Api>>;
}
