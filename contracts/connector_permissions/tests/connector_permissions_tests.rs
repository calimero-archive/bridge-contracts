#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector_permissions {
    mod test {
        use near_sdk::serde_json;
        use near_sdk::serde_json::json;
        use near_units::parse_gas;
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker};
        use types::ConnectorType;

        const FT_CONNECTOR_ACCOUNT_ID: &str =  "dev-1111111111111-11111111111111";
        const NFT_CONNECTOR_ACCOUNT_ID: &str = "dev-2222222222222-22222222222222";
        const XSC_CONNECTOR_ACCOUNT_ID: &str = "dev-3333333333333-33333333333333";
        const ALICE_ACCOUNT_ID: &str =         "dev-4444444444444-44444444444444";
        const BOB_ACCOUNT_ID: &str =           "dev-5555555555555-55555555555555";

        async fn init() -> (Worker<Sandbox>, Contract) {
            let worker = workspaces::sandbox().await.unwrap();

            let tla1 = workspaces::AccountId::try_from(FT_CONNECTOR_ACCOUNT_ID.to_string()).unwrap();
            let tla2 = workspaces::AccountId::try_from(NFT_CONNECTOR_ACCOUNT_ID.to_string()).unwrap();
            let tla3 = workspaces::AccountId::try_from(XSC_CONNECTOR_ACCOUNT_ID.to_string()).unwrap();

            let connector_permissions_wasm = std::fs::read(
                "./target/wasm32-unknown-unknown/release/connector_permissions.wasm",
            )
                .unwrap();
            let connector_permissions_contract = worker.dev_deploy(&connector_permissions_wasm).await.unwrap();
            connector_permissions_contract
                .call(&worker, "new")
                .args_json(json!({
                    "ft_connector_account": tla1,
                    "nft_connector_account": tla2,
                    "xsc_connector_account": tla3,
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            (worker, connector_permissions_contract)
        }

        async fn test_add_remove_and_read_for_connector_type(worker: &Worker<Sandbox>, connector_permissions_contract: &Contract, connector_type: ConnectorType) {

            let connector_str = match connector_type {
                ConnectorType::FT => FT_CONNECTOR_ACCOUNT_ID,
                ConnectorType::NFT => NFT_CONNECTOR_ACCOUNT_ID,
                ConnectorType::XSC => XSC_CONNECTOR_ACCOUNT_ID
            };

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, format!("secret_key_ft_connector_{}", connector_str).as_str());
            let tla = workspaces::AccountId::try_from(connector_str.to_string()).unwrap();
            let connector_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

            let add_to_deny_list_result = connector_account.call(&worker, connector_permissions_contract.id(), "deny_bridge")
                .args_json(json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "connector_type": connector_type,
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            assert!(add_to_deny_list_result.is_success());

            // Alice is denied, should return false
            let can_alice_bridge_result_1: bool = worker.view(
                connector_permissions_contract.id(),
                "can_bridge",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "connector_type": connector_type,
                })).unwrap())
                .await
                .unwrap()
                .json()
                .unwrap();

            assert_eq!(can_alice_bridge_result_1, false);

            // Bob is not denied, should return true
            let can_bob_bridge_result: bool = worker.view(
                connector_permissions_contract.id(),
                "can_bridge",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": BOB_ACCOUNT_ID,
                    "connector_type": connector_type,
                })).unwrap())
                .await
                .unwrap()
                .json()
                .unwrap();

            assert_eq!(can_bob_bridge_result, true);

            let remove_from_deny_list_result = connector_account.call(&worker, connector_permissions_contract.id(), "allow_bridge")
                .args_json(json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "connector_type": connector_type,
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            assert!(remove_from_deny_list_result.is_success());

            // Alice is no longer denied, should return true
            let can_alice_bridge_result_2: bool = worker.view(
                connector_permissions_contract.id(),
                "can_bridge",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "connector_type": connector_type,
                })).unwrap())
                .await
                .unwrap()
                .json()
                .unwrap();

            assert_eq!(can_alice_bridge_result_2, true);
        }

        #[tokio::test]
        async fn test_deny_list_for_all_connector_types() {
            let (worker, connector_permissions_contract) = init().await;
            test_add_remove_and_read_for_connector_type(&worker, &connector_permissions_contract, ConnectorType::FT).await;
            test_add_remove_and_read_for_connector_type(&worker, &connector_permissions_contract, ConnectorType::NFT).await;
            test_add_remove_and_read_for_connector_type(&worker, &connector_permissions_contract, ConnectorType::XSC).await;
        }

        #[tokio::test]
        #[should_panic]
        async fn test_add_to_deny_list_by_non_connector_account() {
            let (worker, connector_permissions_contract) = init().await;

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_alice");
            let tla = workspaces::AccountId::try_from(ALICE_ACCOUNT_ID.to_string()).unwrap();
            let alice_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

            let result = alice_account.call(&worker, connector_permissions_contract.id(), "deny_bridge")
                .args_json(json!({
                    "account_id": alice_account.id(),
                    "connector_type": ConnectorType::FT,
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            assert!(!result.is_success());
        }

        #[tokio::test]
        async fn test_deny_account_for_contract() {
            let (worker, connector_permissions_contract) = init().await;
            //test_add_remove_and_read_for_connector_type(&worker, &connector_permissions_contract, ConnectorType::XSC);

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_for_xsc");
            let tla = workspaces::AccountId::try_from(XSC_CONNECTOR_ACCOUNT_ID.to_string()).unwrap();
            let xsc_connector_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

            let add_to_deny_contract_list_result = xsc_connector_account.call(&worker, connector_permissions_contract.id(), "deny_cross_shard_call_per_contract")
                .args_json(json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "contract_id": "forbidden.for.alice.testnet",
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();
            assert!(add_to_deny_contract_list_result.is_success());

            let can_alice_call_contract_result: bool = worker.view(
                connector_permissions_contract.id(),
                "can_make_cross_shard_call_for_contract",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "contract_id": "forbidden.for.alice.testnet",
                })).unwrap())
                .await
                .unwrap()
                .json()
                .unwrap();
            assert_eq!(can_alice_call_contract_result, false);

            let can_bob_call_contract_result: bool = worker.view(
                connector_permissions_contract.id(),
                "can_make_cross_shard_call_for_contract",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": BOB_ACCOUNT_ID,
                    "contract_id": "forbidden.for.alice.testnet",
                })).unwrap())
                .await
                .unwrap()
                .json()
                .unwrap();
            assert_eq!(can_bob_call_contract_result, true);

            let remove_from_deny_contract_list_result = xsc_connector_account.call(&worker, connector_permissions_contract.id(), "allow_cross_shard_call_per_contract")
                .args_json(json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "contract_id": "forbidden.for.alice.testnet",
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();
            assert!(remove_from_deny_contract_list_result.is_success());

            let can_alice_call_contract_result: bool = worker.view(
                connector_permissions_contract.id(),
                "can_make_cross_shard_call_for_contract",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "contract_id": "forbidden.for.alice.testnet",
                })).unwrap())
                .await
                .unwrap()
                .json()
                .unwrap();
            assert_eq!(can_alice_call_contract_result, true);

            let add_to_deny_list_result = xsc_connector_account.call(&worker, connector_permissions_contract.id(), "deny_bridge")
                .args_json(json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "connector_type": ConnectorType::XSC,
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();
            assert!(add_to_deny_list_result.is_success());

            // The call for specific contract should still return true if that account is denied for making any cross shard calls
            let can_alice_call_contract_result: bool = worker.view(
                connector_permissions_contract.id(),
                "can_make_cross_shard_call_for_contract",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID,
                    "contract_id": "forbidden.for.alice.testnet",
                })).unwrap())
                .await
                .unwrap()
                .json()
                .unwrap();
            assert_eq!(can_alice_call_contract_result, false);
        }
    }
}
