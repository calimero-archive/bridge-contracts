use admin_controlled::Mask;
use near_contract_standards::fungible_token::events::FtMint;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, AccountId, Gas, PanicOnDefault, Promise,
    PromiseOrValue, StorageUsage,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct BridgeToken {
    controller: AccountId,
    token: FungibleToken,
    name: String,
    symbol: String,
    reference: String,
    reference_hash: Base64VecU8,
    decimals: u8,
    paused: Mask,
    icon: Option<String>,
}

#[ext_contract(ext_connector)]
trait ExtConnector {
    fn burn(&self, burner_id: AccountId, transferable: U128);
}

/// Gas to call burn method on controller.
const BURN_GAS: Gas = Gas(30_000_000_000_000);

const PAUSE_WITHDRAW: Mask = 1 << 0;

#[near_bindgen]
impl BridgeToken {
    #[init]
    pub fn new(controller: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            controller,
            token: FungibleToken::new(b"t".to_vec()),
            name: String::default(),
            symbol: String::default(),
            reference: String::default(),
            reference_hash: Base64VecU8(vec![]),
            decimals: 0,
            paused: Mask::default(),
            icon: None,
        }
    }

    // TODO see if this needs to be secured to only call it once
    pub fn set_metadata(
        &mut self,
        name: Option<String>,
        symbol: Option<String>,
        reference: Option<String>,
        reference_hash: Option<Base64VecU8>,
        decimals: Option<u8>,
        icon: Option<String>,
    ) {
        // Only owner can change the metadata
        assert!(self.controller_or_self());

        name.map(|name| self.name = name);
        symbol.map(|symbol| self.symbol = symbol);
        reference.map(|reference| self.reference = reference);
        reference_hash.map(|reference_hash| self.reference_hash = reference_hash);
        decimals.map(|decimals| self.decimals = decimals);
        icon.map(|icon| self.icon = Some(icon));
    }

    #[payable]
    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "Only controller can call mint"
        );

        self.storage_deposit(Some(account_id.clone()), None);
        self.token.internal_deposit(&account_id, amount.into());
        FtMint {
            owner_id: &account_id,
            amount: &amount,
            memo: None,
        }
        .emit();
    }

    #[payable]
    pub fn withdraw(&mut self, amount: U128) -> Promise {
        self.assert_not_paused(PAUSE_WITHDRAW);

        assert_one_yocto();

        let burn_promise = ext_connector::ext(self.controller.clone())
            .with_static_gas(BURN_GAS)
            .burn(env::predecessor_account_id(), amount);

        self.token
            .internal_withdraw(&env::predecessor_account_id(), amount.into());

        Promise::new(env::predecessor_account_id())
            .transfer(near_sdk::ONE_YOCTO)
            .then(burn_promise)
    }

    pub fn account_storage_usage(&self) -> StorageUsage {
        self.token.account_storage_usage
    }

    /// Return true if the caller is either controller or self
    pub fn controller_or_self(&self) -> bool {
        let caller = env::predecessor_account_id();
        caller == self.controller || caller == env::current_account_id()
    }
}

near_contract_standards::impl_fungible_token_core!(BridgeToken, token);
near_contract_standards::impl_fungible_token_storage!(BridgeToken, token);

#[near_bindgen]
impl FungibleTokenMetadataProvider for BridgeToken {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            icon: self.icon.clone(),
            reference: Some(self.reference.clone()),
            reference_hash: Some(self.reference_hash.clone()),
            decimals: self.decimals,
        }
    }
}

admin_controlled::impl_admin_controlled!(BridgeToken, paused);
