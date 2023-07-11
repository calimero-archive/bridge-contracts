#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::serde_json::json;
        use near_units::{parse_gas, parse_near};
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker, Account, AccountId};

        const DEPLOYER_ACCOUNT_ID: &str = "dev-1668507284663-45605813374523";
        const BRIDGE_ACCOUNT_ID: &str = "dev-1656412997567-26565713922487";

        async fn init() -> (Worker<Sandbox>, Contract, Account) {
            let worker = workspaces::sandbox().await.unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_1");
            let tla = workspaces::AccountId::try_from(DEPLOYER_ACCOUNT_ID.to_string()).unwrap();
            let deployer_wasm = std::fs::read(
                "../target/wasm32-unknown-unknown/release/bridge_token_deployer.wasm",
            )
                .unwrap();

            let deployer_contract = worker.create_tla_and_deploy(tla, sec, &deployer_wasm)
                .await
                .unwrap()
                .unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_2");
            let tla = workspaces::AccountId::try_from(BRIDGE_ACCOUNT_ID.to_string()).unwrap();
            let bridge_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

            // initialize contracts

            deployer_contract
                .call(&worker, "new")
                .args_json(json!({
                "bridge_account": bridge_account.id().to_string(),
                "source_master_account": "testnet",
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            (worker, deployer_contract, bridge_account)
        }

        async fn test_deploy_bridge_token(source_address: &str, expected_token: &str) {
            let (worker, deployer_contract, bridge_account) = init().await;

            bridge_account
                .call(&worker, deployer_contract.id(), "deploy_bridge_token")
                .args_json(json!({
                "source_address": source_address,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("1"))
                .transact()
                .await
                .unwrap();

            let expected_account_id: AccountId = expected_token.parse().unwrap();

            assert!(worker.view_account(&expected_account_id).await.is_ok());
        }

        #[tokio::test]
        async fn test_short_name() {
            test_deploy_bridge_token("dev-0.testnet", &format!("dev-0.{}", DEPLOYER_ACCOUNT_ID)).await;
        }

        #[tokio::test]
        async fn test_long_name() {
            test_deploy_bridge_token("dev-1111111111111-11111111111111.testnet", &format!("753ee4393ef6cfddcdcf6fff53188f6.{}", DEPLOYER_ACCOUNT_ID)).await;
        }

        #[tokio::test]
        async fn test_long_name_no_suffix() {
            test_deploy_bridge_token("dev-1111111111111-11111111111111", &format!("753ee4393ef6cfddcdcf6fff53188f6.{}", DEPLOYER_ACCOUNT_ID)).await;
        }

        #[tokio::test]
        async fn test_short_name_different_suffix() {
            test_deploy_bridge_token("dev-0.calimero", &format!("dev-0_calimero.{}", DEPLOYER_ACCOUNT_ID)).await;
        }

        #[tokio::test]
        async fn test_long_name_different_suffix() {
            test_deploy_bridge_token("dev-1111111111111-11111111111111.calimero", &format!("2d39e1f9245ca96d18551c93efb233f.{}", DEPLOYER_ACCOUNT_ID)).await;
        }
    }
}
