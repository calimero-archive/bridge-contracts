#[cfg(test)]
mod light_client {
    mod test {
        use utils::hashes::encode_hex;
        use light_client::LightClient;
        use types::{
            Block, Validator,
        };
        use near_sdk::test_utils::{accounts, VMContextBuilder};
        use near_sdk::{testing_env, AccountId};
        use test_utils::file_as_json;

        const TEST_BLOCK_TIMESTAMP_MULTIPLIER: u64 = 100000000;
        const TEST_LOCK_DURATION: u64 = 10;
        const TEST_REPLACE_DURATION: u64 = 20000000000;

        fn get_context(
            predecessor_account_id: AccountId,
            block_timestamp: u64,
            block_index: u64,
        ) -> VMContextBuilder {
            let mut builder = VMContextBuilder::new();
            builder
                .current_account_id(accounts(0))
                .signer_account_id(predecessor_account_id.clone())
                .predecessor_account_id(predecessor_account_id)
                .block_timestamp(block_timestamp)
                .block_index(block_index);
            builder
        }

        fn init() -> LightClient {
            LightClient::new(TEST_LOCK_DURATION, TEST_REPLACE_DURATION)
        }

        #[test]
        fn block_hashes() {
            let mut context =
                get_context(accounts(0), 9605 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 9605);
            testing_env!(context.build());

            let mut bridge = init();
            let block9605 = file_as_json::<Block>("./block_9605.json").unwrap();
            let block9610 = file_as_json::<Block>("./block_9610.json").unwrap();

            let initial_validators = block9605.next_bps.as_ref().unwrap();

            bridge.init_with_validators(initial_validators.to_vec());
            bridge.init_with_block(block9605);

            assert!(
                encode_hex(&bridge.block_hashes(9605).unwrap())
                    == "c4770276d5e782d847ea3ce0674894a572df3ea75b960ff57d66395df0eb2a34"
            );

            //get_context(accounts(0), 9605 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 9605);
            testing_env!(context
                .block_timestamp(9610 * TEST_BLOCK_TIMESTAMP_MULTIPLIER)
                .block_index(9610)
                .build());
            bridge.add_light_client_block(block9610);

            let some_future_block_index = 9620;
            testing_env!(context
                .block_timestamp(some_future_block_index * TEST_BLOCK_TIMESTAMP_MULTIPLIER)
                .block_index(some_future_block_index)
                .build());

            assert!(
                encode_hex(&bridge.block_hashes(9610).unwrap())
                    == "f28629da269e59f2494c6bf283e9e67dadaa1c1f753607650d21e5e5b916a0dc"
            );
        }

        #[test]
        fn check_signature() {
            let mut bridge = init();
            let validators = file_as_json::<Vec<Validator>>("validators.json").unwrap();
            let block93439858 = file_as_json::<Block>("block_93439858.json").unwrap();
            let block93447397 = file_as_json::<Block>("block_93447397.json").unwrap();
            let context_93439858 = get_context(accounts(0), 93439858 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 93439858);
            testing_env!(context_93439858.build());
            bridge.init_with_validators(validators);
            bridge.init_with_block(block93439858);

            let context_93447397 = get_context(accounts(0), 93447397 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 93447397);
            testing_env!(context_93447397.build());
            bridge.add_light_client_block(block93447397.clone());


            let mut i = 0;
            let approvals_after_next = block93447397.approvals_after_next;
            for key in approvals_after_next {
                if !key.is_none() {
                    assert!(bridge.check_block_producer_signature_in_head(i))
                }
                i += 1;
            }

            assert!(true);
        }

        #[test]
        fn adding_block_in_first_epoch() {
            let mut bridge = init();
            // Get "initial validators" that will produce block 304
            let block244 = file_as_json::<Block>("244.json").unwrap();
            let initial_validators = block244.next_bps.unwrap();

            let block304 = file_as_json::<Block>("304.json").unwrap();
            let block308 = file_as_json::<Block>("308.json").unwrap();

            let approvals_after_next = block308.approvals_after_next.clone();

            let context_244 = get_context(accounts(0), 244 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 244);
            testing_env!(context_244.build());
            bridge.init_with_validators(initial_validators);

            let context_304 = get_context(accounts(0), 304 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 304);
            testing_env!(context_304.build());
            bridge.init_with_block(block304);

            bridge.block_hashes(304);
            assert!(
                encode_hex(&bridge.block_hashes(304).unwrap())
                    == "ea43feedc69d8df45d6afcb25cf428ab0ba8044dd818586e48979797f5f55a01"
            );

            assert!(
                encode_hex(&bridge.block_merkle_roots(304).unwrap())
                    == "5cbeabb6f5d6ddaeaa6250c82ff52a7858e8b0ce25de0593dd7b728becd7b102"
            );

            let context_308 = get_context(accounts(0), 308 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 308);
            testing_env!(context_308.build());
            bridge.add_light_client_block(block308);

            let context_future_320 =
                get_context(accounts(0), 320 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 320);
            testing_env!(context_future_320.build());

            assert!(
                encode_hex(&bridge.block_hashes(308).unwrap())
                    == "92c231eb7719d7cc7598e7bc614bbd0eb0be3729b47a36ede4a66033aa5051d9"
            );

            assert!(
                encode_hex(&bridge.block_merkle_roots(308).unwrap())
                    == "7e4e19fea8f998800da1bd289f7e420395f2f32fb5683237deaf6d6a3ecfbdae"
            );

            assert!(
                encode_hex(&bridge.block_merkle_roots(304).unwrap())
                    == "5cbeabb6f5d6ddaeaa6250c82ff52a7858e8b0ce25de0593dd7b728becd7b102"
            );

            let mut i = 0;
            for key in approvals_after_next {
                if !key.is_none() {
                    assert!(bridge.check_block_producer_signature_in_head(i))
                }
                i += 1;
            }

            assert!(true)
        }
    }
}
