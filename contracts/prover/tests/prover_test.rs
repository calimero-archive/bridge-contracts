#[cfg(test)]
mod prover {
    mod test {
        use near_sdk::serde_json::json;
        use near_units::parse_gas;
        use types::FullOutcomeProof;
        use test_utils::file_as_json;
        use utils::hashes::decode_hex;
        use utils::Hash;
        use workspaces::prelude::*;
        use workspaces::{network::Sandbox, Contract, Worker};

        async fn init() -> (Worker<Sandbox>, Contract, Contract) {
            let worker = workspaces::sandbox().await.unwrap();
            // deploy contracts
            let bridge_wasm = std::fs::read(
                "../target/wasm32-unknown-unknown/release/mock_light_client.wasm",
            )
            .unwrap();
            let bridge_contract = worker.dev_deploy(&bridge_wasm).await.unwrap();
            let prover_wasm = std::fs::read(
                "../target/wasm32-unknown-unknown/release/prover.wasm",
            )
            .unwrap();
            let prover_contract = worker.dev_deploy(&prover_wasm).await.unwrap();

            // initialize contracts
            bridge_contract
            .call(&worker, "new")
            .args_json(json!({}))
            .unwrap()
            .transact()
            .await
            .unwrap();
            
            prover_contract
            .call(&worker, "new")
            .args_json(json!({
                "light_client_account_id": bridge_contract.id().to_string()
            }))
            .unwrap()
            .transact()
            .await
            .unwrap();

            (worker, bridge_contract, prover_contract)
        }

        // if call does not panic, it is considered valid, since return is always true
        async fn proof_valid(filename: &str, block_height: u64, block_merkle_root: Hash) {
            let (worker, bridge, prover) = init().await;
            bridge
            .call(&worker, "add_merkle_root")
            .args_json(json!({
                "height": block_height,
                "hash": block_merkle_root,
            }))
            .unwrap()
            .transact()
            .await
            .unwrap();
            
            let proof = &file_as_json::<FullOutcomeProof>(filename).unwrap();
            let execution_details = prover
            .call(&worker, "prove_outcome")
            .args_json(json!({
                "block_height": block_height,
                "full_outcome_proof": proof,
            }))
            .unwrap()
            .gas(parse_gas!("300 Tgas") as u64)
            .transact()
            .await
            .unwrap();
            
            assert!(execution_details.is_success(), "Not correct proof");
        }

        #[tokio::test]
        async fn proof2() {
            proof_valid(
                "proof2.json",
                498,
                decode_hex("22f00dd154366d758cd3e4fe81c1caed8e0db6227fe4b2b52a8e5a468aa0a723")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        async fn proof3() {
            proof_valid(
                "proof3.json",
                1705,
                decode_hex("0d0776820a9a81481a559c36fd5d69c33718fb7d7fd3be7564a446e043e2cb35")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        async fn proof4() {
            proof_valid(
                "proof4.json",
                5563,
                decode_hex("1f7129496c461c058fb3daf258d89bf7dacb4efad5742351f66098a00bb6fa53")
                    .try_into()
                    .unwrap(),
                ).await;
        }
        
        #[tokio::test]
        async fn proof5() {
            proof_valid(
                "proof5.json",
                384,
                decode_hex("a9cd8eb4dd92ba5f2fef47d68e1d73ac8c57047959f6f8a2dcc664419e74e4b8")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        async fn proof6() {
            proof_valid(
                "proof6.json",
                377,
                decode_hex("cc3954a51b7c1a86861df8809f79c2bf839741e3e380e28360b8b3970a5d90bd")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        async fn proof7() {
            proof_valid(
                "proof7.json",
                93544034,
                decode_hex("8298c9cd1048df03e9ccefac4b022636a30a2f7e6a8c33cc4104901b92e08dfd")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        async fn proof8() {
            proof_valid(
                "proof8.json",
                93571735,
                decode_hex("9f0e0586da201bf08a2150f3e4e8525b812c415751c4f635cbe3d0f3bdd491e6")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        async fn proof9() {
            proof_valid(
                "proof9.json",
                2238,
                decode_hex("477243d6526e351ee0bea6f97fde49bdb7b71602279afd9f3591989f2d9ea79f")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        async fn proof10() {
            proof_valid(
                "proof10.json",
                95868967,
                decode_hex("685eba49d8de4d6020db910f2f982305b546f85b151ba9b78c281785d0731475")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof2_fail() {
            proof_valid(
                "proof2.json",
                498,
                decode_hex("22f00dd154366d758cd3e4fe81c1caed8e0db6227fe4b2b52a8e5a468aa0a724")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof3_fail() {
            proof_valid(
                "proof3.json",
                1705,
                decode_hex("0d0776820a9a81481a559c36fd5d69c33718fb7d7fd3be7564a446e043e2cb36")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof4_fail() {
            proof_valid(
                "proof4.json",
                5563,
                decode_hex("1f7129496c461c058fb3daf258d89bf7dacb4efad5742351f66098a00bb6fa54")
                    .try_into()
                    .unwrap(),
                ).await;
        }
        
        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof5_fail() {
            proof_valid(
                "proof5.json",
                384,
                decode_hex("a9cd8eb4dd92ba5f2fef47d68e1d73ac8c57047959f6f8a2dcc664419e74e4b9")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof6_fail() {
            proof_valid(
                "proof6.json",
                377,
                decode_hex("cc3954a51b7c1a86861df8809f79c2bf839741e3e380e28360b8b3970a5d90be")
                    .try_into()
                    .unwrap(),
                ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof7_fail() {
            proof_valid(
                "proof7.json",
                93544034,
                decode_hex("8298c9cd1048df03e9ccefac4b022636a30a2f7e6a8c33cc4104901b92e08dfe")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof8_fail() {
            proof_valid(
                "proof8.json",
                93571735,
                decode_hex("9f0e0586da201bf08a2150f3e4e8525b812c415751c4f635cbe3d0f3bdd491e7")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof9_fail() {
            proof_valid(
                "proof9.json",
                2238,
                decode_hex("477243d6526e351ee0bea6f97fde49bdb7b71602279afd9f3591989f2d9ea79e")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "block proof is not valid")]
        async fn proof10_fail() {
            proof_valid(
                "proof10.json",
                95868967,
                decode_hex("685eba49d8de4d6020db910f2f982305b546f85b151ba9b78c281785d0731476")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "outcome merkle proof is not valid")]
        async fn incorrect_proof1() {
            proof_valid(
                "incorrect_proof1.json",
                99681735,
                decode_hex("11e6a298d110201c162fe50fad27e780dbb4ddb9cb8d61b454c25529adee31cc")
                    .try_into()
                    .unwrap(),
            ).await;
        }

        #[tokio::test]
        #[should_panic(expected = "outcome merkle proof is not valid")]
        async fn incorrect_proof2() {
            proof_valid(
                "incorrect_proof2.json",
                99681735,
                decode_hex("1837c82163944ddbfd246895920c1712f7c0b341eb5dd19f3fa08f3bb0e73143")
                    .try_into()
                    .unwrap(),
            ).await;
        }

    }
}
