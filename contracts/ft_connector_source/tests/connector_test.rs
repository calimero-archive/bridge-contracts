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
        use workspaces::result::CallExecutionDetails;

        const FT_CONTRACT_ACCOUNT_ID: &str = "dev-1661337044068-74633164378532";
        const ALICE_ACCOUNT_ID: &str = "dev-1656412997567-26565713922485";

        async fn init() -> (Worker<Sandbox>, Contract, Contract, Contract) {
            let worker = workspaces::sandbox().await.unwrap();
            // deploy contracts
            let prover_wasm = std::fs::read(
                "../mock_prover/target/wasm32-unknown-unknown/release/mock_prover.wasm",
            )
                .unwrap();
            let prover_contract = worker.dev_deploy(&prover_wasm).await.unwrap();
            let connector_wasm = std::fs::read(
                "./target/wasm32-unknown-unknown/release/ft_connector_source.wasm",
            )
                .unwrap();
            let connector_contract = worker.dev_deploy(&connector_wasm).await.unwrap();
            let fungible_token_wasm = std::fs::read(
                "./tests/fungible_token.wasm",
            )
                .unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_1");
            let tla = workspaces::AccountId::try_from(FT_CONTRACT_ACCOUNT_ID.to_string()).unwrap();

            let fungible_token_contract = worker.create_tla_and_deploy(tla, sec, &fungible_token_wasm)
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
                "source_master_account": "todo_remove_this_field",
                "destination_master_account": "todo_remove_this_field"
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            connector_contract
                .call(&worker, "set_locker")
                .args_json(json!({
                "locker_account": "ftdc.n.calimero.testnet",
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            fungible_token_contract
                .call(&worker, "new")
                .args_json(json!({
                "owner_id": fungible_token_contract.id().to_string(), 
                "total_supply": "1000000000000000", 
                "metadata": {
                    "spec": "ft-1.0.0",
                    "name": "Example Token Name",
                    "symbol": "EXMPL",
                    "decimals": 8
                }
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            (worker, prover_contract, connector_contract, fungible_token_contract)
        }

        async fn lock_ft(worker: &Worker<Sandbox>, _prover: &Contract, connector: &Contract, fungible_token: &Contract) -> CallExecutionDetails {
            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_2");
            let tla = workspaces::AccountId::try_from(ALICE_ACCOUNT_ID.to_string()).unwrap();
            let alice_account = worker.create_tla(tla, sec).await.unwrap().unwrap();
            println!("ALICE ACCOUNT ID: {}", alice_account.id());
            println!("SOURCE CONNECTOR ID: {}", connector.id());

            // register alice account with the FT contract
            alice_account.call(&worker, fungible_token.id(), "storage_deposit")
                .args_json(json!({
                    "accountId": alice_account.id()
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1 NEAR") as u128)
                .transact()
                .await
                .unwrap();

            // register source connector contract with the FT contract
            connector.as_account().call(&worker, fungible_token.id(), "storage_deposit")
                .args_json(json!({
                    "accountId": connector.id()
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1 NEAR") as u128)
                .transact()
                .await
                .unwrap();

            // transfer 88000 tokens to Alice
            fungible_token.call(
                    &worker,
                    "ft_transfer",
                ).args_json(json!({
                            "amount": "88000",
                            "receiver_id": alice_account.id()
                        }))
                    .unwrap()
                    .gas(parse_gas!("300 Tgas") as u64)
                    .deposit(parse_near!("1yoctoNEAR") as u128)
                    .transact()
                    .await
                    .unwrap();

            let balance_before_lock: String = worker.view(
                fungible_token.id(),
                "ft_balance_of",
                serde_json::to_vec(&serde_json::json!({
                        "account_id": ALICE_ACCOUNT_ID
                    })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(balance_before_lock == "88000");

            alice_account.call(&worker, fungible_token.id(), "ft_transfer_call")
                .args_json(json!({
                "receiver_id": connector.id(),
                    "amount": "12345",
                    "msg": "testnet"
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1yoctoNEAR") as u128)
                .transact()
                .await
                .unwrap()
        }

        // checks for deploy ft proof and maps the ft contracts on source connector
        async fn register_ft(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, _fungible_token: &Contract) -> CallExecutionDetails {
            let expected_block_merkle_root: Hash = decode_hex("5d4288c3a6ec76235b7475df26908882ac1d0a5e6573b405b8e11e2f23729fa4").try_into().unwrap();
            prover
                .call(&worker, "add_approved_hash")
                .args_json(json!({
                "hash": expected_block_merkle_root,
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            let deploy_proof = &file_as_json::<FullOutcomeProof>("deploy_proof.json").unwrap();

            prover
                .call(&worker, "prove_outcome")
                .args_json(json!({
                    "full_outcome_proof": deploy_proof,
                    "block_height": 9999, // not important for mock prover
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            connector
                .call(&worker, "register_ft_on_private")
                .args_json(json!({
                "proof": deploy_proof,
                "height": 9999, // not important for mock prover
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap()
        }

        // if provided with burn proof unlocks the fts
        async fn unlock_ft(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, _fungible_token: &Contract, burn_proof: &FullOutcomeProof) -> CallExecutionDetails {
            let block_merkle_root: &str = "07eaa6707866030c2000234d49da3e911a3ccb943515144f00a16c3f5b3740a9";
            let expected_block_merkle_root: Hash = decode_hex(block_merkle_root).try_into().unwrap();
            prover
                .call(&worker, "add_approved_hash")
                .args_json(json!({
                "hash": expected_block_merkle_root,
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            let unlock_execution_result = connector
                .call(&worker, "unlock")
                .args_json(json!({
                "proof": burn_proof,
                "height": 2474,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("25") as u128)
                .transact()
                .await
                .unwrap();

            assert!(unlock_execution_result.logs().len() == 2);
            assert!(unlock_execution_result.logs()[0] == format!("RecordProof:{}", block_merkle_root));

            // Alice got 345 unlocked from the source connector
            let event_json: serde_json::Value = serde_json::from_str(unlock_execution_result.logs()[1].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(event_json["event"] == "ft_transfer");
            assert!(event_json["standard"] == "nep141");
            assert!(event_json["data"][0]["amount"] == "345");
            assert!(event_json["data"][0]["new_owner_id"] == ALICE_ACCOUNT_ID);
            assert!(event_json["data"][0]["old_owner_id"] == connector.id().to_string());

            unlock_execution_result
        }

        async fn lock_and_unlock_should_all_pass(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, fungible_token: &Contract) -> FullOutcomeProof {
            let lock_execution_details = lock_ft(&worker, &prover, &connector, &fungible_token).await;
            assert!(lock_execution_details.is_success());

            let balance_after_lock: String = worker.view(
                fungible_token.id(),
                "ft_balance_of",
                serde_json::to_vec(&serde_json::json!({
                        "account_id": ALICE_ACCOUNT_ID
                    })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(balance_after_lock == "75655");

            let register_execution_details = register_ft(&worker, &prover, &connector, &fungible_token).await;
            assert!(register_execution_details.is_success());

            let burn_proof = &file_as_json::<FullOutcomeProof>("burn_proof.json").unwrap();
            let unlock_execution_details = unlock_ft(&worker, &prover, &connector, &fungible_token, &burn_proof).await;
            assert!(unlock_execution_details.is_success());

            let balance_after_unlock: String = worker.view(
                fungible_token.id(),
                "ft_balance_of",
                serde_json::to_vec(&serde_json::json!({
                        "account_id": ALICE_ACCOUNT_ID
                    })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(balance_after_unlock == "76000");

            burn_proof.clone()
        }

        #[tokio::test]
        async fn test_lock_works() {
            let (worker, prover, connector, fungible_token) = init().await;
            let lock_execution_details = lock_ft(&worker, &prover, &connector, &fungible_token).await;

            assert!(lock_execution_details.logs().len() == 2);

            // this event is emitted from the ft contract
            let event_json: serde_json::Value = serde_json::from_str(lock_execution_details.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(event_json["event"] == "ft_transfer");
            assert!(event_json["standard"] == "nep141");
            assert!(event_json["data"][0]["amount"] == "12345");
            assert!(event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            // verify lock event happened, this event is emitted from the ft_connector_source contract
            let parts: Vec<&str> = lock_execution_details.logs()[1].split(":").collect();
            assert!(parts.len() == 4);
            assert!(parts[0] == "CALIMERO_EVENT_LOCK");
            assert!(parts[1] == fungible_token.id().to_string());
            assert!(parts[2] == ALICE_ACCOUNT_ID);
            assert!(parts[3] == "12345");
        }

        #[tokio::test]
        async fn test_unlock() {
            let (worker, prover, connector, fungible_token) = init().await;
            lock_and_unlock_should_all_pass(&worker, &prover, &connector, &fungible_token).await;
        }

        #[tokio::test]
        #[should_panic]
        async fn test_proof_reuse_panics() {
            let (worker, prover, connector, fungible_token) = init().await;
            let used_proof = lock_and_unlock_should_all_pass(&worker, &prover, &connector, &fungible_token).await;

            // should panic since reusing proof
            unlock_ft(&worker, &prover, &connector, &fungible_token, &used_proof).await;
        }
    }
}
