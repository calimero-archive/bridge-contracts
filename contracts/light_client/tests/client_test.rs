#[cfg(test)]
mod light_client {
    mod test {
        use light_client::hashes::encode_hex;
        use light_client::{Block, BlockHeaderInnerLite, LightClient, Validator, Signature};
        use near_sdk::serde::de::DeserializeOwned;
        use near_sdk::serde_json::Value;
        use near_sdk::test_utils::{accounts, VMContextBuilder};
        use near_sdk::{testing_env, AccountId};
        use std::error::Error;
        use std::fs::File;
        use std::io::BufReader;

        const TEST_BLOCK_TIMESTAMP_MULTIPLIER: u64 = 100000000;
        const TEST_LOCK_DURATION: u64 = 10;
        const TEST_REPLACE_DURATION: u64 = 20000000000;

        fn get_context(predecessor_account_id: AccountId, block_timestamp: u64, block_index: u64) -> VMContextBuilder {
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

        fn file_as_json<T: DeserializeOwned>(filename: &str) -> Result<T, Box<dyn Error>> {
            let file = File::open(format!("./tests/{}", filename))?;
            let reader = BufReader::new(file);
            let value = near_sdk::serde_json::from_reader(reader)?;

            return Ok(value);
        }

        fn array_to_validators(validators: Option<&Vec<Value>>) -> Vec<Validator> {
            let mut ret = Vec::<Validator>::new();
            for item in validators.unwrap() {
                ret.push(Validator {
                    account_id: String::from(item["account_id"].as_str().unwrap()),
                    public_key: String::from(item["public_key"].as_str().unwrap()),
                    stake: item["stake"].as_str().unwrap().parse::<u128>().unwrap(),
                    is_chunk_only: false,
                });
            }

            return ret;
        }

        fn array_to_signatures(signatures: Option<&Vec<Value>>) -> Vec<Option<Signature>> {
            let mut ret = Vec::<Option<Signature>>::new();
            for item in signatures.unwrap() {
                let val = item.as_str();
                if val.is_none() {
                    ret.push(None);
                } else {
                    ret.push(Some(Signature { signature: val.unwrap().to_string() }));
                }
            }
            return ret;
        }

        fn value_to_block(block_value: &Value) -> Block {
            Block::new(
                String::from(block_value["prev_block_hash"].as_str().unwrap()),
                String::from(block_value["next_block_inner_hash"].as_str().unwrap()),
                BlockHeaderInnerLite::new(
                    block_value["inner_lite"]["height"].as_u64().unwrap(),
                    String::from(block_value["inner_lite"]["epoch_id"].as_str().unwrap()),
                    String::from(block_value["inner_lite"]["next_epoch_id"].as_str().unwrap()),
                    String::from(
                        block_value["inner_lite"]["prev_state_root"]
                            .as_str()
                            .unwrap(),
                    ),
                    String::from(block_value["inner_lite"]["outcome_root"].as_str().unwrap()),
                    String::from(block_value["inner_lite"]["timestamp"].as_str().unwrap()),
                    String::from(block_value["inner_lite"]["next_bp_hash"].as_str().unwrap()),
                    String::from(
                        block_value["inner_lite"]["block_merkle_root"]
                            .as_str()
                            .unwrap(),
                    ),
                ),
                String::from(block_value["inner_rest_hash"].as_str().unwrap()),
                Some(array_to_validators(block_value["next_bps"].as_array())),
                array_to_signatures(block_value["approvals_after_next"].as_array()),
            )
        }

        #[test]
        fn block_hashes() {
            let mut context = get_context(accounts(0), 9605 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 9605);
            testing_env!(context.build());

            let mut bridge = init();
            let block9605 = file_as_json::<Value>("./block_9605.json").unwrap();
            let block9610 = file_as_json::<Value>("./block_9610.json").unwrap();

            let initial_validators = array_to_validators(block9605["next_bps"].as_array());

            bridge.init_with_validators(initial_validators);
            bridge.init_with_block(value_to_block(&block9605));

            assert!(
                encode_hex(&bridge.block_hashes(9605).unwrap())
                    == "c4770276d5e782d847ea3ce0674894a572df3ea75b960ff57d66395df0eb2a34"
            );

            //get_context(accounts(0), 9605 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 9605);
            testing_env!(context
                .block_timestamp(9610 * TEST_BLOCK_TIMESTAMP_MULTIPLIER)
                .block_index(9610)
                .build());
            bridge.add_light_client_block(value_to_block(&block9610));

            let some_future_block_index = 9620;
            testing_env!(context
                .block_timestamp(some_future_block_index * TEST_BLOCK_TIMESTAMP_MULTIPLIER)
                .block_index(some_future_block_index)
                .build());

            assert!(encode_hex(&bridge.block_hashes(9610).unwrap()) == "f28629da269e59f2494c6bf283e9e67dadaa1c1f753607650d21e5e5b916a0dc");
        }

        #[test]
        fn adding_block_in_first_epoch() {
            let mut bridge = init();
            // Get "initial validators" that will produce block 304
            let block244 = file_as_json::<Value>("244.json").unwrap();
            let initial_validators = array_to_validators(block244["next_bps"].as_array());

            let block304 = file_as_json::<Value>("304.json").unwrap();
            // TODO add correct type
            let block308 = file_as_json::<Value>("308.json").unwrap();

            let approvals_after_next = block308["approvals_after_next"].as_array();

            let context_244 = get_context(accounts(0), 244 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 244);
            testing_env!(context_244.build());
            bridge.init_with_validators(initial_validators);

            let context_304 = get_context(accounts(0), 304 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 304);
            testing_env!(context_304.build());
            bridge.init_with_block(value_to_block(&block304));

            bridge.block_hashes(304);
            assert!(
                encode_hex(&bridge.block_hashes(304).unwrap())
                    == "ea43feedc69d8df45d6afcb25cf428ab0ba8044dd818586e48979797f5f55a01"
            );

            let context_308 = get_context(accounts(0), 308 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 308);
            testing_env!(context_308.build());
            bridge.add_light_client_block(value_to_block(&block308));

            let context_future_320 = get_context(accounts(0), 320 * TEST_BLOCK_TIMESTAMP_MULTIPLIER, 320);
            testing_env!(context_future_320.build());

            // TODO uncomment when add_light_client_block will do something
            assert!(encode_hex(&bridge.block_hashes(308).unwrap()) == "92c231eb7719d7cc7598e7bc614bbd0eb0be3729b47a36ede4a66033aa5051d9");

            for key_value in approvals_after_next.unwrap() {
                if !key_value.is_null() {
                    let key = String::from(key_value.as_str().unwrap());

                    // TODO uncomment when check_block_producer_signature_in_head will exist and do something
                    //assert!(bridge.check_block_producer_signature_in_head(key))
                }
            }

            assert!(true)
        }
    }
}
