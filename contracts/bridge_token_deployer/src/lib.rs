use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, serde_json, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault, PromiseResult
};

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
    pub fn new(
        bridge_account: AccountId,
        source_master_account: AccountId,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        Self {
            bridge_account,
            source_master_account,
        }
    }

    #[payable]
    pub fn deploy_bridge_token(&mut self, source_address: String) {
        require!(env::predecessor_account_id() == self.bridge_account);

        let bridge_token_account_id = AccountId::new_unchecked(format!(
            "{}.{}",
            source_address
                .strip_suffix(&format!(".{}", self.source_master_account))
                .unwrap_or(&source_address)
                .replace('.', "_"),
            env::current_account_id()
        ));

        let promise = env::promise_batch_create(&bridge_token_account_id);
        env::promise_batch_action_create_account(promise);
        env::promise_batch_action_transfer(promise, BRIDGE_TOKEN_INIT_BALANCE);
        env::promise_batch_action_add_key_with_full_access(promise, &env::signer_account_pk(), 0);
        env::promise_batch_action_deploy_contract(promise, BRIDGE_TOKEN_BINARY);
        env::promise_batch_action_function_call(
            promise,
            "new",
            &serde_json::to_vec(&(env::predecessor_account_id(),)).unwrap(),
            NO_DEPOSIT,
            BRIDGE_TOKEN_NEW,
        );
        env::promise_return(env::promise_then(
            promise,
            env::current_account_id(),
            "complete_deployment",
            &serde_json::to_vec(&(bridge_token_account_id,)).unwrap(),
            NO_DEPOSIT,
            BRIDGE_TOKEN_COMPLETE
        ));
    }

    #[private]
    pub fn complete_deployment(&mut self, bridge_token_address: AccountId) {
        require!(env::promise_results_count() == 1);

        match env::promise_result(0) {
            PromiseResult::Successful(_) => (),
            _ => env::panic_str("Bridge token deployer deployment failed"),
        };

        env::value_return(&serde_json::to_vec(&(bridge_token_address,)).unwrap());
    }
}
