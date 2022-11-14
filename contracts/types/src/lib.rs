pub mod errors;
pub mod signature;

pub use crate::signature::{PublicKey, Signature};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::env;
use near_sdk::serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use utils::{
    base64_format, base_hash_format, base_hash_format_many, hashes, logging, merkle_u8_format,
    option_base64_format, swap_bytes16, swap_bytes4, swap_bytes8, u128_dec_format, u64_dec_format,
    u64_dec_format_compatible, Hash, Hashable, string_bytes_format_many
};

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
    #[serde(with = "base_hash_format")]
    pub prev_block_hash: Hash,
    #[serde(with = "base_hash_format")]
    pub next_block_inner_hash: Hash,
    pub inner_lite: BlockHeaderInnerLite,
    #[serde(with = "base_hash_format")]
    pub inner_rest_hash: Hash,
    pub next_bps: Option<Vec<Validator>>,
    pub approvals_after_next: Vec<Option<Signature>>,
}

impl_header_hash!(Block);

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
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
    pub height: u64, // Height of this block since the genesis block (height 0).
    #[serde(with = "base_hash_format")]
    pub epoch_id: Hash, // Epoch start hash of this block's epoch. Used for retrieving validator information
    #[serde(with = "base_hash_format")]
    pub next_epoch_id: Hash,
    #[serde(with = "base_hash_format")]
    pub prev_state_root: Hash, // Root hash of the state at the previous block.
    #[serde(with = "base_hash_format")]
    pub outcome_root: Hash, // Root of the outcomes of transactions and receipts.
    #[serde(with = "u64_dec_format")]
    #[serde(rename = "timestamp_nanosec")]
    pub timestamp: u64, // Timestamp at which the block was built.
    #[serde(with = "base_hash_format")]
    pub next_bp_hash: Hash, // Hash of the next epoch block producers set
    #[serde(with = "base_hash_format")]
    pub block_merkle_root: Hash,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct OptionalBlockProducers {}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Epoch {
    #[serde(with = "base_hash_format")]
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
    #[serde(with = "base_hash_format")]
    pub block_hash: Hash,
    #[serde(flatten)]
    pub outcome_with_id: ExecutionOutcomeWithId,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
#[serde(transparent)]
pub struct MerklePath {
    #[serde(flatten)]
    pub items: Vec<MerklePathItem>,
}

pub const MERKLE_PATH_LEFT: u8 = 0;
pub const MERKLE_PATH_RIGHT: u8 = 1;

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct MerklePathItem {
    #[serde(with = "base_hash_format")]
    pub hash: Hash,
    #[serde(with = "merkle_u8_format")]
    pub direction: u8, // 0 = left, 1 = right
}

impl_self_hash!(MerklePathItem);

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockHeaderLight {
    #[serde(with = "base_hash_format")]
    pub prev_block_hash: Hash,
    #[serde(with = "base_hash_format")]
    pub inner_rest_hash: Hash,
    pub inner_lite: BlockHeaderInnerLite,
}

impl_header_hash!(BlockHeaderLight);

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecutionOutcomeWithId {
    #[serde(with = "base_hash_format")]
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

        env::sha256(&bytes).try_into().unwrap()
    }
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecutionOutcome {
    #[serde(with = "string_bytes_format_many")]
    pub logs: Vec<Vec<u8>>, // Logs from this transaction or receipt.
    #[serde(with = "base_hash_format_many")]
    pub receipt_ids: Vec<Hash>, // Receipt IDs generated by this transaction or receipt.
    pub gas_burnt: u64,     // The amount of the gas burnt by the given transaction or receipt.
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
        let mut res: Vec<Hash> = vec![env::sha256(&bytes).try_into().unwrap()];
        for log in &self.logs {
            res.push(env::sha256(log).try_into().unwrap());
        }

        res
    }
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum ExecutionStatus {
    Unknown(),
    Failed(),
    #[serde(with = "base64_format")]
    SuccessValue(Vec<u8>),
    #[serde(with = "base_hash_format")]
    SuccessReceiptId(Hash),
}

/// Taken from nearcore primitives-core
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum Action {
    /// Create an (sub)account using a transaction `receiver_id` as an ID for
    /// a new account ID must pass validation rules described here
    /// <http://nomicon.io/Primitives/Account.html>.
    CreateAccount(CreateAccountAction),
    /// Sets a Wasm code to a receiver_id
    DeployContract(DeployContractAction),
    FunctionCall(FunctionCallAction),
    Transfer(TransferAction),
    // Calimero will not deal with key and stake Actions
    //Stake(StakeAction),
    //AddKey(AddKeyAction),
    //DeleteKey(DeleteKeyAction),
    //DeleteAccount(DeleteAccountAction),
}

impl Action {
    pub fn get_prepaid_gas(&self) -> u64 {
        match self {
            Action::FunctionCall(a) => a.gas,
            _ => 0,
        }
    }
    pub fn get_deposit_balance(&self) -> u128 {
        match self {
            Action::FunctionCall(a) => a.deposit,
            Action::Transfer(a) => a.deposit,
            _ => 0,
        }
    }
}

/// Create account action
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct CreateAccountAction {}

impl From<CreateAccountAction> for Action {
    fn from(create_account_action: CreateAccountAction) -> Self {
        Self::CreateAccount(create_account_action)
    }
}

/// Deploy contract action
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DeployContractAction {
    /// WebAssembly binary
    #[serde(with = "base64_format")]
    pub code: Vec<u8>,
}

impl From<DeployContractAction> for Action {
    fn from(deploy_contract_action: DeployContractAction) -> Self {
        Self::DeployContract(deploy_contract_action)
    }
}

impl fmt::Debug for DeployContractAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeployContractAction")
            .field(
                "code",
                &format_args!("{}", logging::pretty_utf8(&self.code)),
            )
            .finish()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FunctionCallAction {
    pub method_name: String,
    #[serde(with = "base64_format")]
    pub args: Vec<u8>,
    #[serde(with = "u64_dec_format_compatible")]
    pub gas: u64,
    #[serde(with = "u128_dec_format")]
    pub deposit: u128,
}

impl From<FunctionCallAction> for Action {
    fn from(function_call_action: FunctionCallAction) -> Self {
        Self::FunctionCall(function_call_action)
    }
}

impl fmt::Debug for FunctionCallAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionCallAction")
            .field("method_name", &format_args!("{}", &self.method_name))
            .field(
                "args",
                &format_args!("{}", logging::pretty_utf8(&self.args)),
            )
            .field("gas", &format_args!("{:?}", &self.gas))
            .field("deposit", &format_args!("{}", &self.deposit))
            .finish()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct TransferAction {
    #[serde(with = "u128_dec_format")]
    pub deposit: u128,
}

impl From<TransferAction> for Action {
    fn from(transfer_action: TransferAction) -> Self {
        Self::Transfer(transfer_action)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Transaction {
    pub actions: Vec<Action>,
    #[serde(with = "base_hash_format")]
    pub hash: Hash,
    pub nonce: u64,
    pub public_key: PublicKey,
    pub receiver_id: String,
    pub signature: Signature,
    pub signer_id: String,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct DataReceiver {
    #[serde(with = "base_hash_format")]
    pub data_id: Hash,
    pub receiver_id: String,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum ReceiptType {
    Action {
        signer_id: String,
        signer_public_key: PublicKey,
        #[serde(with = "u128_dec_format")]
        gas_price: u128,
        output_data_receivers: Vec<DataReceiver>,
        #[serde(with = "base_hash_format_many")]
        input_data_ids: Vec<Hash>,
        actions: Vec<Action>,
    },
    Data {
        data_id: Hash,
        #[serde(with = "option_base64_format")]
        data: Option<Vec<u8>>,
    },
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Receipt {
    pub predecessor_id: String,
    pub receipt: ReceiptType,
    #[serde(with = "base_hash_format")]
    pub receipt_id: Hash,
    pub receiver_id: String,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct TransactionStatus {
    pub receipts: Vec<Receipt>,
    pub receipts_outcome: Vec<ExecutionOutcomeWithIdAndProof>,
    pub status: ExecutionStatus,
    pub transaction: Transaction,
    pub transaction_outcome: ExecutionOutcomeWithIdAndProof,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum ConnectorType {
    FT = 0,
    NFT = 1,
    XSC = 2,
}
