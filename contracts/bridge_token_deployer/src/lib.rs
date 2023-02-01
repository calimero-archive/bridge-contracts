use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, near_bindgen, require, serde_json, AccountId, Balance, Gas, PanicOnDefault, PromiseResult,
};
use substring::Substring;
use utils::hashes;

const BRIDGE_TOKEN_BINARY: &[u8] = include_bytes!(std::env!(
    "BRIDGE_TOKEN",
    "Set BRIDGE_TOKEN to be the path of the bridge token binary"
));

const NO_DEPOSIT: Balance = 0;

/// Initial balance for the BridgeToken contract to cover storage and related.
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 30_000_000_000_000_000_000_000_000; // 30e24yN, 30N

/// Gas to initialize BridgeToken contract.
const BRIDGE_TOKEN_NEW: Gas = Gas(50_000_000_000_000);
const BRIDGE_TOKEN_COMPLETE: Gas = Gas(20_000_000_000_000);

const MAX_ACCOUNT_ID_LENGTH: usize = 64;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct BridgeTokenDeployer {
    /// The account of the bridge that will deploy
    bridge_account: AccountId,
    /// NEAR master account on source network, ex. 'testnet'
    /// Used to have nicer names in bridged tokens by stripping suffix from token name.
    /// ex. wrap.testnet mapped to wrap.ft_bridged.shard.calimero.testnet
    /// instead of wrap_testnet.ft_bridged.shard.calimero.testnet.
    ///
    /// If token is bridged that has different master account it will still be bridged normally
    /// but will have different name.
    /// ex. wrap.aurora mapped to wrap_aurora.ft_bridged.shard.calimero.testnet
    source_master_account: AccountId,
}

#[near_bindgen]
impl BridgeTokenDeployer {
    /// Initializes the contract.
    /// `bridge_account`: NEAR account that will initiate deploy of token
    /// `source_master_account`: NEAR master account on source network, ex. 'testnet'
    #[init]
    pub fn new(bridge_account: AccountId, source_master_account: AccountId) -> Self {
        require!(!env::state_exists(), "Already initialized");
        Self {
            bridge_account,
            source_master_account,
        }
    }

    #[payable]
    pub fn deploy_bridge_token(&mut self, source_address: String) {
        require!(
            env::predecessor_account_id() == self.bridge_account,
            "Deploy bridge token can be called with and only with a corresponding bridge account"
        );

        let mut bridge_token_account_name = format!(
            "{}.{}",
            source_address
                .strip_suffix(&format!(".{}", self.source_master_account))
                .unwrap_or(&source_address)
                .replace('.', "_"),
            env::current_account_id()
        );

        if bridge_token_account_name.len() > MAX_ACCOUNT_ID_LENGTH {
            let suffix_length = env::current_account_id().as_str().len() + 1;
            let name_sha = hashes::encode_hex(&env::sha256(bridge_token_account_name.as_bytes()));
            let bridge_token_account_prefix =
                name_sha.substring(0, MAX_ACCOUNT_ID_LENGTH - suffix_length);
            bridge_token_account_name = format!(
                "{}.{}",
                bridge_token_account_prefix,
                env::current_account_id()
            );
        };

        let bridge_token_account_id = AccountId::new_unchecked(bridge_token_account_name);

        let deploy_bridge_token_batch_promise = env::promise_batch_create(&bridge_token_account_id);
        env::promise_batch_action_create_account(deploy_bridge_token_batch_promise);
        env::promise_batch_action_transfer(
            deploy_bridge_token_batch_promise,
            BRIDGE_TOKEN_INIT_BALANCE,
        );
        env::promise_batch_action_add_key_with_full_access(
            deploy_bridge_token_batch_promise,
            &env::signer_account_pk(),
            0,
        );
        env::promise_batch_action_deploy_contract(
            deploy_bridge_token_batch_promise,
            BRIDGE_TOKEN_BINARY,
        );
        env::promise_batch_action_function_call(
            deploy_bridge_token_batch_promise,
            "new",
            &serde_json::to_vec(&(env::predecessor_account_id(),)).unwrap(),
            NO_DEPOSIT,
            BRIDGE_TOKEN_NEW,
        );
        env::promise_return(env::promise_then(
            deploy_bridge_token_batch_promise,
            env::current_account_id(),
            "complete_deployment",
            &serde_json::to_vec(&(bridge_token_account_id,)).unwrap(),
            NO_DEPOSIT,
            BRIDGE_TOKEN_COMPLETE,
        ));
    }

    #[private]
    pub fn complete_deployment(&mut self, bridge_token_address: AccountId) {
        require!(
            env::promise_results_count() == 1,
            "One and only one result was expected"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(_) => (),
            _ => env::panic_str("Bridge token deployer deployment failed"),
        };

        env::value_return(&serde_json::to_vec(&(bridge_token_address,)).unwrap());
    }
}
