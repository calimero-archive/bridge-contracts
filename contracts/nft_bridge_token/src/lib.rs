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
    reference: Option<String>,
    reference_hash: Option<Base64VecU8>,
    base_uri: Option<String>,
    paused: Mask,
    icon: Option<String>,
}

#[ext_contract(ext_connector)]
trait ExtConnector {
    fn burn(&self, burner_id: AccountId, transferable: &TokenId);
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
            token: NonFungibleToken::new(
                b"t".to_vec(),
                env::current_account_id(),
                Some(b"m"),
                Some(b"e"),
                Some(b"a"),
            ),
            name: String::default(),
            symbol: String::default(),
            reference: None,
            reference_hash: None,
            base_uri: None,
            paused: Mask::default(),
            icon: None,
        }
    }

    // TODO see if this needs to be secured to only call it once
    pub fn set_metadata(
        &mut self,
        name: String,
        symbol: String,
        reference: Option<String>,
        reference_hash: Option<Base64VecU8>,
        icon: Option<String>,
        base_uri: Option<String>,
    ) {
        // Only owner can change the metadata
        assert!(self.controller_or_self(), "Only owner can change NFT contract metadata");

        self.name = name;
        self.symbol = symbol;
        self.reference = reference;
        self.reference_hash = reference_hash;
        self.icon = icon;
        self.base_uri = base_uri;
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
        let owner = owner.as_ref().unwrap();

        let burn_promise = ext_connector::ext(self.controller.clone())
            .with_static_gas(BURN_GAS)
            .burn(env::predecessor_account_id(), &token_id);

        self.token.owner_by_id.remove(&token_id);

        if let Some(tokens_per_owner) = &mut self.token.tokens_per_owner {
            // owner_tokens should always exist, so call `unwrap` without guard
            let mut owner_tokens = tokens_per_owner.get(owner).unwrap_or_else(|| {
                env::panic_str("Unable to access tokens per owner in unguarded call.")
            });
            owner_tokens.remove(&token_id);
            if owner_tokens.is_empty() {
                tokens_per_owner.remove(owner);
            } else {
                tokens_per_owner.insert(owner, &owner_tokens);
            }
        }

        NftBurn {
            owner_id: owner,
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
            reference: self.reference.clone(),
            reference_hash: self.reference_hash.clone(),
            base_uri: self.base_uri.clone(),
        }
    }
}

admin_controlled::impl_admin_controlled!(BridgeToken, paused);
