#[cfg(test)]
mod light_client {
    mod test {
        use admin_controlled::AdminControlled;
        use light_client::{LightClient, PAUSE_ADD_BLOCK_HEADER};
        use near_sdk::test_utils::{accounts, VMContextBuilder};
        use near_sdk::{testing_env, AccountId};
        use test_utils::file_as_json;
        use types::{Block, Validator};
        use utils::hashes::encode_hex;

        const TEST_BLOCK_TIMESTAMP_MULTIPLIER: u64 = 100000000;

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

        fn init(blocks_to_keep: Option<usize>) -> LightClient {
            LightClient::new(blocks_to_keep)
        }

        #[test]
        fn block_hashes() {
            let mut context =
                get_context(accounts(0), 9605 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 9605);
            testing_env!(context.build());

            let mut bridge = init(None);
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
            let mut bridge = init(None);
            let validators = file_as_json::<Vec<Validator>>("validators.json").unwrap();
            let block93439858 = file_as_json::<Block>("block_93439858.json").unwrap();
            let block93447397 = file_as_json::<Block>("block_93447397.json").unwrap();
            let context_93439858 = get_context(
                accounts(0),
                93439858 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                93439858,
            );
            testing_env!(context_93439858.build());
            bridge.init_with_validators(validators);
            bridge.init_with_block(block93439858);

            let context_93447397 = get_context(
                accounts(0),
                93447397 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                93447397,
            );
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
            for i in 1..=3 {
                let mut bridge = init(Some(i));
                // Get "initial validators" that will produce block 304
                let block244 = file_as_json::<Block>("244.json").unwrap();
                let initial_validators = block244.next_bps.unwrap();

                let block304 = file_as_json::<Block>("304.json").unwrap();
                let block308 = file_as_json::<Block>("308.json").unwrap();

                let approvals_after_next = block308.approvals_after_next.clone();

                let context_244 =
                    get_context(accounts(0), 244 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 244);
                testing_env!(context_244.build());
                bridge.init_with_validators(initial_validators);

                let context_304 =
                    get_context(accounts(0), 304 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 304);
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

                let context_308 =
                    get_context(accounts(0), 308 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 308);
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

                if i == 1 {
                    assert!(bridge.block_merkle_roots(304).is_none())
                } else {
                    assert!(
                        encode_hex(&bridge.block_merkle_roots(304).unwrap())
                            == "5cbeabb6f5d6ddaeaa6250c82ff52a7858e8b0ce25de0593dd7b728becd7b102"
                    );
                }

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

        #[test]
        #[should_panic(expected = "paused")]
        fn test_panic_on_add_light_client_block_paused() {
            let mut bridge = init(None);
            // Get "initial validators" that will produce block 304
            let block244 = file_as_json::<Block>("244.json").unwrap();
            let initial_validators = block244.next_bps.unwrap();

            let block304 = file_as_json::<Block>("304.json").unwrap();
            let block308 = file_as_json::<Block>("308.json").unwrap();

            let context_244 = get_context(accounts(0), 244 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 244);
            testing_env!(context_244.build());
            bridge.init_with_validators(initial_validators);

            let context_304 = get_context(accounts(0), 304 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 304);
            testing_env!(context_304.build());
            bridge.init_with_block(block304);

            bridge.set_paused(PAUSE_ADD_BLOCK_HEADER);

            // switch context to accounts(1) which is not the admin account, and let that account try to add a block
            let context_308 = get_context(accounts(1), 308 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 308);
            testing_env!(context_308.build());
            bridge.add_light_client_block(block308);
        }

        #[test]
        fn test_rel46_multiple_epochs() {
            let mut bridge = init(None);
            let block105190359 = file_as_json::<Block>("block_105190359.json").unwrap();
            let initial_validators = block105190359.next_bps.unwrap();
            let context_105190359 = get_context(
                accounts(0),
                105190359 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105190359,
            );
            testing_env!(context_105190359.build());
            bridge.init_with_validators(initial_validators);
            let block105233559 = file_as_json::<Block>("block_105233559.json").unwrap();
            let context_105233559 = get_context(
                accounts(0),
                105233559 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105233559,
            );
            testing_env!(context_105233559.build());
            bridge.init_with_block(block105233559);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105233559).unwrap())
                    == "c348cdd4dfa14b1fcdff6688dd2321b2451c237f9eb38ee3353a724b832bb3f6"
            );
            let block105276759 = file_as_json::<Block>("block_105276759.json").unwrap();
            let context_105276759 = get_context(
                accounts(0),
                105276759 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105276759,
            );
            testing_env!(context_105276759.build());
            bridge.add_light_client_block(block105276759);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105276759).unwrap())
                    == "f5bd2431608fa4b190511d90d38032e129d8525725207e452eecebf737db3b1f"
            );
            let block105319959 = file_as_json::<Block>("block_105319959.json").unwrap();
            let context_105319959 = get_context(
                accounts(0),
                105319959 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105319959,
            );
            testing_env!(context_105319959.build());
            bridge.add_light_client_block(block105319959);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105319959).unwrap())
                    == "2866e29fdd356d173b89cf63d08277681c7cc5a2e480ad39772637c8f0e5c563"
            );
            let block105363159 = file_as_json::<Block>("block_105363159.json").unwrap();
            let context_105363159 = get_context(
                accounts(0),
                105363159 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105363159,
            );
            testing_env!(context_105363159.build());
            bridge.add_light_client_block(block105363159);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105363159).unwrap())
                    == "f8796917f0d6b06a3cfdaaf02f09a96dd849017a6c35fccea8e74ee65e57127e"
            );
            let block105406359 = file_as_json::<Block>("block_105406359.json").unwrap();
            let context_105406359 = get_context(
                accounts(0),
                105406359 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105406359,
            );
            testing_env!(context_105406359.build());
            bridge.add_light_client_block(block105406359);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105406359).unwrap())
                    == "06986ec25dc24ce2c6f43bd78bc65bec86cc94252b6afbbc862b222dceea621b"
            );
            let block105449559 = file_as_json::<Block>("block_105449559.json").unwrap();
            let context_105449559 = get_context(
                accounts(0),
                105449559 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105449559,
            );
            testing_env!(context_105449559.build());
            bridge.add_light_client_block(block105449559);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105449559).unwrap())
                    == "3f932a7145333f3545572bcb055dd9dc3b94096d457aef9228dcd9f1fa0f12c3"
            );
            let block105492759 = file_as_json::<Block>("block_105492759.json").unwrap();
            let context_105492759 = get_context(
                accounts(0),
                105492759 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105492759,
            );
            testing_env!(context_105492759.build());
            bridge.add_light_client_block(block105492759);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105492759).unwrap())
                    == "62766c842f2e34d3454e4a21c98c7b28b45dc102aeec5ef5ec3342f3669e68ec"
            );
            let block105535959 = file_as_json::<Block>("block_105535959.json").unwrap();
            let context_105535959 = get_context(
                accounts(0),
                105535959 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105535959,
            );
            testing_env!(context_105535959.build());
            bridge.add_light_client_block(block105535959);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105535959).unwrap())
                    == "8b8d3024bdf1154d8c2b9b38c1d7696d9efb160b1a8e1972715595775f1d9357"
            );
            let block105579159 = file_as_json::<Block>("block_105579159.json").unwrap();
            let context_105579159 = get_context(
                accounts(0),
                105579159 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105579159,
            );
            testing_env!(context_105579159.build());
            bridge.add_light_client_block(block105579159);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105579159).unwrap())
                    == "0eafd1931eeb12e1f59765eaa9cec03cfd93d638a693784d9896655389058693"
            );
            let block105622359 = file_as_json::<Block>("block_105622359.json").unwrap();
            let context_105622359 = get_context(
                accounts(0),
                105622359 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105622359,
            );
            testing_env!(context_105622359.build());
            bridge.add_light_client_block(block105622359);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105622359).unwrap())
                    == "732e95268c8eb236098ef498cc661b2c1bcc4289b9bf0b4bc4bd7496e1c507f4"
            );
            let block105665559 = file_as_json::<Block>("block_105665559.json").unwrap();
            let context_105665559 = get_context(
                accounts(0),
                105665559 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105665559,
            );
            testing_env!(context_105665559.build());
            bridge.add_light_client_block(block105665559);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105665559).unwrap())
                    == "7d5f584aafbbf6738938326894bfa6f0974de5d4e530a5893513f9fee13c7314"
            );
            let block105708759 = file_as_json::<Block>("block_105708759.json").unwrap();
            let context_105708759 = get_context(
                accounts(0),
                105708759 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105708759,
            );
            testing_env!(context_105708759.build());
            bridge.add_light_client_block(block105708759);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105708759).unwrap())
                    == "e3e6e819669a2c860c66fad0412ea43c8e6d3196d6a0106d4847362cb03b62dc"
            );
            let block105738334 = file_as_json::<Block>("block_105738334.json").unwrap();
            let context_105738334 = get_context(
                accounts(0),
                105738334 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105738334,
            );
            testing_env!(context_105738334.build());
            bridge.add_light_client_block(block105738334);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105738334).unwrap())
                    == "fa3373b9464290e43a4253766d7937dd1e55ffd96eb5303d27b5d0f847e3a972"
            );
            let block105738335 = file_as_json::<Block>("block_105738335.json").unwrap();
            let context_105738335 = get_context(
                accounts(0),
                105738335 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105738335,
            );
            testing_env!(context_105738335.build());
            bridge.add_light_client_block(block105738335);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105738335).unwrap())
                    == "d4baecd2cbec159752836944f138ba795e07c207e968cc259f9d8ac6cc031763"
            );
        }

        #[test]
        fn test_brdg121_multiple_epochs() {
            let mut bridge = init(None);
            let block105363159 = file_as_json::<Block>("block_105363159.json").unwrap();
            let initial_validators = block105363159.next_bps.unwrap();
            let context_105363159 = get_context(
                accounts(0),
                105363159 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105363159,
            );
            testing_env!(context_105363159.build());
            bridge.init_with_validators(initial_validators);
            let block105406359 = file_as_json::<Block>("block_105406359.json").unwrap();
            let context_105406359 = get_context(
                accounts(0),
                105406359 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105406359,
            );
            testing_env!(context_105406359.build());
            bridge.init_with_block(block105406359);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105406359).unwrap())
                    == "06986ec25dc24ce2c6f43bd78bc65bec86cc94252b6afbbc862b222dceea621b"
            );
            let block105449559 = file_as_json::<Block>("block_105449559.json").unwrap();
            let context_105449559 = get_context(
                accounts(0),
                105449559 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105449559,
            );
            testing_env!(context_105449559.build());
            bridge.add_light_client_block(block105449559);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105449559).unwrap())
                    == "3f932a7145333f3545572bcb055dd9dc3b94096d457aef9228dcd9f1fa0f12c3"
            );
            let block105492759 = file_as_json::<Block>("block_105492759.json").unwrap();
            let context_105492759 = get_context(
                accounts(0),
                105492759 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105492759,
            );
            testing_env!(context_105492759.build());
            bridge.add_light_client_block(block105492759);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105492759).unwrap())
                    == "62766c842f2e34d3454e4a21c98c7b28b45dc102aeec5ef5ec3342f3669e68ec"
            );
            let block105535959 = file_as_json::<Block>("block_105535959.json").unwrap();
            let context_105535959 = get_context(
                accounts(0),
                105535959 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105535959,
            );
            testing_env!(context_105535959.build());
            bridge.add_light_client_block(block105535959);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105535959).unwrap())
                    == "8b8d3024bdf1154d8c2b9b38c1d7696d9efb160b1a8e1972715595775f1d9357"
            );
            let block105579159 = file_as_json::<Block>("block_105579159.json").unwrap();
            let context_105579159 = get_context(
                accounts(0),
                105579159 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105579159,
            );
            testing_env!(context_105579159.build());
            bridge.add_light_client_block(block105579159);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105579159).unwrap())
                    == "0eafd1931eeb12e1f59765eaa9cec03cfd93d638a693784d9896655389058693"
            );
            let block105622359 = file_as_json::<Block>("block_105622359.json").unwrap();
            let context_105622359 = get_context(
                accounts(0),
                105622359 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105622359,
            );
            testing_env!(context_105622359.build());
            bridge.add_light_client_block(block105622359);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105622359).unwrap())
                    == "732e95268c8eb236098ef498cc661b2c1bcc4289b9bf0b4bc4bd7496e1c507f4"
            );
            let block105665559 = file_as_json::<Block>("block_105665559.json").unwrap();
            let context_105665559 = get_context(
                accounts(0),
                105665559 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105665559,
            );
            testing_env!(context_105665559.build());
            bridge.add_light_client_block(block105665559);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105665559).unwrap())
                    == "7d5f584aafbbf6738938326894bfa6f0974de5d4e530a5893513f9fee13c7314"
            );
            let block105708759 = file_as_json::<Block>("block_105708759.json").unwrap();
            let context_105708759 = get_context(
                accounts(0),
                105708759 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105708759,
            );
            testing_env!(context_105708759.build());
            bridge.add_light_client_block(block105708759);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105708759).unwrap())
                    == "e3e6e819669a2c860c66fad0412ea43c8e6d3196d6a0106d4847362cb03b62dc"
            );
            let block105739252 = file_as_json::<Block>("block_105739252.json").unwrap();
            let context_105739252 = get_context(
                accounts(0),
                105739252 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105739252,
            );
            testing_env!(context_105739252.build());
            bridge.add_light_client_block(block105739252);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105739252).unwrap())
                    == "d7e701a9c6975e2763373d3e8dcb7814cb9ab736827c1828719c2c098a933444"
            );
            let block105739253 = file_as_json::<Block>("block_105739253.json").unwrap();
            let context_105739253 = get_context(
                accounts(0),
                105739253 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105739253,
            );
            testing_env!(context_105739253.build());
            bridge.add_light_client_block(block105739253);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105739253).unwrap())
                    == "8fb3939abf1f8d4614851d115a439a4ac02351079ce0a84f8e889f9aaea7b662"
            );
        }

        #[test]
        #[should_panic(expected="Epoch id of the block is not valid")]
        fn test_brdg121_skip_epoch_panic() {
            let mut bridge = init(None);
            let block105363159 = file_as_json::<Block>("block_105363159.json").unwrap();
            let initial_validators = block105363159.next_bps.unwrap();
            let context_105363159 = get_context(
                accounts(0),
                105363159 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105363159,
            );
            testing_env!(context_105363159.build());
            bridge.init_with_validators(initial_validators);
            let block105406359 = file_as_json::<Block>("block_105406359.json").unwrap();
            let context_105406359 = get_context(
                accounts(0),
                105406359 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105406359,
            );
            testing_env!(context_105406359.build());
            bridge.init_with_block(block105406359);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105406359).unwrap())
                    == "06986ec25dc24ce2c6f43bd78bc65bec86cc94252b6afbbc862b222dceea621b"
            );
            let block105492759 = file_as_json::<Block>("block_105492759.json").unwrap();
            let context_105492759 = get_context(
                accounts(0),
                105492759 * TEST_BLOCK_TIMESTAMP_MULTIPLIER,
                105492759,
            );
            testing_env!(context_105492759.build());
            bridge.add_light_client_block(block105492759);
            assert!(
                encode_hex(&bridge.block_merkle_roots(105492759).unwrap())
                    == "62766c842f2e34d3454e4a21c98c7b28b45dc102aeec5ef5ec3342f3669e68ec"
            );
        }
    }
}
