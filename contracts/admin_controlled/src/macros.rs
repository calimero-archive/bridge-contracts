#[macro_export]
macro_rules! impl_admin_controlled {
    ($contract: ident, $paused: ident) => {
        use admin_controlled::{AdminControlled as AdminControlledInner};
        use near_sdk as near_sdk_inner;

        #[near_bindgen]
        impl AdminControlledInner for $contract {
            fn get_paused(&self) -> Mask {
                self.$paused
            }

            #[private]
            fn set_paused(&mut self, paused: Mask) {
                self.$paused = paused;
            }
        }
    };
}
