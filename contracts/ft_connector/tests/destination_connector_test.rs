#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::serde_json;
        use near_sdk::serde_json::json;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use utils::hashes::decode_hex;
        use types::FullOutcomeProof;
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker};
        use workspaces::result::CallExecutionDetails;
        use ft_connector::{PAUSE_DEPLOY_TOKEN, PAUSE_MINT};

        const FT_CONTRACT_ACCOUNT_ID: &str = "dev-1668507284663-45605813374523";
        const ALICE_ACCOUNT_ID: &str = "dev-1656412997567-26565713922487";

        async fn init() -> (Worker<Sandbox>, Contract, Contract, Contract) {
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
            let deployer_wasm = std::fs::read(
                "../wasm/ft_bridge_token_deployer.wasm",
            )
                .unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_deployer");
            let tla = workspaces::AccountId::try_from("deployer.test.near".to_string()).unwrap();
            let deployer_contract = worker.create_tla_and_deploy(tla, sec, &deployer_wasm).await.unwrap().unwrap();

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

            deployer_contract
                .call(&worker, "new")
                .args_json(json!({
                    "bridge_account": connector_contract.id().to_string(),
                    "source_master_account": "testnet",
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            connector_contract
                .call(&worker, "set_deployer")
                .args_json(json!({
                    "deployer_account": deployer_contract.id().to_string(),
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();


            (worker, prover_contract, connector_contract, deployer_contract)
        }

        async fn reuse_proof(worker: Worker<Sandbox>, _prover: Contract, connector: Contract, _deployer: Contract, proof: FullOutcomeProof, block_height: u64) {
            let _reused_proof_execution_details = connector
                .call(&worker, "mint")
                .args_json(json!({
                    "proof": proof,
                    "height": block_height,
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("25") as u128)
                .transact()
                .await
                .unwrap();
        }

        async fn mint(worker: &Worker<Sandbox>, prover: &Contract, connector: &Contract, _deployer: &Contract) -> (CallExecutionDetails, CallExecutionDetails, FullOutcomeProof) {
            prover
                .call(&worker, "add_approved_hash")
                .args_json(json!({
                    "hash": decode_hex("87003fd9547f2689ed698e30abc91b8bae5952699b678ecf9a035cf75095e160"),
                }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            let proof = &file_as_json::<FullOutcomeProof>("destination_test_assets/lock_proof.json").unwrap();

            connector
                .call(&worker, "set_locker")
                .args_json(json!({
                    "locker_account": "ft_connector.m.dev.calimero.testnet".to_string(),
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();


            let deploy_token_execution_details = connector
                .call(&worker, "deploy_bridge_token")
                .args_json(json!({
                    "source_address": FT_CONTRACT_ACCOUNT_ID,
                }))
                .unwrap()
                .deposit(parse_near!("50N"))
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();
            assert!(deploy_token_execution_details.is_success());

            let random_account = worker.dev_create_account().await.unwrap();

            // anyone can call mint that should pass provided with correct proof
            let mint_execution_details = random_account.call(&worker, connector.id(), "mint")
                .args_json(json!({
                    "proof": proof,
                    "height": 9999999, // not important in this test
                }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("25") as u128)
                .transact()
                .await
                .unwrap();

            assert!(mint_execution_details.is_success(), "Not correct proof");

            (mint_execution_details, deploy_token_execution_details, proof.clone())
        }

        #[tokio::test]
        async fn test_mint_works() {
            let (worker, prover, connector, deployer) = init().await;
            mint(&worker, &prover, &connector, &deployer).await;
        }

        #[tokio::test]
        #[should_panic(expected = "paused")]
        async fn test_mint_paused() {
            let (worker, prover, connector, deployer) = init().await;

            // pause minting on connector
            connector
                .call(&worker, "set_paused")
                .args_json(json!({
                "paused": PAUSE_MINT,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();
            let (_mint_execution_details, _deploy_token_execution_details, _proof) = mint(&worker, &prover, &connector, &deployer).await;
        }

        #[tokio::test]
        #[should_panic(expected = "paused")]
        async fn test_deploy_bridge_token_paused() {
            let (worker, prover, connector, deployer) = init().await;

            // pause minting on connector
            connector
                .call(&worker, "set_paused")
                .args_json(json!({
                "paused": PAUSE_DEPLOY_TOKEN,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();
            let (_mint_execution_details, _deploy_token_execution_details, _proof) = mint(&worker, &prover, &connector, &deployer).await;
        }

        #[tokio::test]
        #[should_panic(expected = "Event cannot be reused for depositing")]
        async fn test_proof_reuse_panics() {
            let (worker, prover, connector, deployer) = init().await;
            let (_mint_execution_details, _deploy_token_execution_details, proof) = mint(&worker, &prover, &connector, &deployer).await;
            reuse_proof(worker, prover, connector, deployer, proof, 99152413).await
        }


        #[tokio::test]
        async fn test_withdraw() {
            let (worker, prover, connector, deployer) = init().await;
            mint(&worker, &prover, &connector, &deployer).await;

            let bridged_ft_contract_id_str: String = worker
                .view(connector.id(),
                      "view_mapping",
                      serde_json::json!({
                        "source_account": FT_CONTRACT_ACCOUNT_ID
                      }).to_string()
                          .into_bytes(),
                )
                .await.unwrap()
                .json().unwrap();


            let bridged_ft_contract_id = &workspaces::AccountId::try_from(bridged_ft_contract_id_str.to_string()).unwrap();
            let balance_after_mint: String = worker.view(
                bridged_ft_contract_id,
                "ft_balance_of",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID
                })).unwrap())
                .await.unwrap()
                .json().unwrap();

            assert!(balance_after_mint == "123");

            // create the account where the newly minted tokens are, so we can withdraw some amount
            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_2");
            let tla = workspaces::AccountId::try_from(ALICE_ACCOUNT_ID.to_string()).unwrap();
            let alice_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

            // call withdraw on the bridged FT
            let withdraw_result =
                alice_account.call(
                    &worker,
                    bridged_ft_contract_id,
                    "withdraw",
                )
                    .args_json(json!({
                        "amount": "23"
                    }))
                    .unwrap()
                    .gas(parse_gas!("300 Tgas") as u64)
                    .deposit(parse_near!("1yoctoNEAR") as u128)
                    .transact()
                    .await
                    .unwrap();

            let logs_from_withdraw = withdraw_result.logs();
            assert!(logs_from_withdraw.len() == 1);

            // verify burn event happened, this event is emitted from the ft_connector contract
            let parts: Vec<&str> = logs_from_withdraw[0].split(":").collect();
            assert!(parts.len() == 4);
            assert!(parts[0] == "CALIMERO_EVENT_BURN_FT");
            assert!(parts[1] == bridged_ft_contract_id_str);
            assert!(parts[2] == alice_account.id().to_string());
            assert!(parts[3] == "23");

            let balance_after_burn: String = worker.view(
                bridged_ft_contract_id,
                "ft_balance_of",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID
                })).unwrap())
                .await.unwrap()
                .json().unwrap();

            assert!(balance_after_burn == "100");
        }
    }
}
