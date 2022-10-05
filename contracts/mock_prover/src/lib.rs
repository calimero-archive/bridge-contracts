extern crate near_sdk;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, serde_json, PanicOnDefault};
use std::collections::HashSet;
use types::FullOutcomeProof;
use utils::Hash;

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct MockProver {
    approved_hashes: HashSet<Hash>,
}

#[near_bindgen]
impl MockProver {
    #[init]
    pub fn new() -> Self {
        MockProver {
            approved_hashes: HashSet::new(),
        }
    }

    pub fn add_approved_hash(&mut self, hash: &Hash) {
        self.approved_hashes.insert(*hash);
    }

    pub fn prove_outcome(&self, full_outcome_proof: FullOutcomeProof, block_height: u64) {
        env::promise_return(env::promise_create(
            env::current_account_id(),
            "check_hash",
            &serde_json::to_vec(&(full_outcome_proof.outcome_proof.block_hash, block_height))
                .unwrap(),
            0,
            env::prepaid_gas() / 2,
        ));
    }

    pub fn check_hash(&self, hash: Hash, _height: u64) {
        near_sdk::assert_self();
        match self.approved_hashes.get(&hash) {
            Some(_) => env::value_return(&serde_json::to_vec(&(true,)).unwrap()),
            None => panic!("Not approved hash"),
        }
    }
}
