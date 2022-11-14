use admin_controlled::Mask;
use connector_base::OtherNetworkAware;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupSet;
use near_sdk::serde_json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault, PromiseResult,
};

use types::FullOutcomeProof;
use utils::{hashes, Hash, Hashable};

const NO_DEPOSIT: Balance = 0;

/// Gas to use for cross_call_execute on self
const CALL_GAS: Gas = Gas(20_000_000_000_000);

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = Gas(50_000_000_000_000);

/// Gas to call can_bridge on permissions manager
const PERMISSIONS_OUTCOME_GAS: Gas = Gas(40_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CrossShardConnector {
    /// The account of the prover that we can use to prove
    pub prover_account: AccountId,
    /// The contract account which can deny certain accounts from initiating a bridge action
    pub connector_permissions_account: AccountId,
    /// The account of the locker on other network that is used to burn NFT
    pub locker_account: Option<AccountId>,
    /// Hashes of the events that were already used.
    pub used_events: LookupSet<Hash>,
    /// Mask determining all paused functions
    paused: Mask,
    /// duration in nanoseconds for which proof is considered valid
    /// not used if not provided
    proof_validity_ns: Option<u64>,
}

connector_base::impl_other_network_aware!(CrossShardConnector);

#[near_bindgen]
impl CrossShardConnector {
    /// Initializes the contract.
    /// `prover_account`: NEAR account of the Near Prover contract;
    #[init]
    pub fn new(
        prover_account: AccountId,
        connector_permissions_account: AccountId,
        proof_validity_ns: Option<u64>,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        Self {
            prover_account,
            connector_permissions_account,
            used_events: LookupSet::new(b"u".to_vec()),
            locker_account: None,
            paused: Mask::default(),
            proof_validity_ns,
        }
    }

    /// Used when initiating call on other network
    /// `destination_contract_args` is expected to be serialized json
    pub fn cross_call(
        &mut self,
        destination_contract_id: String,
        destination_contract_method: String,
        destination_contract_args: String,
        destination_gas: Gas,
        destination_deposit: Balance,
        source_callback_method: String,
    ) {
        let permission_promise = env::promise_create(
            self.connector_permissions_account.clone(),
            "can_make_cross_shard_call_for_contract",
            &serde_json::to_vec(&(env::predecessor_account_id(), &destination_contract_id))
                .unwrap(),
            NO_DEPOSIT,
            PERMISSIONS_OUTCOME_GAS,
        );

        env::promise_return(env::promise_then(
            permission_promise,
            env::current_account_id(),
            "cross_call_resolve",
            &serde_json::to_vec(&(
                destination_contract_id,
                destination_contract_method,
                destination_contract_args,
                destination_gas,
                destination_deposit,
                source_callback_method,
                env::predecessor_account_id().to_string(),
            ))
            .unwrap(),
            NO_DEPOSIT,
            PERMISSIONS_OUTCOME_GAS,
        ));
    }

    pub fn cross_call_resolve(
        &mut self,
        destination_contract_id: String,
        destination_contract_method: String,
        destination_contract_args: String,
        destination_gas: Gas,
        destination_deposit: Balance,
        source_callback_method: String,
        cross_call_initiator_account_id: String,
    ) -> bool {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<bool>(&x).unwrap(),
            _ => false,
        };

        if verification_success {
            env::log_str(&format!(
                "CALIMERO_EVENT_CROSS_CALL:{}:{}:{}:{}:{}:{}:{}",
                destination_contract_id,
                destination_contract_method,
                base64::encode(destination_contract_args),
                destination_gas.0,
                destination_deposit,
                cross_call_initiator_account_id,
                source_callback_method,
            ));
        }

        verification_success
    }

    #[payable]
    pub fn cross_call_execute(&mut self, proof: FullOutcomeProof, height: u64) {
        require!(self.locker_account.is_some());
        require!(
            proof.outcome_proof.outcome_with_id.outcome.executor_id
                == self.locker_account.as_ref().unwrap().to_string(),
            "Untrusted prover account, cross_call receipt proof required"
        );
        let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
        let parts: Vec<&str> = std::str::from_utf8(&event_log)
            .unwrap()
            .split(':')
            .collect();
        require!(
            parts.len() == 8 && parts[0] == "CALIMERO_EVENT_CROSS_CALL",
            "Untrusted proof, cross_call receipt proof required"
        );
        let destination_contract = parts[1];
        let destination_contract_method = parts[2];
        let destination_contract_args = base64::decode(parts[3]).unwrap();
        let destination_gas = Gas(parts[4].parse::<u64>().unwrap());
        let destination_deposit = parts[5].parse::<u128>().unwrap();

        let source_contract = parts[6];
        let source_contract_method = parts[7];

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
            "finish_cross_call_execute",
            &serde_json::to_vec(&(
                env::predecessor_account_id(),
                destination_contract,
                destination_contract_method,
                destination_contract_args,
                destination_gas,
                destination_deposit,
                source_contract,
                source_contract_method,
                proof,
            ))
            .unwrap(),
            env::attached_deposit(),
            env::prepaid_gas() - VERIFY_LOG_ENTRY_GAS - CALL_GAS,
        );

        env::promise_return(promise_result)
    }

    #[payable]
    pub fn finish_cross_call_execute(
        &mut self,
        caller_id: AccountId,
        destination_contract: AccountId,
        destination_contract_method: String,
        destination_contract_args: Vec<u8>,
        destination_gas: Gas,
        destination_deposit: Balance,
        source_contract: AccountId,
        source_contract_method: String,
        proof: FullOutcomeProof,
    ) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<bool>(&x).unwrap(),
            _ => env::panic_str("Prover failed"),
        };
        require!(verification_success, "Failed to verify the proof");

        let remaining_deposit = self.record_proof(&proof);

        let refund_promise = env::promise_batch_create(&caller_id);
        env::promise_batch_action_transfer(refund_promise, remaining_deposit);

        let execution_promise = env::promise_then(
            refund_promise,
            destination_contract.clone(),
            &destination_contract_method,
            &destination_contract_args,
            destination_deposit,
            destination_gas,
        );

        let calimero_response_promise = env::promise_then(
            execution_promise,
            env::current_account_id(),
            "calimero_response",
            &serde_json::to_vec(&(
                source_contract,
                source_contract_method,
                destination_contract,
            ))
            .unwrap(),
            NO_DEPOSIT,
            CALL_GAS,
        );

        env::promise_return(calimero_response_promise);
    }

    pub fn calimero_response(
        &mut self,
        source_contract: AccountId,
        source_contract_method: String,
        destination_contract: AccountId,
    ) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let execution_result = match env::promise_result(0) {
            PromiseResult::Successful(x) => base64::encode(x),
            _ => "FAILED!".to_string(),
        };

        env::log_str(&format!(
            "CALIMERO_EVENT_CROSS_RESPONSE:{}:{}:{}:{}",
            source_contract, source_contract_method, execution_result, destination_contract
        ));
    }

    #[payable]
    pub fn cross_call_receive_response(&mut self, proof: FullOutcomeProof, height: u64) {
        require!(self.locker_account.is_some());
        require!(
            proof.outcome_proof.outcome_with_id.outcome.executor_id
                == self.locker_account.as_ref().unwrap().to_string(),
            "Untrusted prover account, calimero_response receipt proof required"
        );
        let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
        let parts: Vec<&str> = std::str::from_utf8(&event_log)
            .unwrap()
            .split(':')
            .collect();
        require!(
            parts.len() == 5 && parts[0] == "CALIMERO_EVENT_CROSS_RESPONSE",
            "Untrusted proof, calimero_response receipt proof required"
        );
        let source_contract = parts[1];
        let source_contract_method = parts[2];
        let response = parts[3];
        let destination_predecessor = parts[4];

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
            "finish_cross_response",
            &serde_json::to_vec(&(
                env::predecessor_account_id(),
                source_contract,
                source_contract_method,
                destination_predecessor,
                response,
                proof,
            ))
            .unwrap(),
            env::attached_deposit(),
            env::prepaid_gas() - VERIFY_LOG_ENTRY_GAS - CALL_GAS,
        );

        env::promise_return(promise_result)
    }

    #[payable]
    pub fn finish_cross_response(
        &mut self,
        caller_id: AccountId,
        source_contract: AccountId,
        source_contract_method: String,
        destination_predecessor: AccountId,
        response: String,
        proof: FullOutcomeProof,
    ) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<bool>(&x).unwrap(),
            _ => env::panic_str("Prover failed"),
        };
        require!(verification_success, "Failed to verify the proof");

        let remaining_deposit = self.record_proof(&proof);

        let refund_promise = env::promise_batch_create(&caller_id);
        env::promise_batch_action_transfer(refund_promise, remaining_deposit);

        let args = if response == "FAILED!" {
            None
        } else {
            Some(base64::decode(response).unwrap())
        };

        env::promise_return(env::promise_then(
            refund_promise,
            source_contract,
            &source_contract_method,
            &serde_json::to_vec(&serde_json::json!({ "response": args, "calimero_predecessor_id": destination_predecessor })).unwrap(),
            NO_DEPOSIT,
            CALL_GAS,
        ))
    }
}

admin_controlled::impl_admin_controlled!(CrossShardConnector, paused);
