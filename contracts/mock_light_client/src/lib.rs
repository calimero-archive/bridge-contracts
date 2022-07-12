extern crate near_sdk;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, PanicOnDefault};
use std::collections::HashMap;
use utils::Hash;

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct MockLightClient {
    merkle_roots: HashMap<u64, Hash>,
}

#[near_bindgen]
impl MockLightClient {
    #[init]
    pub fn new() -> Self {
        MockLightClient {
            merkle_roots: HashMap::new(),
        }
    }

    pub fn add_merkle_root(&mut self, height: &u64, hash: &Hash) {
        self.merkle_roots.insert(*height, *hash);
    }

    pub fn block_merkle_roots(&self, height: u64) -> Option<Hash> {
        for (k, v) in self.merkle_roots.iter() {
            env::log_str(format!("{} {:?}", k, v).as_str());
        }
        match self.merkle_roots.get(&height) {
            Some(&x) => Some(x),
            None => None,
        }
    }
}
