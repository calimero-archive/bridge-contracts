pub mod macros;
pub use macros::*;

use near_sdk::env;

pub type Mask = u64;

pub trait AdminControlled {
    fn is_owner(&self) -> bool {
        env::current_account_id() == env::predecessor_account_id()
    }

    /// Return the current mask representing all paused events.
    fn get_paused(&self) -> Mask;

    /// Update mask with all paused events.
    /// Implementor is responsible for guaranteeing that this function can only be
    /// called by owner of the contract.
    fn set_paused(&mut self, paused: Mask);

    /// Return if the contract is paused for the current flag and user
    fn is_paused(&self, flag: Mask) -> bool {
        (self.get_paused() & flag) != 0 && !self.is_owner()
    }

    /// Asserts if the contract is paused for the current flag and user
    fn assert_not_paused(&self, flag: Mask) {
        assert!(!self.is_paused(flag));
    }

    /// Asserts if the contract is paused for the current flag
    fn assert_not_paused_flags(&self, flag: Mask) {
        assert!((self.get_paused() & flag) == 0);
    }
}
