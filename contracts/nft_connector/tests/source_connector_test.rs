#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::serde_json;
        use near_sdk::serde_json::json;
        use near_sdk::json_types::U128;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use utils::hashes::{decode_hex, encode_hex};
        use utils::Hash;
        use types::{FullOutcomeProof, ConnectorType};
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker, Account};
        use workspaces::result::CallExecutionDetails;

        const NFT_CONTRACT_ACCOUNT_ID: &str = "dev-1666030964074-54624403721325";
        const ALICE_ACCOUNT_ID: &str = "dev-1658913032484-17227415983110";

        async fn init() -> (Worker<Sandbox>, Contract, Contract, Contract, Contract) {
            let worker = workspaces::sandbox().await.unwrap();
            // deploy contracts
            let prover_wasm = std::fs::read(
                "../target/wasm32-unknown-unknown/release/mock_prover.wasm",
            )
                .unwrap();
            let prover_contract = worker.dev_deploy(&prover_wasm).await.unwrap();
            let connector_permissions_wasm = std::fs::read(
                "../wasm/connector_permissions.wasm",
            )
                .unwrap();
            let connector_permissions_contract = worker.dev_deploy(&connector_permissions_wasm).await.unwrap();
            let connector_wasm = std::fs::read(
                "../target/wasm32-unknown-unknown/release/nft_connector.wasm",
            )
                .unwrap();
            let connector_contract = worker.dev_deploy(&connector_wasm).await.unwrap();
            let non_fungible_token_wasm = std::fs::read(
                "./tests/source_test_assets/non_fungible_token.wasm",
            )
                .unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_1");
            let tla = workspaces::AccountId::try_from(NFT_CONTRACT_ACCOUNT_ID.to_string()).unwrap();

            let non_fungible_token_contract = worker.create_tla_and_deploy(tla, sec, &non_fungible_token_wasm)
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
                    "ft_connector_account": "ft_connector_not_relevant_for_this_test",
                    "nft_connector_account": connector_contract.id().to_string(),
                    "xsc_connector_account": "xsc_connector_not_relevant_for_this_test",
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            // allow anyone to bridge NFTs
            connector_contract.as_account().call(&worker, connector_permissions_contract.id(), "add_allow_regex_rule").
                args_json(json!({
                    "regex_rule": ".*",
                    "connector_type": ConnectorType::NFT,
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1"))
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
                "locker_account": "nftdc.90.calimero.testnet",
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            non_fungible_token_contract
                .call(&worker, "new_default_meta")
                .args_json(json!({
                "owner_id": non_fungible_token_contract.id().to_string(),
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            (worker, prover_contract, connector_contract, non_fungible_token_contract, connector_permissions_contract)
        }

        async fn create_and_fund_alice_account(worker: &Worker<Sandbox>, _prover: &Contract, connector: &Contract, non_fungible_token: &Contract) -> Account {
            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_2");
            let tla = workspaces::AccountId::try_from(ALICE_ACCOUNT_ID.to_string()).unwrap();
            let alice_account = worker.create_tla(tla, sec).await.unwrap().unwrap();
            println!("ALICE ACCOUNT ID: {}", alice_account.id());
            println!("SOURCE CONNECTOR ID: {}", connector.id());

            // mint a token for Alice
            non_fungible_token.call(
                &worker,
                "nft_mint",
            ).args_json(json!({
                "token_id": "0",
                "receiver_id": ALICE_ACCOUNT_ID,
                "token_metadata": {
                    "title": "Luka Modric",
                    "description": "Best footbal player in the world",
                    "media": "https://static01.nyt.com/images/2018/12/04/sports/04SOCCER-web/merlin_144148398_f3816ef7-6049-416c-910e-81c3d6657de7-superJumbo.jpg",
                    "copies": 1
                }
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("0.1 NEAR") as u128)
                .transact()
                .await
                .unwrap();


            let owner_before_lock: serde_json::Value = worker.view(
                non_fungible_token.id(),
                "nft_tokens_for_owner",
                serde_json::to_vec(&serde_json::json!({
                        "account_id": ALICE_ACCOUNT_ID
                    })).unwrap())
                .await.unwrap()
                .json().unwrap();

            assert!(owner_before_lock[0]["owner_id"] == ALICE_ACCOUNT_ID);
            assert!(owner_before_lock[0]["token_id"] == "0");
            assert!(owner_before_lock[0]["metadata"]["title"] == "Luka Modric");

            alice_account
        }

        async fn lock_nft(worker: &Worker<Sandbox>, _prover: &Contract, connector: &Contract, non_fungible_token: &Contract, account: &Account) -> CallExecutionDetails {
            account.call(&worker, non_fungible_token.id(), "nft_transfer_call")
                .args_json(json!({
                    "receiver_id": connector.id(),
                    "token_id": "0",
                    "msg": ""
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1yoctoNEAR") as u128)
                .transact()
                .await
                .unwrap()
        }

        // checks for deploy nft proof and maps the nft contracts on source connector
        async fn register_nft(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, _non_fungible_token: &Contract) -> CallExecutionDetails {
            let expected_block_merkle_root: Hash = decode_hex("9d5f239abca26174d0a3c0c4975bf3bab65b1c7bf94539fa550c39f88e2dfdff").try_into().unwrap();
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

        // if provided with burn proof unlocks the nft
        async fn unlock_nft(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, _non_fungible_token: &Contract, burn_proof: &FullOutcomeProof) -> CallExecutionDetails {
            let block_merkle_root: &str = "8566a7e9df006a314a70a9c3cf874ea02324d28a71a17423bb5b6755e35520a0";
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
            assert!(unlock_execution_result.logs()[0] == format!("RecordProof:{}", encode_hex(burn_proof.outcome_proof.outcome_with_id.id.as_slice())));

            // Alice got the nft unlocked from the source connector
            let event_json: serde_json::Value = serde_json::from_str(unlock_execution_result.logs()[1].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(event_json["event"] == "nft_transfer");
            assert!(event_json["standard"] == "nep171");
            assert!(event_json["data"][0]["token_ids"][0] == "0");
            assert!(event_json["data"][0]["new_owner_id"] == ALICE_ACCOUNT_ID);
            assert!(event_json["data"][0]["old_owner_id"] == connector.id().to_string());

            unlock_execution_result
        }

        async fn lock_and_unlock_should_all_pass(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, non_fungible_token: &Contract) -> FullOutcomeProof {
            let alice_account = create_and_fund_alice_account(&worker, &prover, &connector, &non_fungible_token).await;
            let lock_execution_details = lock_nft(&worker, &prover, &connector, &non_fungible_token, &alice_account).await;
            assert!(lock_execution_details.is_success());

            // On source side nft is never burnt, the supply should stay the same
            let before_unlock_total_supply: U128 = worker.view(
                non_fungible_token.id(),
                "nft_total_supply",
                serde_json::to_vec(&serde_json::json!({
                        })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(before_unlock_total_supply == U128(1));

            // after lock Alice is no longer the owner of the NFT (token_id = 0)
            let owner_after_lock: serde_json::Value = worker.view(
                non_fungible_token.id(),
                "nft_tokens_for_owner",
                serde_json::to_vec(&serde_json::json!({
                        "account_id": ALICE_ACCOUNT_ID
                    })).unwrap())
                .await.unwrap()
                .json().unwrap();
            println!("{}", owner_after_lock.to_string());
            assert!(owner_after_lock.to_string() == "[]");

            let register_execution_details = register_nft(&worker, &prover, &connector, &non_fungible_token).await;
            assert!(register_execution_details.is_success());

            let burn_proof = &file_as_json::<FullOutcomeProof>("source_test_assets/burn_proof.json").unwrap();
            let unlock_execution_details = unlock_nft(&worker, &prover, &connector, &non_fungible_token, &burn_proof).await;
            assert!(unlock_execution_details.is_success());

            let owner_after_unlock: serde_json::Value = worker.view(
                non_fungible_token.id(),
                "nft_tokens_for_owner",
                serde_json::to_vec(&serde_json::json!({
                        "account_id": ALICE_ACCOUNT_ID
                    })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(owner_after_unlock[0]["owner_id"] == ALICE_ACCOUNT_ID);
            assert!(owner_after_unlock[0]["token_id"] == "0");

            // On source side nft is never burnt, the supply should stay the same
            let after_unlock_total_supply: U128 = worker.view(
                    non_fungible_token.id(),
                    "nft_total_supply",
                    serde_json::to_vec(&serde_json::json!({
                        })).unwrap())
                    .await.unwrap()
                    .json().unwrap();
            assert!(after_unlock_total_supply == U128(1));

            burn_proof.clone()
        }

        #[tokio::test]
        async fn test_lock_works() {
            let (worker, prover, connector, non_fungible_token, _connector_permissions) = init().await;
            let alice_account = create_and_fund_alice_account(&worker, &prover, &connector, &non_fungible_token).await;
            let lock_execution_details = lock_nft(&worker, &prover, &connector, &non_fungible_token, &alice_account).await;

            assert!(lock_execution_details.logs().len() == 2);

            // this event is emitted from the nft contract
            let event_json: serde_json::Value = serde_json::from_str(lock_execution_details.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(event_json["event"] == "nft_transfer");
            assert!(event_json["standard"] == "nep171");
            assert!(event_json["data"][0]["token_ids"][0] == "0");
            assert!(event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            // verify lock event happened, this event is emitted from the nft_connector_source contract
            let parts: Vec<&str> = lock_execution_details.logs()[1].split(":").collect();
            assert!(parts.len() == 5);
            assert!(parts[0] == "CALIMERO_EVENT_LOCK_NFT");
            assert!(parts[1] == non_fungible_token.id().to_string());
            assert!(parts[2] == ALICE_ACCOUNT_ID);
            assert!(parts[3] == base64::encode("0"));
            assert!(parts[4] == "eyJ0aXRsZSI6Ikx1a2EgTW9kcmljIiwiZGVzY3JpcHRpb24iOiJCZXN0IGZvb3RiYWwgcGxheWVyIGluIHRoZSB3b3JsZCIsIm1lZGlhIjoiaHR0cHM6Ly9zdGF0aWMwMS5ueXQuY29tL2ltYWdlcy8yMDE4LzEyLzA0L3Nwb3J0cy8wNFNPQ0NFUi13ZWIvbWVybGluXzE0NDE0ODM5OF9mMzgxNmVmNy02MDQ5LTQxNmMtOTEwZS04MWMzZDY2NTdkZTctc3VwZXJKdW1iby5qcGciLCJtZWRpYV9oYXNoIjpudWxsLCJjb3BpZXMiOjEsImlzc3VlZF9hdCI6bnVsbCwiZXhwaXJlc19hdCI6bnVsbCwic3RhcnRzX2F0IjpudWxsLCJ1cGRhdGVkX2F0IjpudWxsLCJleHRyYSI6bnVsbCwicmVmZXJlbmNlIjpudWxsLCJyZWZlcmVuY2VfaGFzaCI6bnVsbH0=");
        }

        #[tokio::test]
        async fn test_unlock() {
            let (worker, prover, connector, non_fungible_token, _connector_permissions) = init().await;
            lock_and_unlock_should_all_pass(&worker, &prover, &connector, &non_fungible_token).await;
        }

        #[tokio::test]
        #[should_panic(expected = "Event cannot be reused for depositing")]
        async fn test_proof_reuse_panics() {
            let (worker, prover, connector, non_fungible_token, _connector_permissions) = init().await;
            let used_proof = lock_and_unlock_should_all_pass(&worker, &prover, &connector, &non_fungible_token).await;

            // should panic since reusing proof
            unlock_nft(&worker, &prover, &connector, &non_fungible_token, &used_proof).await;
        }

        #[tokio::test]
        async fn test_lock_for_denied_account() {
            let (worker, prover, connector, non_fungible_token, connector_permissions) = init().await;

            let deny_result = connector.as_account()
                .call(&worker, connector_permissions.id(), "remove_allowed_regex_rule")
                .args_json(json!({
                    "regex_rule": ".*",
                    "connector_type": ConnectorType::NFT,
                 }))
                .unwrap()
                .transact()
                .await
                .unwrap();
            assert!(deny_result.is_success());

            let alice_account = create_and_fund_alice_account(&worker, &prover, &connector, &non_fungible_token).await;

            let lock_result = lock_nft(&worker, &prover, &connector, &non_fungible_token, &alice_account).await;
            assert!(lock_result.logs().len() == 2);

            // first event shows that alice sent funds to nft_connector
            let event_json: serde_json::Value = serde_json::from_str(lock_result.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(event_json["event"] == "nft_transfer");
            assert!(event_json["standard"] == "nep171");
            assert!(event_json["data"][0]["token_ids"][0] == "0");
            assert!(event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            // second event shows that alice got refund since alice's account is denied
            let refund_event_json: serde_json::Value = serde_json::from_str(lock_result.logs()[1].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(refund_event_json["event"] == "nft_transfer");
            assert!(refund_event_json["standard"] == "nep171");
            assert!(refund_event_json["data"][0]["token_ids"][0] == "0");
            assert!(refund_event_json["data"][0]["new_owner_id"] == ALICE_ACCOUNT_ID);
            assert!(refund_event_json["data"][0]["old_owner_id"] == connector.id().to_string());

            // Allow Alice to use the nft_connector again
            let allow_result = connector.as_account()
                .call(&worker, connector_permissions.id(), "add_allow_regex_rule")
                .args_json(json!({
                    "regex_rule": ALICE_ACCOUNT_ID,
                    "connector_type": ConnectorType::NFT,
                 }))
                .unwrap()
                .deposit(parse_near!("1"))
                .transact()
                .await
                .unwrap();
            assert!(allow_result.is_success());

            // Try locking nft again
            let second_lock_result = lock_nft(&worker, &prover, &connector, &non_fungible_token, &alice_account).await;

            assert!(second_lock_result.logs().len() == 2);
            // verify lock event passed, owner of the NFT is the nft connector now
            let transfer_event_json: serde_json::Value = serde_json::from_str(second_lock_result.logs()[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
            assert!(transfer_event_json["event"] == "nft_transfer");
            assert!(transfer_event_json["standard"] == "nep171");
            assert!(transfer_event_json["data"][0]["token_ids"][0] == "0");
            assert!(transfer_event_json["data"][0]["new_owner_id"] == connector.id().to_string());
            assert!(transfer_event_json["data"][0]["old_owner_id"] == ALICE_ACCOUNT_ID);

            // verify CALIMERO_EVENT_LOCK_NFT event was emitted
            let parts: Vec<&str> = second_lock_result.logs()[1].split(":").collect();
            assert!(parts[0] == "CALIMERO_EVENT_LOCK_NFT");
            assert!(parts[1] == non_fungible_token.id().to_string());
            assert!(parts[2] == ALICE_ACCOUNT_ID);
            assert!(parts[3] == base64::encode("0"));
            assert!(parts[4] == "eyJ0aXRsZSI6Ikx1a2EgTW9kcmljIiwiZGVzY3JpcHRpb24iOiJCZXN0IGZvb3RiYWwgcGxheWVyIGluIHRoZSB3b3JsZCIsIm1lZGlhIjoiaHR0cHM6Ly9zdGF0aWMwMS5ueXQuY29tL2ltYWdlcy8yMDE4LzEyLzA0L3Nwb3J0cy8wNFNPQ0NFUi13ZWIvbWVybGluXzE0NDE0ODM5OF9mMzgxNmVmNy02MDQ5LTQxNmMtOTEwZS04MWMzZDY2NTdkZTctc3VwZXJKdW1iby5qcGciLCJtZWRpYV9oYXNoIjpudWxsLCJjb3BpZXMiOjEsImlzc3VlZF9hdCI6bnVsbCwiZXhwaXJlc19hdCI6bnVsbCwic3RhcnRzX2F0IjpudWxsLCJ1cGRhdGVkX2F0IjpudWxsLCJleHRyYSI6bnVsbCwicmVmZXJlbmNlIjpudWxsLCJyZWZlcmVuY2VfaGFzaCI6bnVsbH0=");

        }
    }
}