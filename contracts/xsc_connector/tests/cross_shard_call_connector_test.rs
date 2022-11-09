#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::serde_json;
        use near_sdk::serde_json::json;
        use near_sdk::Gas;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use types::ConnectorType;
        use utils::hashes::{decode_hex};
        use utils::Hash;
        use types::{FullOutcomeProof};
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker};

        const TIC_TAC_TOE_ACCOUNT_90: &str = "dev-1666269241011-86059863198522";
        const TIC_TAC_TOE_ACCOUNT_REL42: &str = "dev-1668382397562-28997434477113";

        const LOCKER_ACCOUNT_90: &str = "xscc.90.calimero.testnet";
        const LOCKER_ACCOUNT_REL42: &str = "xsc_connector.rel42.calimero.testnet";
        

        async fn init(tic_tac_toe_account: &str, locker_account: &str) -> (Worker<Sandbox>, Contract, Contract, Contract, Contract) {
            let worker = workspaces::sandbox().await.unwrap();
            // deploy contracts
            let prover_wasm = std::fs::read(
                "../mock_prover/target/wasm32-unknown-unknown/release/mock_prover.wasm",
            )
                .unwrap();
            let prover_contract = worker.dev_deploy(&prover_wasm).await.unwrap();
            let connector_permissions_wasm = std::fs::read(
                "../wasm/connector_permissions.wasm",
            )
                .unwrap();
            let connector_permissions_contract = worker.dev_deploy(&connector_permissions_wasm).await.unwrap();

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
            let tla = workspaces::AccountId::try_from(tic_tac_toe_account.to_string()).unwrap();

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

            connector_permissions_contract
                .call(&worker, "new")
                .args_json(json!({
                    "ft_connector_account": "nft_connector_not_relevant_for_this_test",
                    "nft_connector_account": "nft_connector_not_relevant_for_this_test",
                    "xsc_connector_account": connector_contract.id().to_string(),
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            connector_contract
                .call(&worker, "new")
                .args_json(json!({
                    "prover_account": prover_contract.id().to_string(),
                    "connector_permissions_account": connector_permissions_contract.id().to_string(),
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            connector_contract
                .call(&worker, "set_locker")
                .args_json(json!({
                "locker_account": locker_account,
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

            (worker, prover_contract, connector_contract, tic_tac_toe_contract, connector_permissions_contract)
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
            assert!(cross_call_execute_result.logs()[2] == format!("CALIMERO_EVENT_CROSS_RESPONSE:testtictactoe.90.calimero.testnet:callback_game_ended::{}", TIC_TAC_TOE_ACCOUNT_90));

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
            let expected_block_merkle_root: Hash = decode_hex("3681f812e71c92e11cf92bf19494916f994be8688a182b9b078e9025ac68f046").try_into().unwrap();

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
                .call(&worker, "cross_call_receive_response")
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
            assert!(cross_call_receive_response_result.logs()[0] == "RecordProof:3681f812e71c92e11cf92bf19494916f994be8688a182b9b078e9025ac68f046");
            assert!(cross_call_receive_response_result.logs()[1] == "GOT THE CALLBACK WITH EXEC RESULT 1");
        }

        #[tokio::test]
        async fn test_cross_call_execute() {
            let (worker, prover, connector, tic_tac_toe_contract, _connector_permissions) = init(TIC_TAC_TOE_ACCOUNT_90, LOCKER_ACCOUNT_90).await;
            let cross_call_execute_proof = &file_as_json::<FullOutcomeProof>("test_assets/cross_call_execute_proof.json").unwrap();
            cross_call_execute(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_execute_proof).await;
        }

        #[tokio::test]
        async fn test_cross_call_receive_response() {
            let (worker, prover, connector, tic_tac_toe_contract, _connector_permissions) = init(TIC_TAC_TOE_ACCOUNT_REL42, LOCKER_ACCOUNT_REL42).await;
            let cross_call_receive_response_proof = &file_as_json::<FullOutcomeProof>("test_assets/cross_call_receive_response_proof.json").unwrap();
            cross_call_receive_response(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_receive_response_proof).await;
        }

        #[tokio::test]
        #[should_panic]
        async fn test_proof_reuse_panics() {
            let (worker, prover, connector, tic_tac_toe_contract, _connector_permissions) = init(TIC_TAC_TOE_ACCOUNT_REL42, LOCKER_ACCOUNT_REL42).await;
            let cross_call_receive_response_proof = &file_as_json::<FullOutcomeProof>("test_assets/cross_call_receive_response_proof.json").unwrap();
            cross_call_receive_response(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_receive_response_proof).await;

            // should panic since reusing proof
            cross_call_receive_response(&worker, &prover, &connector, &tic_tac_toe_contract, &cross_call_receive_response_proof).await;
        }

        #[tokio::test]
        async fn test_cross_call_for_denied_account() {
            let (worker, _prover, connector, _tic_tac_toe_contract, connector_permissions) = init(TIC_TAC_TOE_ACCOUNT_90, LOCKER_ACCOUNT_90).await;

            const ALICE_ACCOUNT_ID: &str= "dev-1000000000001-10000000000001";

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_alice");
            let tla = workspaces::AccountId::try_from(ALICE_ACCOUNT_ID.to_string()).unwrap();
            let alice_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

            let deny_result = connector.as_account()
                .call(&worker, connector_permissions.id(), "deny_bridge")
                .args_json(json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "connector_type": ConnectorType::XSC,
                 }))
                .unwrap()
                .transact()
                .await
                .unwrap();
            assert!(deny_result.is_success());

            let cross_call_result_for_denied_account = alice_account
                .call(&worker, connector.id(), "cross_call")
                .args_json(json!({
                    "destination_contract_id": TIC_TAC_TOE_ACCOUNT_90,
                    "destination_contract_method": "start_game",
                    "destination_contract_args": json!({"player_a":"player_a.testnet","player_b":"player_b.testnet"}).to_string(),
                    "destination_gas": Gas(20_000_000_000_000),
                    "destination_deposit": 0,
                    "source_callback_method": "game_started"
                 }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            // There should have been no logs
            assert!(cross_call_result_for_denied_account.logs().len() == 0);

            // Allow Alice to use the cross shard connector again
            let allow_result = connector.as_account()
                .call(&worker, connector_permissions.id(), "allow_bridge")
                .args_json(json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "connector_type": ConnectorType::XSC,
                 }))
                .unwrap()
                .transact()
                .await
                .unwrap();
            assert!(allow_result.is_success());

            // Try cross_call again
            let cross_call_result_for_allowed_account = alice_account
                .call(&worker, connector.id(), "cross_call")
                .args_json(json!({
                    "destination_contract_id": TIC_TAC_TOE_ACCOUNT_90,
                    "destination_contract_method": "start_game",
                    "destination_contract_args": json!({
                        "player_a": "player_a.testnet",
                        "player_b": "player_b.testnet",
                    }).to_string(),
                    "destination_gas": Gas(20_000_000_000_000),
                    "destination_deposit": 0,
                    "source_callback_method": "game_started"
                 }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            // There should have been just one log
            assert!(cross_call_result_for_allowed_account.logs().len() == 1);
            assert!(cross_call_result_for_allowed_account.is_success());

            println!("{}", cross_call_result_for_allowed_account.logs()[0]);

            // verify CALIMERO_EVENT_CROSS_CALL event was emitted
            let parts: Vec<&str> = cross_call_result_for_allowed_account.logs()[0].split(":").collect();
            assert_eq!(parts.len(), 8);
            assert!(parts[0] == "CALIMERO_EVENT_CROSS_CALL");
            assert!(parts[1] == TIC_TAC_TOE_ACCOUNT_90);
            assert!(parts[2] == "start_game");
            assert!(parts[3] == base64::encode(json!({"player_a":"player_a.testnet","player_b":"player_b.testnet"}).to_string())); 
            assert!(parts[4] == "20000000000000");
            assert!(parts[5] == "0");
            assert!(parts[6] == ALICE_ACCOUNT_ID);
            assert!(parts[7] == "game_started");
        }
    }
}
