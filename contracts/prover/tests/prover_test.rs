#[cfg(test)]
mod prover {
    use utils::Hash;
    use prover::RemoteLightClient;
    use std::collections::HashMap;
    struct MockLightClient {
        merkle_roots: HashMap<u64, Hash>,
    }

    impl RemoteLightClient for MockLightClient {
        fn block_merkle_roots(&self, height: u64) -> Hash {
            match self.merkle_roots.get(&height) {
                Some(x) => *x,
                _ => Default::default(),
            }
        }
    }

    mod test {
        use utils::hashes::{decode_hex, deserialize_hash};
        use utils::Hash;
        use near_sdk::serde::de::DeserializeOwned;
        use near_sdk::serde_json::Value;
        use near_sdk::serde_json;
        use near_sdk::test_utils::{accounts, VMContextBuilder};
        use near_sdk::{testing_env, AccountId};
        use prover::Prover;
        use std::error::Error;
        use std::fs::File;
        use std::io::BufReader;
        use types::{
            BlockHeaderInnerLite, BlockHeaderLight, ExecutionOutcome, ExecutionOutcomeWithId,
            ExecutionOutcomeWithIdAndProof, ExecutionStatus, FullOutcomeProof, MerklePath,
            MerklePathItem,
        };

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

        fn init() -> Prover {
            Prover::new(String::from("debeligad.testnet"))
        }

        fn file_as_json<T: DeserializeOwned>(filename: &str) -> Result<T, Box<dyn Error>> {
            let file = File::open(format!("./tests/{}", filename))?;
            let reader = BufReader::new(file);
            let value = near_sdk::serde_json::from_reader(reader)?;

            return Ok(value);
        }

        pub fn block_header_inner_lite_from_tests(
            height: u64,
            epoch_id: String,
            next_epoch_id: String,
            prev_state_root: String,
            outcome_root: String,
            timestamp: String,
            next_bp_hash: String,
            block_merkle_root: String,
        ) -> BlockHeaderInnerLite {
            BlockHeaderInnerLite {
                height: height,
                epoch_id: deserialize_hash(&epoch_id).unwrap(),
                next_epoch_id: deserialize_hash(&next_epoch_id).unwrap(),
                prev_state_root: deserialize_hash(&prev_state_root).unwrap(),
                outcome_root: deserialize_hash(&outcome_root).unwrap(),
                timestamp: timestamp.parse().unwrap(),
                next_bp_hash: deserialize_hash(&next_bp_hash).unwrap(),
                block_merkle_root: deserialize_hash(&block_merkle_root).unwrap(),
            }
        }

        fn execution_status_from_tests(execution_status_value: &Value) -> ExecutionStatus {
            if !execution_status_value["Unknown"].is_null() {
                ExecutionStatus::Unknown()
            } else if !execution_status_value["Failed"].is_null() {
                ExecutionStatus::Failed()
            } else if !execution_status_value["SuccessValue"].is_null() {
                let data: Vec<u8> = base64::decode(&String::from(
                    execution_status_value["SuccessValue"].as_str().unwrap(),
                ))
                .unwrap();
                ExecutionStatus::SuccessValue(data)
            } else {
                // TODO
                ExecutionStatus::SuccessReceiptId(Default::default())
            }
        }

        fn execution_outcome_from_tests(execution_outcome_value: &Value) -> ExecutionOutcome {
            let mut receipt_ids: Vec<Hash> = Vec::new();
            for item in execution_outcome_value["receipt_ids"].as_array().unwrap() {
                receipt_ids.push(deserialize_hash(&String::from(item.as_str().unwrap())).unwrap());
            }
            ExecutionOutcome {
                logs: Vec::new(),
                receipt_ids: receipt_ids,
                gas_burnt: execution_outcome_value["gas_burnt"].as_u64().unwrap(),
                tokens_burnt: execution_outcome_value["tokens_burnt"]
                    .as_str()
                    .unwrap()
                    .parse::<u128>()
                    .unwrap(),
                executor_id: String::from(execution_outcome_value["executor_id"].as_str().unwrap()),
                status: execution_status_from_tests(&execution_outcome_value["status"]),
            }
        }

        fn merkle_path_from_tests(merkle_array: Option<&Vec<Value>>) -> MerklePath {
            let mut items: Vec<MerklePathItem> = Vec::new();
            for item in merkle_array.unwrap() {
                items.push(MerklePathItem {
                    hash: deserialize_hash(&String::from(item["hash"].as_str().unwrap())).unwrap(),
                    direction: if item["hash"].as_str().unwrap() == "Left" {
                        types::MERKLE_PATH_LEFT
                    } else {
                        types::MERKLE_PATH_RIGHT
                    },
                })
            }
            MerklePath { items }
        }

        fn outcome_proof_from_tests(outcome_proof_value: &Value) -> ExecutionOutcomeWithIdAndProof {
            ExecutionOutcomeWithIdAndProof {
                proof: merkle_path_from_tests(outcome_proof_value["proof"].as_array()),
                block_hash: Default::default(),
                outcome_with_id: ExecutionOutcomeWithId {
                    id: deserialize_hash(&String::from(
                        outcome_proof_value["id"].as_str().unwrap(),
                    ))
                    .unwrap(),
                    outcome: execution_outcome_from_tests(&outcome_proof_value["outcome"]),
                },
            }
        }

        fn block_header_light_from_tests(header_value: &Value) -> BlockHeaderLight {
            BlockHeaderLight {
                prev_block_hash: deserialize_hash(&String::from(
                    header_value["prev_block_hash"].as_str().unwrap(),
                ))
                .unwrap(),
                inner_rest_hash: deserialize_hash(&String::from(
                    header_value["inner_rest_hash"].as_str().unwrap(),
                ))
                .unwrap(),
                inner_lite: block_header_inner_lite_from_tests(
                    header_value["inner_lite"]["height"].as_u64().unwrap(),
                    String::from(header_value["inner_lite"]["epoch_id"].as_str().unwrap()),
                    String::from(
                        header_value["inner_lite"]["next_epoch_id"]
                            .as_str()
                            .unwrap(),
                    ),
                    String::from(
                        header_value["inner_lite"]["prev_state_root"]
                            .as_str()
                            .unwrap(),
                    ),
                    String::from(header_value["inner_lite"]["outcome_root"].as_str().unwrap()),
                    String::from(header_value["inner_lite"]["timestamp_nanosec"].as_str().unwrap()),
                    String::from(header_value["inner_lite"]["next_bp_hash"].as_str().unwrap()),
                    String::from(
                        header_value["inner_lite"]["block_merkle_root"]
                            .as_str()
                            .unwrap(),
                    ),
                ),
            }
        }

        fn value_to_full_outcome_proof(proof_value: &Value) -> FullOutcomeProof {
            FullOutcomeProof {
                outcome_proof: outcome_proof_from_tests(&proof_value["outcome_proof"]),
                outcome_root_proof: MerklePath { items: Vec::new() },
                block_header_lite: block_header_light_from_tests(&proof_value["block_header_lite"]),
                block_proof: merkle_path_from_tests(proof_value["block_proof"].as_array()),
            }
        }

        use near_sdk::env::promise_return;

        // if call does not panic, it is considered valid, since return is always true
        fn proof_valid(filename: &str, block_height: u64, _block_merkle_root: Hash) {
            let prover = init();
            let proof = value_to_full_outcome_proof(&file_as_json::<Value>(filename).unwrap());
            //println!("{}", serde_json::to_string(&proof).unwrap());

            prover.prove_outcome(proof, block_height);
            let ret = promise_return(0);

            // TODO remove and replace with real assert, when tests will cover external contract
            println!("RET {:?}", ret);

            assert!(true);
        }

        // TODO upgrade tests to:
        // 1. use external contract, currently that part is skipped
        // 2. have examples where logs are not empty
        // 3. have examples where execution status is not SuccessValue

        #[test]
        fn proof2() {
            proof_valid(
                "proof2.json",
                498,
                decode_hex("22f00dd154366d758cd3e4fe81c1caed8e0db6227fe4b2b52a8e5a468aa0a723")
                    .try_into()
                    .unwrap(),
            );
        }
        #[test]
        fn proof3() {
            proof_valid(
                "proof3.json",
                1705,
                decode_hex("0d0776820a9a81481a559c36fd5d69c33718fb7d7fd3be7564a446e043e2cb35")
                    .try_into()
                    .unwrap(),
            );
        }
        #[test]
        fn proof4() {
            proof_valid(
                "proof4.json",
                5563,
                decode_hex("1f7129496c461c058fb3daf258d89bf7dacb4efad5742351f66098a00bb6fa53")
                    .try_into()
                    .unwrap(),
            );
        }
        #[test]
        fn proof5() {
            proof_valid(
                "proof5.json",
                384,
                decode_hex("a9cd8eb4dd92ba5f2fef47d68e1d73ac8c57047959f6f8a2dcc664419e74e4b8")
                    .try_into()
                    .unwrap(),
            );
        }
        #[test]
        fn proof6() {
            proof_valid(
                "proof6.json",
                377,
                decode_hex("cc3954a51b7c1a86861df8809f79c2bf839741e3e380e28360b8b3970a5d90bd")
                    .try_into()
                    .unwrap(),
            );
        }

        #[test]
        fn proof7() {
            proof_valid(
                "proof7.json",
                93544034,
                decode_hex("cc3954a51b7c1a86861df8809f79c2bf839741e3e380e28360b8b3970a5d90bd") // TODO replace with real merkle root for this proof
                    .try_into()
                    .unwrap(),
            );
        }
    }
}
