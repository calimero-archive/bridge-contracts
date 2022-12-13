#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::serde_json;
        use near_sdk::serde_json::json;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use utils::hashes::{decode_hex};
        use utils::Hash;
        use types::{FullOutcomeProof, ConnectorType};
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker, Account};
        use workspaces::result::CallExecutionDetails;
        use ft_connector::PAUSE_LOCK;

        const FT_CONTRACT_ACCOUNT_ID: &str = "dev-1668507284663-45605813374523";
        const ALICE_ACCOUNT_ID: &str = "dev-1656412997567-26565713922487";

        async fn init() -> (Worker<Sandbox>, Contract, Contract, Contract, Contract) {
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
                "./target/wasm32-unknown-unknown/release/ft_connector.wasm",
            )
                .unwrap();
            let connector_contract = worker.dev_deploy(&connector_wasm).await.unwrap();
            let fungible_token_wasm = std::fs::read(
                "./tests/source_test_assets/fungible_token.wasm",
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

            connector_permissions_contract
                .call(&worker, "new")
                .args_json(json!({
                    "ft_connector_account": connector_contract.id().to_string(),
                    "nft_connector_account": "nft_connector_not_relevant_for_this_test",
                    "xsc_connector_account": "xsc_connector_not_relevant_for_this_test",
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            // allow anyone to bridge FTs
            connector_contract.as_account().call(&worker, connector_permissions_contract.id(), "add_allow_regex_rule").
                args_json(json!({
                    "regex_rule": ".*",
                    "connector_type": ConnectorType::FT,
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
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
                "locker_account": "ft_connector.m.calimero.testnet",
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
                    "name": "Mercep test token",
                    "symbol": "MERO",
                    "decimals": 8
                }
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            (worker, prover_contract, connector_contract, fungible_token_contract, connector_permissions_contract)
        }

        async fn create_and_fund_alice_account(worker: &Worker<Sandbox>, _prover: &Contract, connector: &Contract, fungible_token: &Contract) -> Account {
            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_2");
            let tla = workspaces::AccountId::try_from(ALICE_ACCOUNT_ID.to_string()).unwrap();
            let alice_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

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

            alice_account
        }

        async fn lock_ft(worker: &Worker<Sandbox>, _prover: &Contract, connector: &Contract, fungible_token: &Contract, account: &Account) -> CallExecutionDetails {
            account.call(&worker, fungible_token.id(), "ft_transfer_call")
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
            let expected_block_merkle_root: Hash = decode_hex("07b980b31eecbf13db1577b0b02e5186c1bf79ae810c9b7b9eb02dd5dadbeea0").try_into().unwrap();
            prover
                .call(&worker, "add_approved_hash")
                .args_json(json!({
                "hash": expected_block_merkle_root,
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            let deploy_proof = &file_as_json::<FullOutcomeProof>("source_test_assets/deploy_proof.json").unwrap();

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
                .call(&worker, "register_on_other")
                .args_json(json!({
                "proof": deploy_proof,
                "height": 9999, // not important for mock prover
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1"))
                .transact()
                .await
                .unwrap()
        }

        // if provided with burn proof unlocks the fts
        async fn unlock_ft(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, _fungible_token: &Contract, burn_proof: &FullOutcomeProof) -> CallExecutionDetails {
            let block_merkle_root: &str = "8be9635220956786962fbf23facd8e6a55e640bb75a94106956396145e9f12b7";
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
            assert!(event_json["data"][0]["amount"] == "23");
            assert!(event_json["data"][0]["new_owner_id"] == ALICE_ACCOUNT_ID);
            assert!(event_json["data"][0]["old_owner_id"] == connector.id().to_string());

            unlock_execution_result
        }

        async fn lock_and_unlock_should_all_pass(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, fungible_token: &Contract) -> FullOutcomeProof {
            let alice_account = create_and_fund_alice_account(&worker, &prover, &connector, &fungible_token).await;
            let lock_execution_details = lock_ft(&worker, &prover, &connector, &fungible_token, &alice_account).await;
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

            let burn_proof = &file_as_json::<FullOutcomeProof>("source_test_assets/burn_proof.json").unwrap();
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
            assert!(balance_after_unlock == "75678");

            burn_proof.clone()
        }

        #[tokio::test]
        async fn test_lock_works() {
            let (worker, prover, connector, fungible_token, _connector_permissions) = init().await;
            let alice_account = create_and_fund_alice_account(&worker, &prover, &connector, &fungible_token).await;
            let lock_execution_details = lock_ft(&worker, &prover, &connector, &fungible_token, &alice_account).await;

            assert!(lock_execution_details.logs().len() == 2);

            // this event is emitted from the ft contract
            let event_json: serde_json::Value = serde_json::from_str(lock_execution_details.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(event_json["event"] == "ft_transfer");
            assert!(event_json["standard"] == "nep141");
            assert!(event_json["data"][0]["amount"] == "12345");
            assert!(event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            // verify lock event happened, this event is emitted from the ft_connector contract
            let parts: Vec<&str> = lock_execution_details.logs()[1].split(":").collect();
            assert!(parts.len() == 4);
            assert!(parts[0] == "CALIMERO_EVENT_LOCK_FT");
            assert!(parts[1] == fungible_token.id().to_string());
            assert!(parts[2] == ALICE_ACCOUNT_ID);
            assert!(parts[3] == "12345");
        }

        #[tokio::test]
        async fn test_unlock() {
            let (worker, prover, connector, fungible_token, _connector_permissions) = init().await;
            lock_and_unlock_should_all_pass(&worker, &prover, &connector, &fungible_token).await;
        }

        #[tokio::test]
        #[should_panic(expected = "Event cannot be reused for depositing")]
        async fn test_proof_reuse_panics() {
            let (worker, prover, connector, fungible_token, _connector_permissions) = init().await;
            let used_proof = lock_and_unlock_should_all_pass(&worker, &prover, &connector, &fungible_token).await;

            // should panic since reusing proof
            unlock_ft(&worker, &prover, &connector, &fungible_token, &used_proof).await;
        }

        #[tokio::test]
        async fn test_on_lock_paused_should_refund() {
            let (worker, prover, connector, fungible_token, _connector_permissions) = init().await;
            let alice_account = create_and_fund_alice_account(&worker, &prover, &connector, &fungible_token).await;

            // pause locking on connector
            connector
                .call(&worker, "set_paused")
                .args_json(json!({
                "paused": PAUSE_LOCK,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            let lock_execution_details = lock_ft(&worker, &prover, &connector, &fungible_token, &alice_account).await;

            assert!(lock_execution_details.logs().len() == 2);

            let event_json: serde_json::Value = serde_json::from_str(lock_execution_details.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();

            assert!(event_json["event"] == "ft_transfer");
            assert!(event_json["standard"] == "nep141");
            assert!(event_json["data"][0]["amount"] == "12345");
            assert!(event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            let refund_event_json: serde_json::Value = serde_json::from_str(lock_execution_details.logs()[1].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(refund_event_json["event"] == "ft_transfer");
            assert!(refund_event_json["standard"] == "nep141");
            assert!(refund_event_json["data"][0]["amount"] == "12345");
            assert!(refund_event_json["data"][0]["new_owner_id"] == ALICE_ACCOUNT_ID);
            assert!(refund_event_json["data"][0]["old_owner_id"] == connector.id().to_string());
            assert!(refund_event_json["data"][0]["memo"] == "refund");
        }

        #[tokio::test]
        async fn test_lock_for_denied_account() {
            let (worker, prover, connector, fungible_token, connector_permissions) = init().await;

            let deny_result = connector.as_account()
                .call(&worker, connector_permissions.id(), "remove_allowed_regex_rule")
                .args_json(json!({
                    "regex_rule": ".*",
                    "connector_type": ConnectorType::FT,
                 }))
                .unwrap()
                .transact()
                .await
                .unwrap();
            assert!(deny_result.is_success());

            let alice_account = create_and_fund_alice_account(&worker, &prover, &connector, &fungible_token).await;

            let lock_result = lock_ft(&worker, &prover, &connector, &fungible_token, &alice_account).await;
            assert!(lock_result.logs().len() == 2);

            // first event shows that alice sent funds to ft_connector
            let event_json: serde_json::Value = serde_json::from_str(lock_result.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(event_json["event"] == "ft_transfer");
            assert!(event_json["standard"] == "nep141");
            assert!(event_json["data"][0]["amount"] == "12345");
            assert!(event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            // second event shows that alice got refund since alice's account is denied
            let refund_event_json: serde_json::Value = serde_json::from_str(lock_result.logs()[1].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(refund_event_json["event"] == "ft_transfer");
            assert!(refund_event_json["standard"] == "nep141");
            assert!(refund_event_json["data"][0]["amount"] == "12345");
            assert!(refund_event_json["data"][0]["new_owner_id"] == ALICE_ACCOUNT_ID);
            assert!(refund_event_json["data"][0]["old_owner_id"] == connector.id().to_string());
            assert!(refund_event_json["data"][0]["memo"] == "refund");

            // Allow Alice to use the ft_connector again
            let allow_result = connector.as_account()
                .call(&worker, connector_permissions.id(), "add_allow_regex_rule")
                .args_json(json!({
                    "regex_rule": ALICE_ACCOUNT_ID,
                    "connector_type": ConnectorType::FT,
                 }))
                .unwrap()
                .transact()
                .await
                .unwrap();
            assert!(allow_result.is_success());

            // Try locking ft-s again
            let second_lock_result = lock_ft(&worker, &prover, &connector, &fungible_token, &alice_account).await;

            assert!(second_lock_result.logs().len() == 2);
            // this event is emitted from the ft contract
            let transfer_event_json: serde_json::Value = serde_json::from_str(second_lock_result.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(transfer_event_json["event"] == "ft_transfer");
            assert!(transfer_event_json["standard"] == "nep141");
            assert!(transfer_event_json["data"][0]["amount"] == "12345");
            assert!(transfer_event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(transfer_event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            // verify lock event passed, this event is emitted from the ft_connector contract
            let parts: Vec<&str> = second_lock_result.logs()[1].split(":").collect();
            assert!(parts.len() == 4);
            assert!(parts[0] == "CALIMERO_EVENT_LOCK_FT");
            assert!(parts[1] == fungible_token.id().to_string());
            assert!(parts[2] == ALICE_ACCOUNT_ID);
            assert!(parts[3] == "12345");
        }
    }
}
