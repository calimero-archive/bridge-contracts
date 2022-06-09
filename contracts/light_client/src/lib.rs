//! This contract is a minimal test interface for NEAR light client

extern crate near_sdk;
pub mod utils;

pub use crate::utils::{hashes, u128_dec_format};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, require};
use std::collections::HashMap;

// Current assumptions is that private shard only run max 100 block producers
const MAX_BLOCK_PRODUCERS: u32 = 100;

pub type Hash = [u8; 32];

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Block {
    prev_block_hash: Hash,
    next_block_inner_hash: Hash,
    inner_lite: BlockHeaderInnerLite,
    inner_rest_hash: Hash,
    next_bps: Option<Vec<Validator>>,
    approvals_after_next: Vec<Option<Signature>>,
    hash: Hash,
    next_hash: Hash,
}

impl Block {
    pub fn new(
        prev_block_hash: String,
        next_block_inner_hash: String,
        inner_lite: BlockHeaderInnerLite,
        inner_rest_hash: String,
        next_bps: Option<Vec<Validator>>,
        approvals_after_next: Vec<Option<Signature>>,
    ) -> Self {
        let inner_rest_hash_bytes = hashes::deserialize_hash(&inner_rest_hash).unwrap();
        let prev_block_hash_bytes = hashes::deserialize_hash(&prev_block_hash).unwrap();
        let inner_lite_hash_bytes: Hash =
            env::sha256(&inner_lite.try_to_vec().expect("Failed to serialize"))
                .try_into()
                .unwrap();

        let hash = hashes::combine_hash3(
            inner_lite_hash_bytes,
            inner_rest_hash_bytes,
            prev_block_hash_bytes,
        );

        let next_block_inner_hash_bytes = hashes::deserialize_hash(&next_block_inner_hash).unwrap();
        let next_hash = hashes::combine_hash2(next_block_inner_hash_bytes, hash);

        Self {
            prev_block_hash: prev_block_hash_bytes,
            next_block_inner_hash: next_block_inner_hash_bytes,
            inner_lite: inner_lite,
            inner_rest_hash: inner_rest_hash_bytes,
            next_bps: next_bps,
            approvals_after_next: approvals_after_next,
            hash: hash,
            next_hash: next_hash,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum Signature {
    //ED25519(ed25519_dalek::Signature),
    //SECP256K1(Secp256K1Signature),
    ED25519(String),
    SECP256K1(String),
}
#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Validator {
    pub account_id: String,
    pub public_key: String,
    #[serde(with = "u128_dec_format")]
    pub stake: u128,
    pub is_chunk_only: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockHeaderInnerLite {
    height: u64,    // Height of this block since the genesis block (height 0).
    epoch_id: Hash, // Epoch start hash of this block's epoch. Used for retrieving validator information
    next_epoch_id: Hash,
    prev_state_root: Hash, // Root hash of the state at the previous block.
    outcome_root: Hash,    // Root of the outcomes of transactions and receipts.
    timestamp: u64,        // Timestamp at which the block was built.
    next_bp_hash: Hash,    // Hash of the next epoch block producers set
    block_merkle_root: Hash,
}

impl BlockHeaderInnerLite {
    pub fn new(
        height: u64,
        epoch_id: String,
        next_epoch_id: String,
        prev_state_root: String,
        outcome_root: String,
        timestamp: String,
        next_bp_hash: String,
        block_merkle_root: String,
    ) -> Self {
        Self {
            height: height,
            epoch_id: hashes::deserialize_hash(&epoch_id).unwrap(),
            next_epoch_id: hashes::deserialize_hash(&next_epoch_id).unwrap(),
            prev_state_root: hashes::deserialize_hash(&prev_state_root).unwrap(),
            outcome_root: hashes::deserialize_hash(&outcome_root).unwrap(),
            timestamp: timestamp.parse().unwrap(),
            next_bp_hash: hashes::deserialize_hash(&next_bp_hash).unwrap(),
            block_merkle_root: hashes::deserialize_hash(&block_merkle_root).unwrap(),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OptionalBlockProducers {}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Epoch {
    epoch_id: Hash,
    keys: Vec<String>,
    stake_threshold: u128,
    stakes: Vec<u128>,
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LightClient {
    is_initialized: bool,
    epochs: Vec<Epoch>,
    cur_height: u64,
    block_hashes: HashMap<u64, Hash>,
    block_merkle_roots: HashMap<u64, Hash>,
}

#[near_bindgen]
impl LightClient {
    pub fn is_initialized(&self) -> bool {
        return self.is_initialized;
    }

    /// The first part of initialization -- setting the validators of the current epoch.
    pub fn init_with_validators(&mut self, initial_validators: Vec<Validator>) {
        require!(
            !self.is_initialized() && self.epochs.is_empty(),
            "Wrong initialization stage"
        );
        for _ in 1..3 {
            self.epochs.push(Epoch {
                epoch_id: Default::default(),
                keys: Vec::new(),
                stake_threshold: 0,
                stakes: Vec::new(),
            });
        }
        LightClient::set_block_producers(initial_validators, &mut self.epochs[0]);
    }

    /// The second part of the initialization
    pub fn init_with_block(&mut self, block: Block) {
        require!(
            !self.is_initialized() && !self.epochs.is_empty(),
            "Wrong initialization stage"
        );
        require!(
            block.next_bps.is_some(),
            "Initialization block must contain next_bps"
        );

        self.cur_height = block.inner_lite.height;
        self.epochs[0].epoch_id = block.inner_lite.epoch_id;
        self.epochs[1].epoch_id = block.inner_lite.next_epoch_id;
        self.block_hashes
            .insert(block.inner_lite.height, block.hash);
        self.block_merkle_roots
            .insert(block.inner_lite.height, block.inner_lite.block_merkle_root);
        LightClient::set_block_producers(block.next_bps.unwrap(), &mut self.epochs[1]);
    }

    pub fn block_hashes(&self, height: u64) -> Option<Hash> {
        if let Some(res) = self.block_hashes.get(&height) {
            return Some(*res);
        } else if env::block_timestamp() > 0 {
            return None;
        }
        return None;
    }

    ///
    pub fn add_light_client_block(&mut self, block: Block) {}

    fn set_block_producers(block_producers: Vec<Validator>, epoch: &mut Epoch) {
        require!(
            (block_producers.len() as u32) <= MAX_BLOCK_PRODUCERS,
            "It is not expected having that many block producers for the provided block"
        );

        let mut total_stake: u128 = 0;
        for block_producer in &block_producers {
            epoch.keys.push(block_producer.public_key.clone());
            total_stake += block_producer.stake;
            epoch.stakes.push(block_producer.stake);
        }
        epoch.stake_threshold = (total_stake * 2) / 3;
    }
}
