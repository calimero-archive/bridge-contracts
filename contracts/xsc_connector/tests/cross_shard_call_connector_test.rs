#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::serde_json;
        use near_sdk::serde_json::json;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use utils::hashes::{decode_hex};
        use utils::Hash;
        use types::{FullOutcomeProof};
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker};

        const TIC_TAC_TOE_CONTRACT_ACCOUNT_ID: &str = "dev-1666269241011-86059863198522";

        async fn init() -> (Worker<Sandbox>, Contract, Contract, Contract) {
            let worker = workspaces::sandbox().await.unwrap();
            // deploy contracts
            let prover_wasm = std::fs::read(
                "../mock_prover/target/wasm32-unknown-unknown/release/mock_prover.wasm",
            )
                .unwrap();
            let prover_contract = worker.dev_deploy(&prover_wasm).await.unwrap();
            let connector_wasm = std::fs::read(
                "./target/wasm32-unknown-unknown/release/xsc_connector.wasm",
            )
                .unwrap();
            let connector_contract = worker.dev_deploy(&connector_wasm).await.unwrap();
            let tic_tac_toe_wasm = std::fs::read(
                "./tests/test_assets/tictactoe/tic_tac_toe.wasm",
            )
                .unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_1");
            let tla = workspaces::AccountId::try_from(TIC_TAC_TOE_CONTRACT_ACCOUNT_ID.to_string()).unwrap();

            let tic_tac_toe_contract = worker.create_tla_and_deploy(tla, sec, &tic_tac_toe_wasm)
                .await
                .unwrap()
                .unwrap();

            // initialize contracts
            prover_contract
                .call(&worker, "new")
                .args_json(json!({}))
                .unwrap()
                .transact()
                .await
                .unwrap();

            connector_contract
                .call(&worker, "new")
                .args_json(json!({
                "prover_account": prover_contract.id().to_string(),
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            connector_contract
                .call(&worker, "set_locker")
                .args_json(json!({
                "locker_account": "xscc.90.calimero.testnet",
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            tic_tac_toe_contract
                .call(&worker, "new")
                .transact()
                .await
                .unwrap();

            (worker, prover_contract, connector_contract, tic_tac_toe_contract)
        }

        async fn cross_call_execute(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, contract_making_cross_shard_calls: &Contract, cross_call_execute_proof: &FullOutcomeProof) {
            let expected_block_merkle_root: Hash = decode_hex("f596ebe3e36802cd905f53bb44a09c83feaad206ae6d8535262b1f4c4d5c00bc").try_into().unwrap();

                prover
                    .call(&worker, "add_approved_hash")
                    .args_json(json!({
                    "hash": expected_block_merkle_root,
                }))
                    .unwrap()
                    .transact()
                    .await
                    .unwrap();

            // This game has not ended yet, so should not exist under this contract
            let ended_game_before_cross_call: serde_json::Value = worker.view(
                contract_making_cross_shard_calls.id(),
                "get_game",
                serde_json::to_vec(&serde_json::json!({
                    "game_id": 0
                        })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(ended_game_before_cross_call.to_string() == "null");

            let cross_call_execute_result = connector
                .call(&worker, "cross_call_execute")
                .args_json(json!({
                    "proof": cross_call_execute_proof,
                    "height": 9999, // not important for mock prover
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("3 NEAR") as u128)
                .transact()
                .await
                .unwrap();

            assert!(cross_call_execute_result.logs().len() == 3);
            assert!(cross_call_execute_result.logs()[0] == "RecordProof:f596ebe3e36802cd905f53bb44a09c83feaad206ae6d8535262b1f4c4d5c00bc");
            assert!(cross_call_execute_result.logs()[1] == format!("game ended, called by the connector {}", connector.id().to_string()));
            assert!(cross_call_execute_result.logs()[2] == "CALIMERO_EVENT_CROSS_RESPONSE:testtictactoe.90.calimero.testnet:callback_game_ended:");

            let ended_game_after_cross_call: serde_json::Value = worker.view(
                contract_making_cross_shard_calls.id(),
                "get_game",
                serde_json::to_vec(&serde_json::json!({
                    "game_id": 0
                        })).unwrap())
                .await.unwrap()
                .json().unwrap();

            println!("Stored ended game after cross call {}", ended_game_after_cross_call);
            let expected_output: serde_json::Value = json!({
                "board":[
                    ["O","X","O"],
                    ["X","X","O"],
                    ["O","O","X"]
                ],
                "player_a":"igi.testnet",
                "player_a_turn":false,
                "player_b":"mikimaus.testnet",
                "status":"Tie"
            });
            assert!(ended_game_after_cross_call.to_string() == expected_output.to_string());
        }

        async fn cross_call_receive_response(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, _contract_making_cross_shard_calls: &Contract, cross_call_receive_response_proof: &FullOutcomeProof) {
            let expected_block_merkle_root: Hash = decode_hex("b0b9a40654dfd0daff7829f71314e08eb27bafcd1c65de3fe13136afdbddeea1").try_into().unwrap();

            prover
                .call(&worker, "add_approved_hash")
                .args_json(json!({
                    "hash": expected_block_merkle_root,
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            let cross_call_receive_response_result = connector
                .call(&worker, "cross_call_receive_reponse")
                .args_json(json!({
                    "proof": cross_call_receive_response_proof,
                    "height": 9999, // not important for mock prover
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1 NEAR") as u128)
                .transact()
                .await
                .unwrap();

            assert!(cross_call_receive_response_result.logs().len() == 2);
            assert!(cross_call_receive_response_result.logs()[0] == "RecordProof:b0b9a40654dfd0daff7829f71314e08eb27bafcd1c65de3fe13136afdbddeea1");
            assert!(cross_call_receive_response_result.logs()[1] == "GOT THE CALLBACK WITH EXEC RESULT 0");
        }

        #[tokio::test]
        async fn test_cross_call_execute() {
            let (worker, prover, connector, tic_tac_toe_contract) = init().await;
            let cross_call_execute_proof = &file_as_json::<FullOutcomeProof>("test_assets/cross_call_execute_proof.json").unwrap();
            cross_call_execute(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_execute_proof).await;
        }

        #[tokio::test]
        async fn test_cross_call_receive_response() {
            let (worker, prover, connector, tic_tac_toe_contract) = init().await;
            let cross_call_receive_response_proof = &file_as_json::<FullOutcomeProof>("test_assets/cross_call_receive_response_proof.json").unwrap();
            cross_call_receive_response(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_receive_response_proof).await;
        }

        #[tokio::test]
        #[should_panic]
        async fn test_proof_reuse_panics() {
            let (worker, prover, connector, tic_tac_toe_contract) = init().await;
            let cross_call_receive_response_proof = &file_as_json::<FullOutcomeProof>("test_assets/cross_call_receive_response_proof.json").unwrap();
            cross_call_receive_response(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_receive_response_proof).await;

            // should panic since reusing proof
            cross_call_receive_response(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_receive_response_proof).await;
        }
    }
}
