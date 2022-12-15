use admin_controlled::Mask;
use connector_base::{
    DeployerAware, OtherNetworkAware, OtherNetworkTokenAware, TokenMint, TokenUnlock,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, LookupSet};
use near_sdk::json_types::U128;
use near_sdk::serde_json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault, PromiseResult,
};
use types::{ConnectorType, FullOutcomeProof};
use utils::{hashes, Hash};

use near_sdk::PublicKey;

const NO_DEPOSIT: Balance = 0;

/// Initial balance for the BridgeToken contract to cover storage and related.
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 20_000_000_000_000_000_000_000_000; // 20e24yN, 20N

/// Gas to initialize BridgeToken contract.
const BRIDGE_TOKEN_NEW: Gas = Gas(80_000_000_000_000);
const BRIDGE_TOKEN_COMPLETE: Gas = Gas(20_000_000_000_000);

/// Gas to call mint method on bridge token.
const MINT_GAS: Gas = Gas(30_000_000_000_000);

/// Gas to call ft_transfer_call when the target of deposit is a contract
const TRANSFER_CALL_GAS: Gas = Gas(80_000_000_000_000);

/// Gas for deploying bridge token contract
const DEPLOY_GAS: Gas = Gas(180_000_000_000_000);

/// Gas to call finish deposit method.
/// This doesn't cover the gas required for calling mint method.
const FINISH_DEPOSIT_GAS: Gas = Gas(230_000_000_000_000);

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = Gas(50_000_000_000_000);

/// Gas to call register method on FT.
const REGISTER_FT_GAS: Gas = Gas(50_000_000_000_000);

/// Gas to call finish unlock method.
const FINISH_UNLOCK_GAS: Gas = Gas(30_000_000_000_000);

/// Gas to call prove_outcome on prover.
const PROVE_OUTCOME_GAS: Gas = Gas(40_000_000_000_000);

/// Gas to call can_bridge on permissions manager
const PERMISSIONS_OUTCOME_GAS: Gas = Gas(40_000_000_000_000);

pub const PAUSE_DEPLOY_TOKEN: Mask = 1 << 0;
pub const PAUSE_MINT: Mask = 1 << 1;
pub const PAUSE_LOCK: Mask = 1 << 2;

const CALIMERO_EVENT_DEPLOY_FT: &str = "CALIMERO_EVENT_DEPLOY_FT";
const CALIMERO_EVENT_BURN_FT: &str = "CALIMERO_EVENT_BURN_FT";
const CALIMERO_EVENT_LOCK_FT: &str = "CALIMERO_EVENT_LOCK_FT";

connector_base::impl_deployer_aware!(FungibleTokenConnector, CALIMERO_EVENT_DEPLOY_FT);
connector_base::impl_other_network_aware!(FungibleTokenConnector);
connector_base::impl_other_network_token_aware!(FungibleTokenConnector, CALIMERO_EVENT_DEPLOY_FT);
connector_base::impl_token_mint!(FungibleTokenConnector);
connector_base::impl_token_unlock!(
    FungibleTokenConnector,
    CALIMERO_EVENT_BURN_FT,
    U128,
    "ft_transfer"
);

#[near_bindgen]
impl FungibleTokenConnector {
    /// Used to register this connector to use an FT that requires prior registration
    /// ex. wrap.testnet
    #[payable]
    pub fn register_ft(&mut self, ft_address: AccountId) {
        env::promise_return(env::promise_create(
            ft_address,
            "storage_deposit",
            &Vec::<u8>::new(),
            env::attached_deposit(),
            REGISTER_FT_GAS,
        ))
    }

    /// Emits a calimero lock event if transfer is successful
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        #[allow(unused_variables)] msg: String,
    ) {
        let permission_promise = env::promise_create(
            self.connector_permissions_account.clone(),
            "can_bridge",
            &serde_json::to_vec(&(&sender_id, ConnectorType::FT)).unwrap(),
            NO_DEPOSIT,
            PERMISSIONS_OUTCOME_GAS,
        );

        self.assert_not_paused(PAUSE_LOCK);

        env::promise_return(env::promise_then(
            permission_promise,
            env::current_account_id(),
            "lock",
            &serde_json::to_vec(&(sender_id, env::predecessor_account_id(), amount)).unwrap(),
            NO_DEPOSIT,
            PERMISSIONS_OUTCOME_GAS,
        ));
    }

    #[private]
    pub fn lock(&mut self, sender_id: AccountId, ft_contract_id: AccountId, amount: U128) {
        require!(env::promise_results_count() == 1);

        let verification_success = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<bool>(&x).unwrap(),
            _ => false,
        };

        if verification_success {
            env::log_str(&format!(
                "{}:{}:{}:{}",
                CALIMERO_EVENT_LOCK_FT,
                ft_contract_id, 
                sender_id, 
                amount.0
            ));

            env::value_return(&serde_json::to_vec(&U128(0).0.to_string()).unwrap());
        } else {
            env::value_return(&serde_json::to_vec(&amount.0.to_string()).unwrap());
        }
    }

    fn transform_transferable(amount: U128) -> u128 {
        amount.0
    }

    fn parse_transferable(amount: String) -> U128 {
        U128(amount.parse::<u128>().unwrap())
    }

    fn verify_mint_params(params: Vec<String>) {
        require!(
            params.len() == 4 && params[0] == CALIMERO_EVENT_LOCK_FT,
            "Untrusted proof, lock receipt proof required"
        );
    }

    fn token_mint_params(params: Vec<String>) -> near_sdk::serde_json::Value {
        serde_json::json!({ "account_id": params[2], "amount": U128(params[3].parse::<u128>().unwrap())})
    }

    fn token_unlock_params(
        receiver: AccountId,
        transferable: U128,
        memo: String,
    ) -> (AccountId, U128, Option<String>) {
        (receiver, transferable, Some(memo))
    }
}

admin_controlled::impl_admin_controlled!(FungibleTokenConnector, paused);
