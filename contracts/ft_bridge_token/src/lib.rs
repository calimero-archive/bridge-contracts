use admin_controlled::Mask;
use near_contract_standards::fungible_token::events::FtMint;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::storage_management::{
    StorageManagement, StorageBalance, StorageBalanceBounds
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
    reference: Option<String>,
    reference_hash: Option<Base64VecU8>,
    decimals: u8,
    paused: Mask,
    icon: Option<String>,
    metadata_set: bool,
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
            reference: None,
            reference_hash: None,
            decimals: 0,
            paused: Mask::default(),
            icon: None,
            metadata_set: false,
        }
    }

    pub fn set_metadata(
        &mut self,
        name: String,
        symbol: String,
        reference: Option<String>,
        reference_hash: Option<Base64VecU8>,
        decimals: u8,
        icon: Option<String>,
    ) {
        // Only owner can change the metadata
        assert!(self.controller_or_self(), "Only owner can change FT contract metadata");
        assert!(!self.metadata_set, "Metadata was already set");

        self.name = name;
        self.symbol = symbol;
        self.reference = reference;
        self.reference_hash = reference_hash;
        self.decimals = decimals;
        self.icon = icon;
        self.metadata_set = true;
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

#[near_bindgen]
impl StorageManagement for BridgeToken {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, _amount: Option<U128>) -> StorageBalance {
        panic!("storage_withdraw method is not supported for bridged tokens.")
    }

    #[payable]
    fn storage_unregister(&mut self, _force: Option<bool>) -> bool {
        panic!("storage_unregister method is not supported for bridged tokens.")
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for BridgeToken {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            icon: self.icon.clone(),
            reference: self.reference.clone(),
            reference_hash: self.reference_hash.clone(),
            decimals: self.decimals,
        }
    }
}

admin_controlled::impl_admin_controlled!(BridgeToken, paused);
