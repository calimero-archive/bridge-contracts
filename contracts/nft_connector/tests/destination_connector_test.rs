#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::serde_json;
        use near_sdk::json_types::U128;
        use near_sdk::serde_json::json;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use utils::hashes::decode_hex;
        use utils::Hash;
        use types::FullOutcomeProof;
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker};

        const NFT_CONTRACT_ACCOUNT_ID: &str = "dev-1666030964074-54624403721325";
        const ALICE_ACCOUNT_ID: &str = "dev-1658913032484-17227415983110";

        async fn init() -> (Worker<Sandbox>, Contract, Contract) {
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
                "./target/wasm32-unknown-unknown/release/nft_connector.wasm",
            )
                .unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_connector");
            let tla = workspaces::AccountId::try_from("connector.test.near".to_string()).unwrap();
            let connector_contract = worker.create_tla_and_deploy(tla, sec, &connector_wasm).await.unwrap().unwrap();

            let deployer_wasm = std::fs::read(
                "../wasm/nft_bridge_token_deployer.wasm",
            )
                .unwrap();

            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "secret_key_deployer");
            let tla = workspaces::AccountId::try_from("deployer.test.near".to_string()).unwrap();
            let deployer_contract = worker.create_tla_and_deploy(tla, sec, &deployer_wasm).await.unwrap().unwrap();

            for _ in 0..10 {
                let temp_account_to_feed_connector_with_more_funds = worker.dev_create_account()
                    .await
                    .unwrap();
            
                temp_account_to_feed_connector_with_more_funds
                    .transfer_near(&worker, deployer_contract.id(), parse_near!("49 N"))
                    .await
                    .unwrap();
                
                temp_account_to_feed_connector_with_more_funds
                    .transfer_near(&worker, connector_contract.id(), parse_near!("49 N"))
                    .await
                    .unwrap();
            }

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
                    "nft_connector_account": "nft_connector_not_relevant_for_this_test",
                    "xsc_connector_account": connector_permissions_contract.id().to_string(),
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

            (worker, prover_contract, connector_contract)
        }

        async fn transfer_nft(file_prefix: &str, block_height: u64, hash: Hash, locker_account: String) -> (Worker<Sandbox>, Contract, Contract, FullOutcomeProof) {
            let (worker, prover, connector) = init().await;
            prover
                .call(&worker, "add_approved_hash")
                .args_json(json!({
                "hash": hash,
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            let proof = &file_as_json::<FullOutcomeProof>(&format!("destination_test_assets/{}{}", file_prefix, "proof.json")).unwrap();

            connector
                .call(&worker, "set_locker")
                .args_json(json!({
                "locker_account": locker_account,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            connector
                .call(&worker, "deploy_bridge_token")
                .args_json(json!({
                "source_address": NFT_CONTRACT_ACCOUNT_ID,
            }))
                .unwrap()
                .deposit(parse_near!("50N"))
                .gas(parse_gas!("300 Tgas") as u64)
                .transact()
                .await
                .unwrap();

            let execution_details = connector
                .call(&worker, "mint")
                .args_json(json!({
                "proof": proof,
                "height": block_height,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("30") as u128)
                .transact()
                .await
                .unwrap();

            assert!(execution_details.is_success(), "Not correct proof");

            (worker, prover, connector, proof.clone())
        }

        async fn reuse_proof(worker: Worker<Sandbox>, _prover: Contract, connector: Contract, proof: FullOutcomeProof, block_height: u64) {
            let _reused_proof_execution_details = connector
                .call(&worker, "mint")
                .args_json(json!({
                "proof": proof,
                "height": block_height,
            }))
                .unwrap()
                .gas(parse_gas!("300 Tgas") as u64)
                .deposit(parse_near!("30") as u128)
                .transact()
                .await
                .unwrap();
        }

        async fn mint_case1() -> (Worker<Sandbox>, Contract, Contract, FullOutcomeProof) {
            transfer_nft(
                "lock_",
                99152413,
                decode_hex("96d33e10b38722ffb2eb5031132ff775cfb6abc85b84e7596006c59f13f227de")
                    .try_into()
                    .unwrap(),
                "nftsc.90.apptest-development.testnet".to_string(),
            ).await
        }

        #[tokio::test]
        async fn test_mint_works() {
            mint_case1().await;
        }

        #[tokio::test]
        #[should_panic]
        async fn test_proof_reuse_panics() {
            let (worker, prover, connector, proof) = mint_case1().await;
            reuse_proof(worker, prover, connector, proof, 99152413).await
        }

        #[tokio::test]
        async fn test_withdraw() {
            let (worker, _prover, connector, _proof) = mint_case1().await;

            let bridged_nft_contract_id_str: String = worker
                .view(connector.id(),
                      "view_mapping",
                      serde_json::json!({
                        "source_account": NFT_CONTRACT_ACCOUNT_ID
                      }).to_string()
                          .into_bytes(),
                )
                .await.unwrap()
                .json().unwrap();


            let bridged_nft_contract_id = &workspaces::AccountId::try_from(bridged_nft_contract_id_str.to_string()).unwrap();
            let bridged_nft_after_mint: serde_json::Value = worker.view(
                bridged_nft_contract_id,
                "nft_tokens_for_owner",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": ALICE_ACCOUNT_ID,
                })).unwrap())
                .await.unwrap().json().unwrap();

            let nft_total_supply_before_burn: U128 = worker.view(
                bridged_nft_contract_id,
                "nft_total_supply",
                serde_json::to_vec(&serde_json::json!({
                })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(nft_total_supply_before_burn == U128(1));

            assert!(bridged_nft_after_mint[0]["owner_id"] == ALICE_ACCOUNT_ID);
            assert!(bridged_nft_after_mint[0]["token_id"] == "0");
            assert!(bridged_nft_after_mint[0]["metadata"]["title"] == "Luka Modric");

            // create the account where the newly minted token is, so we can withdraw the NFT
            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "lala");
            let tla = workspaces::AccountId::try_from(ALICE_ACCOUNT_ID.to_string()).unwrap();
            let account_with_bridged_nfts = worker.create_tla(tla, sec).await.unwrap().unwrap();

            // call withdraw on the bridged NFT
            let withdraw_result =
                account_with_bridged_nfts.call(
                    &worker,
                    bridged_nft_contract_id,
                    "withdraw",
                ).args_json(json!({
                    "token_id": "0"
                }))
                    .unwrap()
                    .gas(parse_gas!("300 Tgas") as u64)
                    .deposit(parse_near!("1yoctoNEAR") as u128)
                    .transact()
                    .await
                    .unwrap();

            let logs_from_withdraw = withdraw_result.logs();
            println!("LOGSS {}", logs_from_withdraw.len());
            println!("LOGS1 {}", logs_from_withdraw[0]);
            println!("LOGS2 {}", logs_from_withdraw[1]);
            assert!(logs_from_withdraw.len() == 2);

            // verify burn event happened, this event is emitted from the nft_connector_destination contract
            let parts: Vec<&str> = logs_from_withdraw[1].split(":").collect();
            assert!(parts.len() == 4);
            assert!(parts[0] == "CALIMERO_EVENT_BURN_NFT");
            assert!(parts[1] == bridged_nft_contract_id_str);
            assert!(parts[2] == account_with_bridged_nfts.id().to_string());
            assert!(parts[3] == "MA==");

            let nft_total_supply_after_burn: U128 = worker.view(
                bridged_nft_contract_id,
                "nft_total_supply",
                serde_json::to_vec(&serde_json::json!({
                })).unwrap())
                .await.unwrap()
                .json().unwrap();
            assert!(nft_total_supply_after_burn == U128(0));

            let tokens_after_burn: serde_json::Value = worker.view(
                 bridged_nft_contract_id,
                 "nft_tokens_for_owner",
                 serde_json::to_vec(&serde_json::json!({
                     "account_id": ALICE_ACCOUNT_ID
                 })).unwrap())
                 .await.unwrap()
                 .json().unwrap();
            println!("After burn {}", tokens_after_burn);
            assert!(tokens_after_burn.to_string() == "[]");
        }
    }
}