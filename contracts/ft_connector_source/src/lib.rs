use admin_controlled::Mask;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde_json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault, PromiseResult,
};

use types::FullOutcomeProof;
use utils::{hashes, Hash, Hashable};

const NO_DEPOSIT: Balance = 0;

/// Gas to call mint method on bridge token.
const MINT_GAS: Gas = Gas(10_000_000_000_000);

/// Gas to call ft_transfer_call when the target of deposit is a contract
const FT_TRANSFER_CALL_GAS: Gas = Gas(80_000_000_000_000);

/// Gas to call finish deposit method.
/// This doesn't cover the gas required for calling mint method.
const FINISH_DEPOSIT_GAS: Gas = Gas(30_000_000_000_000);

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = Gas(50_000_000_000_000);

/// Gas to call register method on FT.
const REGISTER_FT_GAS: Gas = Gas(50_000_000_000_000);

const CALIMERO_EVENT: &str = "CALIMERO_EVENT";

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct FungibleTokenConnector {
    /// The account of the prover that we can use to prove
    pub prover_account: AccountId,
    source_master_account: AccountId,
    destination_master_account: AccountId,
    /// The account of the locker on other network that is used to burn FT
    pub locker_account: Option<AccountId>,
    /// Hashes of the events that were already used.
    pub used_events: UnorderedSet<Hash>,
    /// Mappings between FT contract on main network and FT contract on this network
    contracts_mapping: UnorderedMap<AccountId, AccountId>,
    /// Mask determining all paused functions
    paused: Mask,
}

#[near_bindgen]
impl FungibleTokenConnector {
    /// Initializes the contract.
    /// `prover_account`: NEAR account of the Near Prover contract;
    /// `source_master_account`: NEAR master account on source network, ex. 'testnet'
    /// `destination_master_account`: NEAR master account on this network, ex. 'shard.calimero.testnet'
    #[init]
    pub fn new(
        prover_account: AccountId,
        source_master_account: AccountId,
        destination_master_account: AccountId,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        Self {
            prover_account,
            source_master_account,
            destination_master_account,
            used_events: UnorderedSet::new(b"u".to_vec()),
            contracts_mapping: UnorderedMap::new(b"c".to_vec()),
            locker_account: None,
            paused: Mask::default(),
        }
    }

    #[payable]
    pub fn set_locker(&mut self, locker_account: AccountId) {
        near_sdk::assert_self();
        require!(self.locker_account.is_none());
        let initial_storage = env::storage_usage() as u128;
        self.locker_account = Some(locker_account);
        let current_storage = env::storage_usage() as u128;
        require!(
            env::attached_deposit()
                >= env::storage_byte_cost() * (current_storage - initial_storage),
            "Not enough attached deposit to complete network connection"
        );
    }

    #[payable]
    pub fn register_ft(&mut self, ft_address: AccountId, method: String) {
        env::promise_return(env::promise_create(
            ft_address,
            &method,
            &Vec::<u8>::new(),
            env::attached_deposit(),
            REGISTER_FT_GAS,
        ))
    }

    /// Used when sending FT to other network
    /// `msg` is expected to contain valid other network id
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        self.lock(sender_id, amount, msg)
    }

    fn lock(&mut self, sender_id: AccountId, amount: U128, _msg: String) -> U128 {
        env::log_str(&format!(
            "CALIMERO_EVENT:{}:{}:{}",
            env::predecessor_account_id(),
            sender_id,
            amount.0
        ));
        U128(0)
    }

    #[payable]
    pub fn map_contracts(&mut self, source_contract: AccountId, destination_contract: AccountId) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<Vec<bool>>(&x).unwrap()[0],
            _ => env::panic_str("Prover failed"),
        };
        require!(verification_success, "Failed to verify the proof");

        self.contracts_mapping
            .insert(&destination_contract, &source_contract);
    }

    pub fn register_ft_on_private(&mut self, proof: FullOutcomeProof, height: u64) {
        require!(!self.locker_account.is_none());
        require!(
            proof.outcome_proof.outcome_with_id.outcome.executor_id
                == self.locker_account.as_ref().unwrap().to_string(),
            "Untrusted proof, deploy_bridge_token receipt proof required"
        );
        let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
        let parts: Vec<&str> = std::str::from_utf8(&event_log)
            .unwrap()
            .split(":")
            .collect();
        require!(
            parts.len() == 3 && parts[0] == CALIMERO_EVENT,
            "Untrusted proof, deploy_bridge_token receipt proof required"
        );

        let ft_token_contract_account_source: AccountId = parts[1].parse().unwrap();
        let ft_token_contract_account_destination: AccountId = parts[2].parse().unwrap();

        // check that account deployment was done by locker_account
        let promise_prover = env::promise_create(
            self.prover_account.clone(),
            "prove_outcome",
            &serde_json::to_vec(&(proof.clone(), height)).unwrap(),
            NO_DEPOSIT,
            VERIFY_LOG_ENTRY_GAS,
        );

        let promise_result = env::promise_then(
            promise_prover,
            env::current_account_id(),
            "map_contracts",
            &serde_json::to_vec(&(
                ft_token_contract_account_source,
                ft_token_contract_account_destination,
            ))
            .unwrap(),
            env::attached_deposit(),
            FINISH_DEPOSIT_GAS,
        );

        env::promise_return(promise_result)
    }

    /// Used when receiving FT from other network
    #[payable]
    pub fn unlock(&mut self, proof: FullOutcomeProof, height: u64) {
        require!(!self.locker_account.is_none());
        require!(
            proof.outcome_proof.outcome_with_id.outcome.executor_id
                == self.locker_account.as_ref().unwrap().to_string(),
            "Untrusted proof, burn receipt proof required"
        );
        let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
        let parts: Vec<&str> = std::str::from_utf8(&event_log)
            .unwrap()
            .split(":")
            .collect();
        require!(
            parts.len() == 4 && parts[0] == CALIMERO_EVENT,
            "Untrusted proof, burn receipt proof required"
        );
        let destination_contract = parts[1];
        let destination_receiver_account = parts[2];
        let amount = U128(parts[3].parse::<u128>().unwrap());

        let ft_token_contract_account: AccountId = self
            .contracts_mapping
            .get(&destination_contract.parse().unwrap())
            .unwrap();
        let ft_token_receiver_account: String = format!(
            "{}{}",
            destination_receiver_account
                .strip_suffix(&self.destination_master_account.to_string())
                .unwrap_or(&destination_receiver_account),
            self.source_master_account
        );

        let promise_prover = env::promise_create(
            self.prover_account.clone(),
            "prove_outcome",
            &serde_json::to_vec(&(proof.clone(), height)).unwrap(),
            NO_DEPOSIT,
            VERIFY_LOG_ENTRY_GAS,
        );

        let promise_result = env::promise_then(
            promise_prover,
            env::current_account_id(),
            "finish_deposit",
            &serde_json::to_vec(&(
                ft_token_contract_account,
                ft_token_receiver_account,
                amount,
                proof,
            ))
            .unwrap(),
            env::attached_deposit(),
            FINISH_DEPOSIT_GAS + MINT_GAS + FT_TRANSFER_CALL_GAS,
        );

        env::promise_return(promise_result)
    }

    /// Finish depositing once the proof was successfully validated. Can only be called by the contract
    /// itself.
    #[payable]
    pub fn finish_deposit(
        &mut self,
        ft_token_contract_account: AccountId,
        ft_token_receiver_account: AccountId,
        amount: U128,
        proof: FullOutcomeProof,
    ) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<Vec<bool>>(&x).unwrap()[0],
            _ => env::panic_str("Prover failed"),
        };
        require!(verification_success, "Failed to verify the proof");

        let required_deposit = self.record_proof(&proof);

        require!(
            env::attached_deposit() >= required_deposit,
            "Deposit too low"
        );

        let memo = String::from(format!(
            "Transfer from {}",
            self.locker_account.as_ref().unwrap().to_string()
        ));

        env::promise_return(env::promise_create(
            ft_token_contract_account,
            "ft_transfer",
            &serde_json::to_vec(&(ft_token_receiver_account, amount, Some(memo))).unwrap(),
            near_sdk::ONE_YOCTO,
            MINT_GAS,
        ))
    }

    /// Record proof to make sure it is not re-used later for another deposit.
    fn record_proof(&mut self, proof: &FullOutcomeProof) -> Balance {
        near_sdk::assert_self();
        let initial_storage = env::storage_usage();

        let proof_key = proof.block_header_lite.hash();
        require!(
            !self.used_events.contains(&proof_key),
            "Event cannot be reused for depositing."
        );
        self.used_events.insert(&proof_key);
        let current_storage = env::storage_usage();
        let required_deposit =
            Balance::from(current_storage - initial_storage) * env::storage_byte_cost();

        env::log_str(&format!("RecordProof:{}", hashes::encode_hex(&proof_key)));
        required_deposit
    }
}

admin_controlled::impl_admin_controlled!(FungibleTokenConnector, paused);
