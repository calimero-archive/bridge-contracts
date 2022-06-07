//! This contract is a minimal test interface for NEAR light client

extern crate near_sdk;

use near_sdk::{require, near_bindgen};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

// Current assumptions is that private shard only run max 100 block producers
const MAX_BLOCK_PRODUCERS: u32 = 100;

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Block {

}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
struct Epoch {
    epoch_id: String,
    keys: Vec<String>,
    stake_threshold: u128,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Validator {
    account_id: String,
    public_key: String,
    stake: u128,
    validator_stake_struct_version: String
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LightClient {
    epochs: Vec<Epoch>,
}

#[near_bindgen]
impl LightClient {
    pub fn is_initialized(&self) -> bool {
        return false;
    }

    /// The first part of initialization -- setting the validators of the current epoch.
    pub fn init_with_validators(&mut self, initial_validators: Vec<Validator>) {
        require!(!self.is_initialized() && self.epochs.is_empty(), "Wrong initialization stage");
        self.epochs.push(Epoch{epoch_id: "".to_string(), keys: Vec::new(), stake_threshold: 0});
        LightClient::set_block_producers(initial_validators, &mut self.epochs[0]);
    }

    /// The second part of the initialization
    pub fn init_with_block(&mut self, block: Block) {
        require!(!self.is_initialized() && !self.epochs.is_empty(), "Wrong initialization stage");
        // set initialized to true and continue with logic
    }

    ///
    pub fn add_light_client_block(&mut self, block: Block) {
    }

    fn set_block_producers(block_producers: Vec<Validator>, epoch: &mut Epoch) {
        require!(
            (block_producers.len() as u32) <= MAX_BLOCK_PRODUCERS,
            "It is not expected having that many block producers for the provided block"
        );

        let mut total_stake: u128 = 0;
        for block_producer in &block_producers {
            epoch.keys.push(block_producer.public_key.clone());
            total_stake += block_producer.stake;
        }
        epoch.stake_threshold = (total_stake * 2) / 3;
    }
}
