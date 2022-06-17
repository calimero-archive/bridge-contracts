//! This contract is a minimal test interface for NEAR light client

extern crate near_sdk;
pub mod errors;
pub mod signature;
pub mod utils;

pub use crate::signature::{PublicKey, Signature};
use admin_controlled::Mask;
pub use crate::utils::{hashes, u128_dec_format};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};

// Current assumptions is that private shard only run max 100 block producers
const MAX_BLOCK_PRODUCERS: u32 = 100;
const NUM_OF_EPOCHS: usize = 3;

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

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Validator {
    pub account_id: String,
    pub public_key: PublicKey,
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
    keys: Vec<PublicKey>,
    stake_threshold: u128,
    stakes: Vec<u128>,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LightClient {
    epochs: Vec<Epoch>,
    last_valid_at: u64,
    current_height: u64,
    // The most recently added block. May still be in its challenge period, so should not be trusted.
    untrusted_height: u64,
    // Address of the account which submitted the last block.
    last_submitter: AccountId,
    // Whether the contract was initialized.
    initialized: bool,
    untrusted_next_epoch: bool,
    untrusted_hash: Hash,
    untrusted_merkle_root: Hash,
    untrusted_next_hash: Hash,
    untrusted_timestamp: u64,
    untrusted_signature_set: u128,
    untrusted_signatures: Vec<Signature>,
    // lockDuration and replaceDuration shouldn't be extremely big, so adding them to an uint64 timestamp should not overflow uint128.
    lock_duration: u64,
    // replaceDuration is in nanoseconds, because it is a difference between NEAR timestamps.
    replace_duration: u64,
    current_epoch_index: usize,
    block_hashes: LookupMap<u64, Hash>,
    block_merkle_roots: LookupMap<u64, Hash>,
    // Mask determining all paused functions
    paused: Mask,
}

const PAUSE_ADD_BLOCK_HEADER: Mask = 1;

#[near_bindgen]
impl LightClient {
    #[init]
    pub fn new(lock_duration: u64, replace_duration: u64) -> Self {
        Self {
            epochs: Vec::new(),
            last_valid_at: 0,
            current_height: 0,
            untrusted_height: 0,
            last_submitter: env::signer_account_id(),
            initialized: false,
            untrusted_next_epoch: false,
            untrusted_hash: Default::default(),
            untrusted_merkle_root: Default::default(),
            untrusted_next_hash: Default::default(),
            untrusted_timestamp: 0,
            untrusted_signature_set: 0,
            untrusted_signatures: Vec::new(),
            lock_duration: lock_duration,
            replace_duration: replace_duration,
            current_epoch_index: 0,
            block_hashes: LookupMap::new(b"h"),
            block_merkle_roots: LookupMap::new(b"m"),
            paused: Mask::default(),
        }
    }

    pub fn is_initialized(&self) -> bool {
        return self.initialized;
    }

    /// The first part of initialization -- setting the validators of the current epoch.
    pub fn init_with_validators(&mut self, initial_validators: Vec<Validator>) {
        near_sdk::assert_self();
        require!(
            !self.is_initialized() && self.epochs.is_empty(),
            "Wrong initialization stage"
        );
        for _ in 0..NUM_OF_EPOCHS {
            self.epochs.push(Epoch {
                epoch_id: Default::default(),
                keys: Vec::new(),
                stake_threshold: 0,
                stakes: Vec::new(),
            });
        }
        for _ in 0..MAX_BLOCK_PRODUCERS {
            self.untrusted_signatures.push(Default::default());
        }
        LightClient::set_block_producers(initial_validators, &mut self.epochs[0]);
    }

    /// The second part of the initialization
    pub fn init_with_block(&mut self, block: Block) {
        near_sdk::assert_self();
        require!(
            !self.is_initialized() && !self.epochs.is_empty(),
            "Wrong initialization stage"
        );
        require!(
            block.next_bps.is_some(),
            "Initialization block must contain next_bps"
        );
        self.initialized = true;

        self.current_height = block.inner_lite.height;
        self.epochs[0].epoch_id = block.inner_lite.epoch_id;
        self.epochs[1].epoch_id = block.inner_lite.next_epoch_id;
        self.block_hashes
            .insert(&block.inner_lite.height, &block.hash);
        self.block_merkle_roots.insert(
            &block.inner_lite.height,
            &block.inner_lite.block_merkle_root,
        );
        LightClient::set_block_producers(block.next_bps.unwrap(), &mut self.epochs[1]);
    }

    pub fn block_hashes(&self, height: u64) -> Option<Hash> {
        if let Some(res) = &self.block_hashes.get(&height) {
            return Some(*res);
        } else if env::block_timestamp() >= self.last_valid_at
            && self.last_valid_at != 0
            && height == self.untrusted_height
        {
            return Some(self.untrusted_hash);
        }
        return None;
    }

    pub fn add_light_client_block(&mut self, block: Block) {
        require!(self.is_initialized(), "Contract is not initialized");
        self.assert_not_paused(PAUSE_ADD_BLOCK_HEADER);

        // Commit the previous block, or make sure that it is OK to replace it.
        if env::block_timestamp() < self.last_valid_at {
            require!(
                block.inner_lite.timestamp >= self.untrusted_timestamp + self.replace_duration,
                "Can only replace with a sufficiently newer block"
            );
        } else if self.last_valid_at != 0 {
            self.current_height = self.untrusted_height;
            if self.untrusted_next_epoch {
                self.current_epoch_index = (self.current_epoch_index + 1) % NUM_OF_EPOCHS;
            }
            self.last_valid_at = 0;

            self.block_hashes
                .insert(&self.current_height, &self.untrusted_hash);
            self.block_merkle_roots
                .insert(&self.current_height, &self.untrusted_merkle_root);
        }

        // Check that the new block's height is greater than the current one's.
        require!(
            block.inner_lite.height > self.current_height,
            "New block must have higher height"
        );

        let from_next_epoch =
            if block.inner_lite.epoch_id == self.epochs[self.current_epoch_index].epoch_id {
                false
            } else if block.inner_lite.epoch_id
                == self.epochs[(self.current_epoch_index + 1) % NUM_OF_EPOCHS].epoch_id
            {
                true
            } else {
                // in this case do a revert
                require!(false, "Epoch id of the block is not valid");
                false
            };

        // Check that the new block is signed by more than 2/3 of the validators.
        let this_epoch = if from_next_epoch {
            &self.epochs[(self.current_epoch_index + 1) % NUM_OF_EPOCHS]
        } else {
            &self.epochs[self.current_epoch_index]
        };

        // Last block in the epoch might contain extra approvals that light client can ignore.
        require!(
            block.approvals_after_next.len() >= this_epoch.keys.len(),
            "Approval list is too short"
        );

        // The sum of uint128 values cannot overflow.
        let mut voted_for: u128 = 0;
        for i in 0..this_epoch.keys.len() {
            if let Some(_) = block.approvals_after_next[i] {
                voted_for += this_epoch.stakes[i];
            }
        }
        require!(voted_for > this_epoch.stake_threshold, "Too few approvals");

        // If the block is from the next epoch, make sure that next_bps is supplied and has a correct hash.
        if from_next_epoch {
            require!(block.next_bps.is_some(), "Next next_bps should not be None");
            // TODO: Calculate hash of next block producers
            // require(
            //     HashOf(block.next_bps) == nearBlock.inner_lite.next_bp_hash,
            //     "Hash of block producers does not match"
            // );
        }

        self.untrusted_height = block.inner_lite.height;
        self.untrusted_timestamp = block.inner_lite.timestamp;
        self.untrusted_hash = block.hash;
        self.untrusted_merkle_root = block.inner_lite.block_merkle_root;
        self.untrusted_next_hash = block.next_hash;

        let mut signature_set: u128 = 0;
        let mut i = 0;
        while i < this_epoch.keys.len() {
            if let Some(approval) = block.approvals_after_next[i].clone() {
                signature_set |= 1 << i;
                self.untrusted_signatures[i] = approval;
            }
            i += 1;
        }
        self.untrusted_signature_set = signature_set;
        self.untrusted_next_epoch = from_next_epoch;
        if from_next_epoch {
            let mut next_epoch = &mut self.epochs[(self.current_epoch_index + 2) % NUM_OF_EPOCHS];
            next_epoch.epoch_id = block.inner_lite.next_epoch_id;
            LightClient::set_block_producers(block.next_bps.unwrap(), next_epoch);
        }
        self.last_submitter = env::predecessor_account_id();
        self.last_valid_at = env::block_timestamp() + self.lock_duration;
    }

    pub fn check_block_producer_signature_in_head(&self, signature_index: usize) -> bool {
        require!(
            self.untrusted_signature_set & (1 << signature_index) != 0,
            "No such signature"
        );
        let untrusted_epoch = &self.epochs[if self.untrusted_next_epoch {
            (self.current_epoch_index + 1) % NUM_OF_EPOCHS
        } else {
            self.current_epoch_index
        }];
        let signature = &self.untrusted_signatures[signature_index];
        let message = [
            &[0],
            &self.untrusted_next_hash as &[_],
            &crate::utils::swap_bytes8(self.untrusted_height + 2).to_be_bytes() as &[_],
        ]
        .concat();

        return signature.verify(&message, &untrusted_epoch.keys[signature_index]);
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
            epoch.stakes.push(block_producer.stake);
        }
        epoch.stake_threshold = (total_stake * 2) / 3;
    }
}

admin_controlled::impl_admin_controlled!(LightClient, paused);
