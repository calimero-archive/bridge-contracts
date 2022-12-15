use admin_controlled::Mask;
use connector_base::{
    DeployerAware, OtherNetworkAware, OtherNetworkTokenAware, TokenMint, TokenUnlock,
};
use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, LookupSet};
use near_sdk::serde_json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Balance, Gas, PanicOnDefault, PromiseResult,
};
use types::{ConnectorType, FullOutcomeProof};
use utils::{hashes, Hash};

use near_sdk::PublicKey;

const NO_DEPOSIT: Balance = 0;

/// Gas to initialize BridgeToken contract.
const BRIDGE_TOKEN_NEW: Gas = Gas(80_000_000_000_000);
const BRIDGE_TOKEN_COMPLETE: Gas = Gas(20_000_000_000_000);

/// Initial balance for the BridgeToken contract to cover storage and related.
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 50_000_000_000_000_000_000_000_000; // 50e24yN, 50N

/// Gas to call mint method on bridge token.
const MINT_GAS: Gas = Gas(30_000_000_000_000);

/// Gas to call finish deposit method.
/// This doesn't cover the gas required for calling mint method.
const FINISH_DEPOSIT_GAS: Gas = Gas(230_000_000_000_000);

/// Gas to call prove_outcome on prover.
const PROVE_OUTCOME_GAS: Gas = Gas(40_000_000_000_000);

/// Gas for deploying bridge token contract
const DEPLOY_GAS: Gas = Gas(180_000_000_000_000);

/// Gas to call nft_transfer_call when the target of deposit is a contract
const TRANSFER_CALL_GAS: Gas = Gas(80_000_000_000_000);

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = Gas(50_000_000_000_000);

/// Gas to call finish unlock method.
const FINISH_UNLOCK_GAS: Gas = Gas(30_000_000_000_000);

/// Gas to call can_bridge on permissions manager
const PERMISSIONS_OUTCOME_GAS: Gas = Gas(40_000_000_000_000);

pub const PAUSE_DEPLOY_TOKEN: Mask = 1 << 0;
pub const PAUSE_MINT: Mask = 1 << 1;
pub const PAUSE_LOCK: Mask = 1 << 2;

const CALIMERO_EVENT_DEPLOY_NFT: &str = "CALIMERO_EVENT_DEPLOY_NFT";
const CALIMERO_EVENT_BURN_NFT: &str = "CALIMERO_EVENT_BURN_NFT";
const CALIMERO_EVENT_LOCK_NFT: &str = "CALIMERO_EVENT_LOCK_NFT";

connector_base::impl_deployer_aware!(NonFungibleTokenConnector, CALIMERO_EVENT_DEPLOY_NFT);
connector_base::impl_other_network_aware!(NonFungibleTokenConnector);
connector_base::impl_other_network_token_aware!(
    NonFungibleTokenConnector,
    CALIMERO_EVENT_DEPLOY_NFT
);
connector_base::impl_token_mint!(NonFungibleTokenConnector);
connector_base::impl_token_unlock!(
    NonFungibleTokenConnector,
    CALIMERO_EVENT_BURN_NFT,
    TokenId,
    "nft_transfer"
);

#[near_bindgen]
impl NonFungibleTokenConnector {
    /// Used when sending NFT to other network
    /// `msg` is expected to contain valid other network id
    pub fn nft_on_transfer(
        &mut self,
        sender_id: String,
        previous_owner_id: String,
        token_id: String,
        msg: String,
    ) {
        let promise_nft_token = env::promise_create(
            env::predecessor_account_id(),
            "nft_token",
            &serde_json::to_vec(&serde_json::json!({ "token_id": token_id })).unwrap(),
            NO_DEPOSIT,
            env::prepaid_gas() / 3,
        );

        self.assert_not_paused(PAUSE_LOCK);

        env::promise_return(env::promise_then(
            promise_nft_token,
            env::current_account_id(),
            "lock",
            &serde_json::to_vec(&(
                env::predecessor_account_id(),
                sender_id,
                previous_owner_id,
                token_id,
                msg,
            ))
            .unwrap(),
            NO_DEPOSIT,
            env::prepaid_gas() / 3,
        ));
    }

    #[private]
    pub fn lock(
        &mut self,
        token_account: String,
        #[allow(unused_variables)] sender_id: String,
        previous_owner_id: String,
        token_id: String,
        #[allow(unused_variables)] msg: String,
    ) {
        require!(env::promise_results_count() == 1);

        let promise_result = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<Option<Token>>(&x).unwrap(),
            _ => None,
        };

        let metadata = if let Some(token_data) = promise_result {
            token_data.metadata
        } else {
            // TODO check this and handle none from the above
            Some(TokenMetadata {
                title: None,
                description: None,
                media: None,
                media_hash: None,
                copies: None,
                issued_at: None,
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: None,
                reference: None,
                reference_hash: None,
            })
        };

        let permission_promise = env::promise_create(
            self.connector_permissions_account.clone(),
            "can_bridge",
            &serde_json::to_vec(&(&sender_id, ConnectorType::NFT)).unwrap(),
            NO_DEPOSIT,
            PERMISSIONS_OUTCOME_GAS,
        );

        env::promise_return(env::promise_then(
            permission_promise,
            env::current_account_id(),
            "lock_with_metadata",
            &serde_json::to_vec(&(token_account, previous_owner_id, token_id, metadata)).unwrap(),
            NO_DEPOSIT,
            env::prepaid_gas() / 3,
        ));
    }

    #[private]
    pub fn lock_with_metadata(
        &mut self,
        token_account: String,
        previous_owner_id: String,
        token_id: String,
        metadata: Option<TokenMetadata>,
    ) {
        require!(env::promise_results_count() == 1);

        let can_bridge_promise_result = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<bool>(&x).unwrap(),
            _ => false,
        };

        if can_bridge_promise_result {
            env::log_str(&format!(
                "{}:{}:{}:{}:{}",
                CALIMERO_EVENT_LOCK_NFT,
                token_account,
                previous_owner_id,
                base64::encode(token_id),
                base64::encode(serde_json::to_string(&metadata.unwrap()).unwrap()),
            ));
            env::value_return(&serde_json::to_vec(&false).unwrap());
        } else {
            env::value_return(&serde_json::to_vec(&true).unwrap());
        }
    }

    fn transform_transferable(token_id: String) -> String {
        base64::encode(token_id)
    }

    fn parse_transferable(encoded_token_id: String) -> String {
        std::str::from_utf8(&base64::decode(encoded_token_id).unwrap())
            .unwrap()
            .to_owned()
    }

    fn verify_mint_params(params: Vec<String>) {
        require!(
            params.len() == 5 && params[0] == CALIMERO_EVENT_LOCK_NFT,
            "Untrusted proof, lock receipt proof required"
        );
    }

    fn token_mint_params(params: Vec<String>) -> near_sdk::serde_json::Value {
        let nft_token_receiver_account = params[2].clone();
        let token_id = std::str::from_utf8(&base64::decode(params[3].clone()).unwrap())
            .unwrap()
            .to_owned();
        let token_metadata: TokenMetadata = serde_json::from_str(
            std::str::from_utf8(&base64::decode(params[4].clone()).unwrap()).unwrap(),
        )
        .unwrap();
        serde_json::json!({ "account_id": nft_token_receiver_account, "token_id": token_id, "token_metadata": token_metadata })
    }

    fn token_unlock_params(
        receiver: AccountId,
        transferable: TokenId,
        memo: String,
    ) -> (AccountId, TokenId, Option<u64>, Option<String>) {
        (receiver, transferable, None, Some(memo))
    }
}

admin_controlled::impl_admin_controlled!(NonFungibleTokenConnector, paused);
