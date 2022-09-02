#[cfg(all(test, not(target_arch = "wasm32")))]
mod connector {
    mod test {
        use near_sdk::{AccountId, serde_json};
        use near_sdk::serde_json::json;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use utils::hashes::{decode_hex, deserialize_hash};
        use utils::Hash;
        use types::{FullOutcomeProof, TransactionStatus};
        use types::signature::SecretKey;
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker};

        async fn init() -> (Worker<Sandbox>, Contract, Contract) {
            let worker = workspaces::sandbox().await.unwrap();
            // deploy contracts
            let prover_wasm = std::fs::read(
                "../mock_prover/target/wasm32-unknown-unknown/release/mock_prover.wasm",
            )
                .unwrap();
            let prover_contract = worker.dev_deploy(&prover_wasm).await.unwrap();
            let connector_wasm = std::fs::read(
                "./target/wasm32-unknown-unknown/release/ft_connector_destination.wasm",
            )
                .unwrap();
            let connector_contract = worker.dev_deploy(&connector_wasm).await.unwrap();

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
                "source_master_account": "testnet",
                "destination_master_account": "testnettestnettestnettestnet1234"
            }))
                .unwrap()
                .transact()
                .await
                .unwrap();

            (worker, prover_contract, connector_contract)
        }

        async fn transfer_ft(file_prefix: &str, block_height: u64, hash: Hash, locker_account: String) -> (Worker<Sandbox>, Contract, Contract, FullOutcomeProof) {
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

            let proof = &file_as_json::<FullOutcomeProof>(&format!("{}{}", file_prefix, "proof.json")).unwrap();

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
                "source_address": "usdn.testnet",
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
                .deposit(parse_near!("25") as u128)
                .transact()
                .await
                .unwrap();

            assert!(execution_details.is_success(), "Not correct proof");

            (worker, prover, connector, proof.clone())
        }

        async fn reuse_proof(worker: Worker<Sandbox>, prover: Contract, connector: Contract, proof: FullOutcomeProof, block_height: u64) {
            let reused_proof_execution_details = connector
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

        async fn mint_case1() -> (Worker<Sandbox>, Contract, Contract, FullOutcomeProof) {
            transfer_ft(
                "mint_",
                99152413,
                decode_hex("19171804be07b83588399e9d0dc6864197a1d676f7b5f6c59991b7857809f2b7")
                    .try_into()
                    .unwrap(),
                "connector.lucija.igi.testnet".to_string(),
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
            let (worker, prover, connector, proof) = mint_case1().await;

            let bridged_ft_contract_id_str: String = worker
                .view(connector.id(),
                      "view_mapping",
                      serde_json::json!({
                        "source_account": "usdn.testnet"
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
                    "account_id": "igi.testnettestnettestnettestnet1234"
                })).unwrap())
                .await.unwrap()
                .json().unwrap();

            assert!(balance_after_mint == "888");

            // create the account where the newly minted tokens are, so we can withdraw some amount
            let sec = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "lala");
            let tla = workspaces::AccountId::try_from("testnettestnettestnettestnet1234".to_string()).unwrap();
            let root_account = worker.create_tla(tla, sec).await.unwrap().unwrap();

            let secret_for_subaccount = workspaces::types::SecretKey::from_seed(workspaces::types::KeyType::ED25519, "lala_sub");
            let account_with_bridged_fts = root_account.create_subaccount(&worker, "igi")
                .initial_balance(parse_near!("25") as u128)
                .keys(secret_for_subaccount)
                .transact()
                .await
                .unwrap()
                .unwrap();

            // call withdraw on the bridged FT
            let withdraw_result =
                account_with_bridged_fts.call(
                    &worker,
                    bridged_ft_contract_id,
                    "withdraw",
                ).args_json(json!({
                    "amount": "88"
                }))
                    .unwrap()
                    .gas(parse_gas!("300 Tgas") as u64)
                    .deposit(parse_near!("1yoctoNEAR") as u128)
                    .transact()
                    .await
                    .unwrap();

            let logs_from_withdraw = withdraw_result.logs();
            assert!(logs_from_withdraw.len() == 1);

            // verify burn event happened, this event is emitted from the ft_connector_destination contract
            let parts: Vec<&str> = logs_from_withdraw[0].split(":").collect();
            assert!(parts.len() == 4);
            assert!(parts[0] == "CALIMERO_EVENT_BURN");
            assert!(parts[1] == bridged_ft_contract_id_str);
            assert!(parts[2] == account_with_bridged_fts.id().to_string());
            assert!(parts[3] == "88");

            let balance_after_burn: String = worker.view(
                bridged_ft_contract_id,
                "ft_balance_of",
                serde_json::to_vec(&serde_json::json!({
                    "account_id": "igi.testnettestnettestnettestnet1234"
                })).unwrap())
                .await.unwrap()
                .json().unwrap();

            assert!(balance_after_burn == "800");
        }
    }
}
