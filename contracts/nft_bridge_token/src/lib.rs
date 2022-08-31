use admin_controlled::Mask;
use near_contract_standards::non_fungible_token::events::NftBurn;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, AccountId, Gas, PanicOnDefault, Promise,
    PromiseOrValue,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct BridgeToken {
    controller: AccountId,
    token: NonFungibleToken,
    name: String,
    symbol: String,
    reference: String,
    reference_hash: Base64VecU8,
    base_uri: Option<String>,
    paused: Mask,
    icon: Option<String>,
}

#[ext_contract(ext_connector)]
trait ExtConnector {
    fn burn(&self, burner_id: AccountId, token_id: &TokenId);
}

/// Gas to call burn method on controller.
const BURN_GAS: Gas = Gas(30_000_000_000_000);

const PAUSE_WITHDRAW: Mask = 1 << 0;

#[near_bindgen]
impl BridgeToken {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            controller: env::predecessor_account_id(),
            token: NonFungibleToken::new(
                b"t".to_vec(),
                env::current_account_id(),
                Some(b"m"),
                Some(b"e"),
                Some(b"a"),
            ),
            name: String::default(),
            symbol: String::default(),
            reference: String::default(),
            reference_hash: Base64VecU8(vec![]),
            base_uri: None,
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
        icon: Option<String>,
        base_uri: Option<String>,
    ) {
        // Only owner can change the metadata
        assert!(self.controller_or_self());

        name.map(|name| self.name = name);
        symbol.map(|symbol| self.symbol = symbol);
        reference.map(|reference| self.reference = reference);
        reference_hash.map(|reference_hash| self.reference_hash = reference_hash);
        icon.map(|icon| self.icon = Some(icon));
        base_uri.map(|base_uri| self.base_uri = Some(base_uri));
    }

    #[payable]
    pub fn mint(
        &mut self,
        account_id: AccountId,
        token_id: TokenId,
        token_metadata: TokenMetadata,
    ) {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "Only controller can call mint"
        );
        self.token
            .internal_mint(token_id, account_id, Some(token_metadata));
    }

    #[payable]
    pub fn withdraw(&mut self, token_id: TokenId) -> Promise {
        self.assert_not_paused(PAUSE_WITHDRAW);
        let owner = self.token.owner_by_id.get(&token_id);

        assert_one_yocto();
        assert_eq!(
            Some(env::predecessor_account_id()),
            owner,
            "Only owner can call withdraw"
        );

        let burn_promise = ext_connector::ext(self.controller.clone())
            .with_static_gas(BURN_GAS)
            .burn(env::predecessor_account_id(), &token_id);

        self.token.owner_by_id.remove(&token_id);
        NftBurn {
            owner_id: &owner.unwrap(),
            token_ids: &[&token_id],
            memo: None,
            authorized_id: None, 
        }.emit();

        Promise::new(env::predecessor_account_id())
            .transfer(near_sdk::ONE_YOCTO)
            .then(burn_promise)
    }

    /// Return true if the caller is either controller or self
    pub fn controller_or_self(&self) -> bool {
        let caller = env::predecessor_account_id();
        caller == self.controller || caller == env::current_account_id()
    }
}

near_contract_standards::impl_non_fungible_token_core!(BridgeToken, token);
near_contract_standards::impl_non_fungible_token_enumeration!(BridgeToken, token);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for BridgeToken {
    fn nft_metadata(&self) -> NFTContractMetadata {
        NFTContractMetadata {
            spec: NFT_METADATA_SPEC.to_string(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            icon: self.icon.clone(),
            reference: Some(self.reference.clone()),
            reference_hash: Some(self.reference_hash.clone()),
            base_uri: self.base_uri.clone(),
        }
    }
}

admin_controlled::impl_admin_controlled!(BridgeToken, paused);
