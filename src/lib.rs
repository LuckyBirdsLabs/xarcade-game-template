#![no_std]

multiversx_sc::imports!();

pub mod play;
pub mod storage;
pub mod types;

/// A game for xArcade. It holds RULES ONLY: every token sits in the xArcade hub, and every endpoint here can
/// only be called by the hub, never by a player. To make your own game, change `play.rs` (see the `YOUR GAME`
/// markers) and keep everything else as it is.
#[multiversx_sc::contract]
pub trait XarcadeGameTemplate: storage::StorageModule + play::PlayModule {
    /// `hub_address` is the xArcade hub this game will be listed on. Every call from anyone else is refused.
    #[init]
    fn init(&self, hub_address: ManagedAddress) {
        self.hub_address().set(&hub_address);
    }

    #[upgrade]
    fn upgrade(&self, hub_address: ManagedAddress) {
        self.hub_address().set(&hub_address);
    }
}
