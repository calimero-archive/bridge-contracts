//! This contract is a minimal test interface for NEAR light client

extern crate near_sdk;

use admin_controlled::Mask;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};
use std::collections::VecDeque;
use types::signature::Signature;
use types::{Block, Epoch, Validator};
use utils::{hashes, Hash, Hashable};

// Current assumptions is that private shard only run max 100 block producers
const MAX_BLOCK_PRODUCERS: u32 = 100;
const NUM_OF_EPOCHS: usize = 3;

// when checking proof the last added block is always used as reference
// this gives 7 additional blocks added (at least 7 seconds) to fetch proof
// and use it to prove. 2 blocks are used for call it self which leaves (at least) 5
// seconds to retrieve proof.
const DEFAULT_BLOCKS_TO_KEEP: usize = 7;

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LightClient {
    epochs: Vec<Epoch>,
    current_height: u64,
    // Address of the account which submitted the last block.
    last_submitter: AccountId,
    // Whether the contract was initialized.
    initialized: bool,
    next_epoch: bool,
    hash: Hash,
    merkle_root: Hash,
    next_hash: Hash,
    timestamp: u64,
    signature_set: u128,
    signatures: Vec<Signature>,
    current_epoch_index: usize,
    block_hashes: VecDeque<(u64, Hash)>,
    block_merkle_roots: VecDeque<(u64, Hash)>,
    // Mask determining all paused functions
    paused: Mask,
    // number of latest added blocks client keeps
    blocks_to_keep: usize,
}

const PAUSE_ADD_BLOCK_HEADER: Mask = 1;

#[near_bindgen]
impl LightClient {
    #[init]
    pub fn new(max_blocks: Option<usize>) -> Self {
        let blocks_to_keep = if let Some(blocks_to_keep) = max_blocks {
            blocks_to_keep
        } else {
            DEFAULT_BLOCKS_TO_KEEP
        };
        Self {
            epochs: Vec::new(),
            current_height: 0,
            last_submitter: env::signer_account_id(),
            initialized: false,
            next_epoch: false,
            hash: Default::default(),
            merkle_root: Default::default(),
            next_hash: Default::default(),
            timestamp: 0,
            signature_set: 0,
            signatures: Vec::new(),
            current_epoch_index: 0,
            block_hashes: VecDeque::new(),
            block_merkle_roots: VecDeque::new(),
            paused: Mask::default(),
            blocks_to_keep,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    #[cfg(reset)]
    pub fn reset_state(&mut self) {
        near_sdk::assert_self();
        self.epochs = Vec::new();
        self.current_height = 0;
        self.initialized = false;
        self.next_epoch = false;
        self.signature_set = 0;
        self.signatures = Vec::new();
        self.current_epoch_index = 0;
        self.block_merkle_roots = VecDeque::new();
        self.block_hashes = VecDeque::new();
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
            self.signatures.push(Default::default());
        }
        LightClient::set_block_producers(&initial_validators, &mut self.epochs[0]);
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
            .push_front((self.current_height, block.hash()));
        self.block_merkle_roots
            .push_front((self.current_height, block.inner_lite.block_merkle_root));

        LightClient::set_block_producers(&block.next_bps.unwrap(), &mut self.epochs[1]);
    }

    pub fn current_height(&self) -> u64 {
        self.current_height
    }

    pub fn block_hashes(&self, height: u64) -> Option<Hash> {
        for (known_height, hash) in self.block_hashes.iter() {
            if &height == known_height {
                return Some(*hash);
            }
        }
        None
    }

    pub fn block_merkle_roots(&self, height: u64) -> Option<Hash> {
        for (known_height, merkle_root) in self.block_merkle_roots.iter() {
            if &height == known_height {
                return Some(*merkle_root);
            }
        }
        None
    }

    pub fn add_light_client_block(&mut self, block: Block) {
        require!(self.is_initialized(), "Contract is not initialized");
        self.assert_not_paused(PAUSE_ADD_BLOCK_HEADER);

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
            if block.approvals_after_next[i].is_some() {
                voted_for += this_epoch.stakes[i];
            }
        }
        require!(voted_for > this_epoch.stake_threshold, "Too few approvals");

        // If the block is from the next epoch, make sure that next_bps is supplied and has a correct hash.
        if from_next_epoch {
            require!(block.next_bps.is_some(), "Next next_bps should not be None");
            require!(
                LightClient::hash_of_block_producers(block.next_bps.as_ref().unwrap())
                    == block.inner_lite.next_bp_hash,
                "Hash of block producers does not match"
            );
        }

        self.current_height = block.inner_lite.height;
        self.timestamp = block.inner_lite.timestamp;

        self.hash = block.hash();
        self.merkle_root = block.inner_lite.block_merkle_root;
        self.next_hash = hashes::combine_hash2(block.next_block_inner_hash, self.hash);

        let keys_len = this_epoch.keys.len();
        self.signature_set = 0;
        let mut signature_stake: u128 = 0;
        for i in 0..keys_len {
            if let Some(approval) = block.approvals_after_next[i].clone() {
                self.signature_set |= 1 << i;
                self.signatures[i] = approval;
                if signature_stake < this_epoch.stake_threshold {
                    self.check_block_producer_signature_in_head(i);
                }
                signature_stake += this_epoch.stakes[i];
            }
        }
        self.next_epoch = from_next_epoch;

        if from_next_epoch {
            let mut next_epoch = &mut self.epochs[(self.current_epoch_index + 2) % NUM_OF_EPOCHS];
            next_epoch.epoch_id = block.inner_lite.next_epoch_id;
            LightClient::set_block_producers(block.next_bps.as_ref().unwrap(), next_epoch);
        }
        self.last_submitter = env::predecessor_account_id();

        self.block_hashes
            .push_front((self.current_height, block.hash()));
        self.block_merkle_roots
            .push_front((self.current_height, block.inner_lite.block_merkle_root));

        while self.block_hashes.len() > self.blocks_to_keep {
            self.block_hashes.pop_back();
            self.block_merkle_roots.pop_back();
        }

        if from_next_epoch {
            self.current_epoch_index = (self.current_epoch_index + 1) % NUM_OF_EPOCHS;
        }
    }

    pub fn check_block_producer_signature_in_head(&self, signature_index: usize) -> bool {
        require!(
            self.signature_set & (1 << signature_index) != 0,
            "No such signature"
        );
        let untrusted_epoch = &self.epochs[if self.next_epoch {
            (self.current_epoch_index + 1) % NUM_OF_EPOCHS
        } else {
            self.current_epoch_index
        }];
        let signature = &self.signatures[signature_index];
        let message = [
            &[0],
            &self.next_hash as &[_],
            &utils::swap_bytes8(self.current_height + 2).to_be_bytes() as &[_],
        ]
        .concat();

        signature.verify(&message, &untrusted_epoch.keys[signature_index])
    }

    fn hash_of_block_producers(block_producers: &Vec<Validator>) -> Hash {
        env::sha256(&block_producers.try_to_vec().expect("Failed to serialize"))
            .try_into()
            .unwrap()
    }

    fn set_block_producers(block_producers: &Vec<Validator>, epoch: &mut Epoch) {
        require!(
            (block_producers.len() as u32) <= MAX_BLOCK_PRODUCERS,
            "It is not expected having that many block producers for the provided block"
        );

        epoch.keys = Vec::new();
        epoch.stakes = Vec::new();

        let mut total_stake: u128 = 0;
        for block_producer in block_producers {
            epoch.keys.push(block_producer.public_key().clone());
            total_stake += *block_producer.stake();
            epoch.stakes.push(*block_producer.stake());
        }
        epoch.stake_threshold = (total_stake * 2) / 3;
    }
}

admin_controlled::impl_admin_controlled!(LightClient, paused);
