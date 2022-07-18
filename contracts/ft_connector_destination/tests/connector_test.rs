#[cfg(test)]
mod connector {
    mod test {
        use near_sdk::serde_json::json;
        use near_units::{parse_gas, parse_near};
        use test_utils::file_as_json;
        use utils::hashes::decode_hex;
        use utils::Hash;
        use types::{FullOutcomeProof, TransactionStatus};
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
                "prover_account": prover_contract.id().to_string()
            }))
            .unwrap()
            .transact()
            .await
            .unwrap();

            (worker, prover_contract, connector_contract)
        }

        async fn transfer_ft(file_prefix: &str, block_height: u64, hash: Hash, locker_account: String) {
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
            let transaction = &file_as_json::<TransactionStatus>(&format!("{}{}", file_prefix, "transaction.json")).unwrap();

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

            let execution_details = connector
            .call(&worker, "mint")
            .args_json(json!({
                "transaction": transaction,
                "proof": proof,
                "height": block_height,
            }))
            .unwrap()
            .gas(parse_gas!("300 Tgas") as u64)
            .deposit(parse_near!("25") as u128)
            .transact()
            .await
            .unwrap();

            println!("{:?}", execution_details);
            
            assert!(execution_details.is_success(), "Not correct proof");
        }

        #[tokio::test]
        async fn test() {
          transfer_ft(
                "mint_",
                498,
                decode_hex("5cce013ecbe6998b332435cfb5b6fd72f1ec1349549b3f06482dd9f5b57795a5")
                    .try_into()
                    .unwrap(),
                "dev-1658741559551-54592026254591".to_string(),
                ).await;
        }
    }
}
