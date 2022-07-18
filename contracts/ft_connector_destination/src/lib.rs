use admin_controlled::Mask;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde_json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault, PromiseIndex,
    PromiseResult,
};

use types::{Action, FullOutcomeProof, Receipt, ReceiptType, TransactionStatus};
use utils::{hashes, Hash, Hashable};

use near_sdk::PublicKey;

const BRIDGE_TOKEN_BINARY: &'static [u8] = include_bytes!(std::env!(
    "BRIDGE_TOKEN",
    "Set BRIDGE_TOKEN to be the path of the bridge token binary"
));

const NO_DEPOSIT: Balance = 0;

/// Initial balance for the BridgeToken contract to cover storage and related.
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 20_000_000_000_000_000_000_000_000; // 20e24yN, 20N

/// Gas to initialize BridgeToken contract.
const BRIDGE_TOKEN_NEW: Gas = Gas(10_000_000_000_000);

/// Gas to call mint method on bridge token.
const MINT_GAS: Gas = Gas(10_000_000_000_000);

/// Gas to call ft_transfer_call when the target of deposit is a contract
const FT_TRANSFER_CALL_GAS: Gas = Gas(80_000_000_000_000);

/// Gas to call finish deposit method.
/// This doesn't cover the gas required for calling mint method.
const FINISH_DEPOSIT_GAS: Gas = Gas(30_000_000_000_000);

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = Gas(50_000_000_000_000);

const PAUSE_DEPLOY_TOKEN: Mask = 1 << 0;
const PAUSE_DEPOSIT: Mask = 1 << 1;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct FungibleTokenConnector {
    /// The account of the prover that we can use to prove
    pub prover_account: AccountId,
    /// The account of the locker on other network that is used to lock FT
    pub locker_account: Option<AccountId>,
    /// Hashes of the events that were already used.
    pub used_events: UnorderedSet<Hash>,
    /// Public key of the account deploying connector.
    pub owner_pk: PublicKey,
    /// Mappings between FT contract on main network and FT contract on this network
    contracts_mapping: UnorderedMap<AccountId, AccountId>,
    /// Mask determining all paused functions
    paused: Mask,
}

#[near_bindgen]
impl FungibleTokenConnector {
    /// Initializes the contract.
    /// `prover_account`: NEAR account of the Near Prover contract;
    #[init]
    pub fn new(prover_account: AccountId) -> Self {
        require!(!env::state_exists(), "Already initialized");
        Self {
            prover_account,
            used_events: UnorderedSet::new(b"u".to_vec()),
            contracts_mapping: UnorderedMap::new(b"c".to_vec()),
            locker_account: None,
            owner_pk: env::signer_account_pk(),
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

    /// Used when receiving FT from other network
    #[payable]
    pub fn mint(&mut self, transaction: TransactionStatus, proof: FullOutcomeProof, height: u64) {
        self.assert_not_paused(PAUSE_DEPOSIT);
        require!(!self.locker_account.is_none());
        // TODO figure out how to verify that received TransactionStatus is indeed correct
        let ft_on_transfer: Receipt = transaction
            .receipts
            .into_iter()
            .find(|r| {
                &r.receiver_id.parse::<AccountId>().unwrap()
                    == self.locker_account.as_ref().unwrap()
            })
            .unwrap();
        let receipt_actions = match ft_on_transfer.receipt {
            ReceiptType::Action { actions, .. } => actions,
            _ => panic!("Not correct function call"),
        };

        let action = match &receipt_actions[0] {
            Action::FunctionCall(function_call_action) => function_call_action,
            _ => panic!("Not correct function call"),
        };
        let ft_token_contract_account: String = transaction.transaction.receiver_id;
        let ft_token_receiver_account: String = transaction.transaction.signer_id; // TODO convert to correct id for this network
        let amount: U128 = Self::decode_on_transfer_args(action.args.clone());

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
        ft_token_contract_account: String,
        ft_token_receiver_account: String,
        amount: U128,
        proof: FullOutcomeProof,
    ) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<bool>(&x).unwrap(),
            _ => env::panic_str("Prover failed"),
        };
        require!(verification_success, "Failed to verify the proof");

        let rest_of_deposit = self.record_proof(&proof);
        let transfer_promise = if let Some(ft_contract) = self
            .contracts_mapping
            .get(&ft_token_contract_account.parse().unwrap())
        {
            env::promise_create(
                ft_contract,
                "mint",
                &serde_json::to_vec(&(ft_token_receiver_account, amount)).unwrap(),
                near_sdk::ONE_YOCTO,
                MINT_GAS,
            )
        } else {
            let deploy_promise = self.map_ft(ft_token_contract_account.clone(), rest_of_deposit);
            env::promise_batch_action_function_call(
                deploy_promise,
                "mint",
                &serde_json::to_vec(&(ft_token_receiver_account, amount)).unwrap(),
                near_sdk::ONE_YOCTO,
                MINT_GAS,
            );
            deploy_promise
        };

        env::promise_return(transfer_promise)
    }

    fn decode_on_transfer_args(args: Vec<u8>) -> U128 {
        let as_str = std::str::from_utf8(&args).unwrap();
        let json: serde_json::Value = serde_json::from_str(as_str).unwrap();
        let amount_str: u128 = json["amount"].as_str().unwrap().parse().unwrap();

        U128(amount_str)
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

        require!(
            env::attached_deposit() >= required_deposit,
            "Deposit too low"
        );
        env::attached_deposit() - required_deposit
    }

    fn deploy_bridge_token(&mut self, source_address: String) -> PromiseIndex {
        self.assert_not_paused(PAUSE_DEPLOY_TOKEN);

        let currently_mapped = self.contracts_mapping.len() + 1;
        let bridge_token_account_id = AccountId::new_unchecked(format!(
            "ft{}.{}",
            currently_mapped,
            env::current_account_id()
        ));
        self.contracts_mapping
            .insert(&source_address.parse().unwrap(), &bridge_token_account_id);

        let promise = env::promise_batch_create(&bridge_token_account_id);
        env::promise_batch_action_create_account(promise);
        env::promise_batch_action_transfer(promise, BRIDGE_TOKEN_INIT_BALANCE);
        env::promise_batch_action_add_key_with_full_access(promise, &self.owner_pk.clone(), 0);
        env::promise_batch_action_deploy_contract(promise, &BRIDGE_TOKEN_BINARY.to_vec());
        env::promise_batch_action_function_call(
            // TODO resolve tests, currently fails due to Promise being used 
            promise,
            "new",
            &vec![],
            NO_DEPOSIT,
            BRIDGE_TOKEN_NEW,
        );
        promise
    }

    /// Function deploys FT smart contract that will be wrapped representation of
    /// ```ft_token_contract_account``` from source network.
    /// That connection is then saved to be used for other mints of that particular token.
    /// Also checks if there is enough unused deposit to successfully deploy FT contract.
    fn map_ft(
        &mut self,
        ft_token_contract_account: String,
        rest_of_deposit: Balance,
    ) -> PromiseIndex {
        near_sdk::assert_self();
        let initial_storage = env::storage_usage();
        let deploy_promise = self.deploy_bridge_token(ft_token_contract_account);
        let current_storage = env::storage_usage();
        let required_deposit = Balance::from(current_storage - initial_storage)
            * env::storage_byte_cost()
            + BRIDGE_TOKEN_INIT_BALANCE;
        require!(rest_of_deposit >= required_deposit, "Deposit too low");
        deploy_promise
    }
}

admin_controlled::impl_admin_controlled!(FungibleTokenConnector, paused);
