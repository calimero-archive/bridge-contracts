extern crate near_sdk;

use admin_controlled::Mask;
pub use utils::{hashes, Hash, Hashable};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, near_bindgen, require, PanicOnDefault, Promise};
use types::{FullOutcomeProof, MerklePath};

#[ext_contract(ext_light_client)]
pub trait RemoteLightClient {
    fn block_merkle_roots(&self, height: u64) -> Hash;
}

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

    pub fn prove_outcome(
        &self,
        full_outcome_proof: FullOutcomeProof,
        block_height: u64,
    ) -> Promise {
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

        ext_light_client::ext(self.light_client_account_id.parse().unwrap())
            .block_merkle_roots(block_height)
            .then(Self::ext(env::current_account_id()).merkle_root_callback(
                full_outcome_proof.block_header_lite.hash(),
                full_outcome_proof.block_proof,
            ))
    }

    #[private]
    pub fn merkle_root_callback(
        #[callback_unwrap] expected_block_merkle_root: Hash,
        block_header_lite_hash: Hash,
        block_proof: MerklePath,
    ) -> bool {
        let computed_block_merkle_root = Prover::compute_root(&block_header_lite_hash, block_proof);
        // TODO remove when tests reach this code
        println!("{:?}", "ENTERED CALLBACK");

        require!(
            expected_block_merkle_root == computed_block_merkle_root,
            "NearProver: block proof is not valid"
        );

        return true;
    }

    fn compute_root(node: &Hash, path: MerklePath) -> Hash {
        let mut hash: Hash = *node;
        for item in path.items {
            hash = match item.direction {
                types::MERKLE_PATH_LEFT => hashes::combine_hash2(item.hash(), hash),
                types::MERKLE_PATH_RIGHT => hashes::combine_hash2(hash, item.hash()),
                _ => panic!("NearProver: unknown merkle path"),
            }
            .try_into()
            .unwrap()
        }
        return hash;
    }
}
