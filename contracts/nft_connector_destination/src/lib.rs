use admin_controlled::Mask;
use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::serde_json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault, PromiseResult,
};
use types::FullOutcomeProof;
use utils::{hashes, Hash, Hashable};

use near_sdk::PublicKey;

const BRIDGE_TOKEN_BINARY: &'static [u8] = include_bytes!(std::env!(
    "BRIDGE_TOKEN",
    "Set BRIDGE_TOKEN to be the path of the bridge token binary"
));

const NO_DEPOSIT: Balance = 0;

/// Initial balance for the BridgeToken contract to cover storage and related.
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 50_000_000_000_000_000_000_000_000; // 50e24yN, 50N

/// Gas to initialize BridgeToken contract.
const BRIDGE_TOKEN_NEW: Gas = Gas(50_000_000_000_000);

/// Gas to call mint method on bridge token.
const MINT_GAS: Gas = Gas(30_000_000_000_000);

/// Gas to call finish deposit method.
/// This doesn't cover the gas required for calling mint method.
const FINISH_DEPOSIT_GAS: Gas = Gas(230_000_000_000_000);

/// Gas to call prove_outcome on prover.
const PROVE_OUTCOME_GAS: Gas = Gas(40_000_000_000_000);

const PAUSE_DEPLOY_TOKEN: Mask = 1 << 0;
const PAUSE_DEPOSIT: Mask = 1 << 1;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct NonFungibleTokenConnector {
    /// The account of the prover that we can use to prove
    pub prover_account: AccountId,
    source_master_account: AccountId,
    destination_master_account: AccountId,
    /// The account of the locker on other network that is used to lock NFT
    pub locker_account: Option<AccountId>,
    /// Hashes of the events that were already used.
    pub used_events: UnorderedSet<Hash>,
    /// Public key of the account deploying connector.
    pub owner_pk: PublicKey,
    /// Mappings between NFT contract on main network and NFT contract on this network
    contracts_mapping: UnorderedMap<AccountId, AccountId>,
    /// All NFT contracts that were deployed by this account
    all_contracts: UnorderedSet<AccountId>,
    /// Mask determining all paused functions
    paused: Mask,
}

#[near_bindgen]
impl NonFungibleTokenConnector {
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
            all_contracts: UnorderedSet::new(b"a".to_vec()),
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

    pub fn view_mapping(&self, source_account: AccountId) -> Option<AccountId> {
        self.contracts_mapping.get(&source_account)
    }

    pub fn burn(&mut self, burner_id: AccountId, token_id: String) {
        require!(
            self.all_contracts.contains(&env::predecessor_account_id()),
            "Untrusted burn"
        );
        env::log_str(&format!(
            "CALIMERO_EVENT_BURN_NFT:{}:{}:{}",
            env::predecessor_account_id(),
            burner_id,
            base64::encode(token_id),
        ));
    }

    /// Used when receiving NFT from other network
    #[payable]
    pub fn mint(&mut self, proof: FullOutcomeProof, height: u64) {
        self.assert_not_paused(PAUSE_DEPOSIT);
        require!(!self.locker_account.is_none());
        require!(
            proof.outcome_proof.outcome_with_id.outcome.executor_id
                == self.locker_account.as_ref().unwrap().to_string(),
            "Untrusted prover account, lock receipt proof required"
        );
        let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
        let parts: Vec<&str> = std::str::from_utf8(&event_log)
            .unwrap()
            .split(":")
            .collect();
        require!(
            parts.len() == 5 && parts[0] == "CALIMERO_EVENT_LOCK_NFT",
            "Untrusted proof, lock receipt proof required"
        );
        let nft_token_contract_account = parts[1];
        let source_receiver_account = parts[2];
        let token_id = std::str::from_utf8(&base64::decode(parts[3]).unwrap())
            .unwrap()
            .to_owned();
        let token_metadata: TokenMetadata =
            serde_json::from_str(std::str::from_utf8(&base64::decode(parts[4]).unwrap()).unwrap())
                .unwrap();
        let nft_token_receiver_account = format!(
            "{}{}",
            source_receiver_account
                .strip_suffix(&self.source_master_account.to_string())
                .unwrap_or(&source_receiver_account),
            self.destination_master_account
        );

        let promise_prover = env::promise_create(
            self.prover_account.clone(),
            "prove_outcome",
            &serde_json::to_vec(&(proof.clone(), height)).unwrap(),
            NO_DEPOSIT,
            PROVE_OUTCOME_GAS,
        );

        let promise_result = env::promise_then(
            promise_prover,
            env::current_account_id(),
            "finish_mint",
            &serde_json::to_vec(&(
                nft_token_contract_account,
                nft_token_receiver_account,
                token_id,
                token_metadata,
                proof,
            ))
            .unwrap(),
            env::attached_deposit(),
            FINISH_DEPOSIT_GAS,
        );

        env::promise_return(promise_result)
    }

    /// Finish depositing once the proof was successfully validated. Can only be called by the contract
    /// itself.
    #[payable]
    pub fn finish_mint(
        &mut self,
        nft_token_contract_account: String,
        nft_token_receiver_account: String,
        token_id: String,
        token_metadata: TokenMetadata,
        proof: FullOutcomeProof,
    ) {
        near_sdk::assert_self();
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<Vec<bool>>(&x).unwrap()[0],
            _ => env::panic_str("Prover failed"),
        };
        require!(verification_success, "Failed to verify the proof");

        self.record_proof(&proof);
        let transfer_promise = if let Some(nft_contract) = self
            .contracts_mapping
            .get(&nft_token_contract_account.parse().unwrap())
        {
            let params = serde_json::json!({ "account_id": nft_token_receiver_account, "token_id": token_id, "token_metadata": token_metadata });
            env::promise_create(
                nft_contract,
                "mint",
                &serde_json::to_vec(&params).unwrap(),
                near_sdk::ONE_NEAR,
                MINT_GAS,
            )
        } else {
            env::panic_str("No Bridge Token deployed")
        };

        env::promise_return(transfer_promise)
    }

    /// Record proof to make sure it is not re-used later for another deposit.
    fn record_proof(&mut self, proof: &FullOutcomeProof) -> Balance {
        near_sdk::assert_self();
        let initial_storage = env::storage_usage();

        let proof_key = proof.block_header_lite.hash();
        require!(
            !self.used_events.contains(&proof_key),
            "Event cannot be reused for minting."
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

    #[payable]
    pub fn deploy_bridge_token(&mut self, source_address: String) {
        near_sdk::assert_self();
        self.assert_not_paused(PAUSE_DEPLOY_TOKEN);

        let bridge_token_account_id = AccountId::new_unchecked(format!(
            "{}.{}",
            source_address
                .strip_suffix(&format!(".{}", self.source_master_account.to_string()))
                .unwrap_or(&source_address)
                .replace(".", "_"),
            env::current_account_id()
        ));
        let initial_storage = env::storage_usage();
        self.contracts_mapping
            .insert(&source_address.parse().unwrap(), &bridge_token_account_id);
        self.all_contracts.insert(&bridge_token_account_id);
        let required_deposit = Balance::from(initial_storage - initial_storage)
            * env::storage_byte_cost()
            + BRIDGE_TOKEN_INIT_BALANCE;
        require!(
            env::attached_deposit() >= required_deposit,
            "Deposit too low"
        );

        env::log_str(&format!(
            "CALIMERO_EVENT_DEPLOY_NFT:{}:{}",
            source_address, bridge_token_account_id
        ));

        let promise = env::promise_batch_create(&bridge_token_account_id);
        env::promise_batch_action_create_account(promise);
        env::promise_batch_action_transfer(promise, BRIDGE_TOKEN_INIT_BALANCE);
        env::promise_batch_action_add_key_with_full_access(promise, &self.owner_pk.clone(), 0);
        env::promise_batch_action_deploy_contract(promise, &BRIDGE_TOKEN_BINARY.to_vec());
        env::promise_batch_action_function_call(
            promise,
            "new",
            &vec![],
            NO_DEPOSIT,
            BRIDGE_TOKEN_NEW,
        );

        env::value_return(&serde_json::to_vec(&(bridge_token_account_id,)).unwrap());
    }
}

admin_controlled::impl_admin_controlled!(NonFungibleTokenConnector, paused);
