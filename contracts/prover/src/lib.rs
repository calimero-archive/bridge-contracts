extern crate near_sdk;

use admin_controlled::Mask;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, require, serde_json, PanicOnDefault, PromiseResult};
use types::{FullOutcomeProof, MerklePath};
pub use utils::{hashes, Hash, Hashable};

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct Prover {
    // account id of light client
    light_client_account_id: String,
    // Mask determining all paused functions
    paused: Mask,
}

#[near_bindgen]
impl Prover {
    #[init]
    pub fn new(light_client_account_id: String) -> Self {
        Prover {
            light_client_account_id,
            paused: Mask::default(),
        }
    }

    // cross contract calls used, hence this is not view method
    pub fn prove_outcome(&self, full_outcome_proof: FullOutcomeProof, block_height: u64) {
        let mut hash = Prover::compute_root(
            &full_outcome_proof.outcome_proof.outcome_with_id.hash(),
            full_outcome_proof.outcome_proof.proof,
        );

        hash = Prover::compute_root(
            &env::sha256(&hash).try_into().unwrap(),
            full_outcome_proof.outcome_root_proof,
        );

        require!(
            hash == full_outcome_proof.block_header_lite.inner_lite.outcome_root,
            "NearProver: outcome merkle proof is not valid",
        );

        let promise_merkle_root = env::promise_create(
            self.light_client_account_id.parse().unwrap(),
            "block_merkle_roots",
            &serde_json::to_vec(&(block_height,)).unwrap(),
            0,
            env::prepaid_gas() / 3,
        );

        let promise_result = env::promise_then(
            promise_merkle_root,
            env::current_account_id(),
            "merkle_root_callback",
            &serde_json::to_vec(&(
                full_outcome_proof.block_header_lite.hash(),
                full_outcome_proof.block_proof,
            ))
            .unwrap(),
            0,
            env::prepaid_gas() / 3,
        );

        env::promise_return(promise_result)
    }

    pub fn merkle_root_callback(&self, block_header_lite_hash: Hash, block_proof: MerklePath) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let expected_block_merkle_root = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<Option<Hash>>(&x).unwrap(),
            _ => env::panic_str("Merkle root promise failed"),
        };

        let computed_block_merkle_root = Prover::compute_root(&block_header_lite_hash, block_proof);

        require!(
            expected_block_merkle_root == Some(computed_block_merkle_root),
            "NearProver: block proof is not valid"
        );
        env::value_return(&serde_json::to_vec(&true).unwrap());
    }

    fn compute_root(node: &Hash, path: MerklePath) -> Hash {
        let mut hash: Hash = *node;
        for item in path.items {
            hash = match item.direction {
                types::MERKLE_PATH_LEFT => hashes::combine_hash2(item.hash(), hash),
                types::MERKLE_PATH_RIGHT => hashes::combine_hash2(hash, item.hash()),
                _ => panic!("NearProver: unknown merkle path"),
            }
        }
        hash
    }
}
