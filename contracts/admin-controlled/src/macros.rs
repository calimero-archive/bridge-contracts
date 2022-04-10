#[macro_export]
macro_rules! impl_admin_controlled {
    ($contract: ident, $paused: ident) => {
        use admin_controlled::{AdminControlled, Mask};

        #[near_bindgen]
        impl AdminControlled for $contract {
            #[result_serializer(borsh)]
            fn get_paused(&self) -> Mask {
                self.$paused
            }

            #[result_serializer(borsh)]
            fn set_paused(&mut self, #[serializer(borsh)] paused: Mask) {
                near_sdk::assert_self();
                self.$paused = paused;
            }
        }
    };
}
