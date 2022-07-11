pub mod errors;
pub mod signature;

pub use crate::signature::{PublicKey, Signature};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::env;
use near_sdk::serde::{Deserialize, Serialize};
use utils::{hashes, swap_bytes16, swap_bytes4, swap_bytes8, u64_dec_format, u128_dec_format, Hash, Hashable};

#[macro_export]
macro_rules! impl_header_hash {
    ($struct: ident) => {
        impl Hashable for $struct {
            fn hash(&self) -> Hash {
                let inner_lite_hash_bytes: Hash =
                    env::sha256(&self.inner_lite.try_to_vec().expect("Failed to serialize"))
                        .try_into()
                        .unwrap();
                let hash = hashes::combine_hash3(
                    inner_lite_hash_bytes,
                    self.inner_rest_hash,
                    self.prev_block_hash,
                );
                hash
            }
        }
    };
}

#[macro_export]
macro_rules! impl_self_hash {
    ($struct: ident) => {
        impl Hashable for $struct {
            fn hash(&self) -> Hash {
                self.hash
            }
        }
    };
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Block {
    pub prev_block_hash: Hash,
    pub next_block_inner_hash: Hash,
    pub inner_lite: BlockHeaderInnerLite,
    pub inner_rest_hash: Hash,
    pub next_bps: Option<Vec<Validator>>,
    pub approvals_after_next: Vec<Option<Signature>>,
}

impl_header_hash!(Block);

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum Validator {
    V1(ValidatorV1),
    V2(ValidatorV2),
}

impl Validator {
    pub fn new_v1(account_id: String, public_key: PublicKey, stake: u128) -> Self {
        Self::V1(ValidatorV1 {
            account_id,
            public_key,
            stake,
        })
    }

    pub fn new_v2(
        account_id: String,
        public_key: PublicKey,
        stake: u128,
        is_chunk_only: bool,
    ) -> Self {
        Self::V2(ValidatorV2 {
            account_id,
            public_key,
            stake,
            is_chunk_only,
        })
    }

    pub fn account_id(&self) -> &String {
        match self {
            Self::V1(v1) => &v1.account_id,
            Self::V2(v2) => &v2.account_id,
        }
    }

    pub fn public_key(&self) -> &PublicKey {
        match self {
            Self::V1(v1) => &v1.public_key,
            Self::V2(v2) => &v2.public_key,
        }
    }

    pub fn stake(&self) -> &u128 {
        match self {
            Self::V1(v1) => &v1.stake,
            Self::V2(v2) => &v2.stake,
        }
    }

    pub fn is_chunk_only(&self) -> &bool {
        match self {
            Self::V1(_v1) => &false,
            Self::V2(v2) => &v2.is_chunk_only,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ValidatorV1 {
    pub account_id: String,
    pub public_key: PublicKey,
    #[serde(with = "u128_dec_format")]
    pub stake: u128,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ValidatorV2 {
    pub account_id: String,
    pub public_key: PublicKey,
    #[serde(with = "u128_dec_format")]
    pub stake: u128,
    pub is_chunk_only: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockHeaderInnerLite {
    pub height: u64,    // Height of this block since the genesis block (height 0).
    pub epoch_id: Hash, // Epoch start hash of this block's epoch. Used for retrieving validator information
    pub next_epoch_id: Hash,
    pub prev_state_root: Hash, // Root hash of the state at the previous block.
    pub outcome_root: Hash,    // Root of the outcomes of transactions and receipts.
    #[serde(with = "u64_dec_format")]
    pub timestamp: u64,        // Timestamp at which the block was built. 
    pub next_bp_hash: Hash,    // Hash of the next epoch block producers set
    pub block_merkle_root: Hash,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct OptionalBlockProducers {}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Epoch {
    pub epoch_id: Hash,
    pub keys: Vec<PublicKey>,
    pub stake_threshold: u128,
    pub stakes: Vec<u128>,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct FullOutcomeProof {
    pub outcome_proof: ExecutionOutcomeWithIdAndProof,
    pub outcome_root_proof: MerklePath, // TODO: now empty array
    pub block_header_lite: BlockHeaderLight,
    pub block_proof: MerklePath,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecutionOutcomeWithIdAndProof {
    pub proof: MerklePath,
    pub block_hash: Hash,
    pub outcome_with_id: ExecutionOutcomeWithId,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct MerklePath {
    pub items: Vec<MerklePathItem>,
}

pub const MERKLE_PATH_LEFT: u8 = 0;
pub const MERKLE_PATH_RIGHT: u8 = 1;

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct MerklePathItem {
    pub hash: Hash,
    pub direction: u8, // 0 = left, 1 = right
}

impl_self_hash!(MerklePathItem);

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockHeaderLight {
    pub prev_block_hash: Hash,
    pub inner_rest_hash: Hash,
    pub inner_lite: BlockHeaderInnerLite,
}

impl_header_hash!(BlockHeaderLight);

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecutionOutcomeWithId {
    pub id: Hash, // The transaction hash or the receipt ID.
    pub outcome: ExecutionOutcome,
}

impl Hashable for ExecutionOutcomeWithId {
    fn hash(&self) -> Hash {
        let merkelization_hashes = self.outcome.merkelization_hashes();
        let len = 1 + merkelization_hashes.len();
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend(&swap_bytes4(len.try_into().unwrap()).to_be_bytes());
        bytes.extend(self.id.try_to_vec().expect("Failed to serialize"));
        for hash in merkelization_hashes {
            bytes.extend(&hash);
        }

        return env::sha256(&bytes).try_into().unwrap();
    }
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecutionOutcome {
    pub logs: Vec<Vec<u8>>,     // Logs from this transaction or receipt.
    pub receipt_ids: Vec<Hash>, // Receipt IDs generated by this transaction or receipt.
    pub gas_burnt: u64,         // The amount of the gas burnt by the given transaction or receipt.
    #[serde(with = "u128_dec_format")]
    pub tokens_burnt: u128, // The total number of the tokens burnt by the given transaction or receipt.
    pub executor_id: String, // The transaction or receipt id that produced this outcome.
    pub status: ExecutionStatus, // Execution status. Contains the result in case of successful execution.
}

impl ExecutionOutcome {
    pub fn merkelization_hashes(&self) -> Vec<Hash> {
        let mut bytes: Vec<u8> = Vec::new();
        let receipt_len: u32 = self.receipt_ids.len() as u32;
        bytes.extend(&swap_bytes4(receipt_len).to_be_bytes());
        for receipt_id in &self.receipt_ids {
            bytes.extend(&receipt_id.try_to_vec().expect("Failed to serialize"));
        }
        bytes.extend(&swap_bytes8(self.gas_burnt).to_be_bytes());
        bytes.extend(&swap_bytes16(self.tokens_burnt).to_be_bytes());
        bytes.extend(&self.executor_id.try_to_vec().expect("Failed to serialize"));
        bytes.extend(&self.status.try_to_vec().expect("Failed to serialize"));
        let mut res: Vec<Hash> = Vec::new();
        res.push(env::sha256(&bytes).try_into().unwrap());
        for log in &self.logs {
            res.push(env::sha256(&log).try_into().unwrap());
        }

        return res;
    }
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum ExecutionStatus {
    Unknown(),
    Failed(),
    SuccessValue(Vec<u8>),
    SuccessReceiptId(Hash),
}
