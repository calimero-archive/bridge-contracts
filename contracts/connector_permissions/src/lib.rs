use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};
use near_sdk::collections::LookupSet;
use types::ConnectorType;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ConnectorPermissions {
    /// The account of the ft connector
    pub ft_connector_account: AccountId,
    /// The account of the nft connector
    pub nft_connector_account: AccountId,
    /// The account of the cross shard connector
    pub xsc_connector_account: AccountId,
    /// set of accounts denied for bridging fts
    pub deny_listed_accounts_for_bridging_fts: LookupSet<AccountId>,
    /// set of accounts denied for bridging nfts
    pub deny_listed_accounts_for_bridging_nfts: LookupSet<AccountId>,
    /// set of accounts denied for making cross shard calls
    pub deny_listed_accounts_for_cross_shard_calls: LookupSet<AccountId>,
    /// set of accounts denied for making cross shard calls per contract id, the data
    /// in the set is in form {contract_id}|{account_id}
    pub deny_listed_account_per_contract_for_cross_shard_calls: LookupSet<String>,
}

#[near_bindgen]
impl ConnectorPermissions {

    #[init]
    pub fn new(
        ft_connector_account: AccountId,
        nft_connector_account: AccountId,
        xsc_connector_account: AccountId,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        Self {
            ft_connector_account,
            nft_connector_account,
            xsc_connector_account,
            deny_listed_accounts_for_bridging_fts: LookupSet::new(b"dft".to_vec()),
            deny_listed_accounts_for_bridging_nfts: LookupSet::new(b"dnft".to_vec()),
            deny_listed_accounts_for_cross_shard_calls: LookupSet::new(b"dxsc".to_vec()),
            deny_listed_account_per_contract_for_cross_shard_calls: LookupSet::new(b"dxscc".to_vec()),
        }
    }

    fn connector_from_type(&self, connector_type: ConnectorType) -> &AccountId {
        return match connector_type {
            ConnectorType::FT => &self.ft_connector_account,
            ConnectorType::NFT => &self.nft_connector_account,
            ConnectorType::XSC => &self.xsc_connector_account,
        };
    }

    /// returns true if account_id is not denied per connector type
    pub fn can_bridge(&self, account_id: &AccountId, connector_type: ConnectorType) -> bool {
        return match connector_type {
            ConnectorType::FT => !self.deny_listed_accounts_for_bridging_fts.contains(&account_id),
            ConnectorType::NFT => !self.deny_listed_accounts_for_bridging_nfts.contains(&account_id),
            ConnectorType::XSC => !self.deny_listed_accounts_for_cross_shard_calls.contains(&account_id),
        }
    }

    /// adds the account id to a denied list per connector type, can only be called by the corresponding connector
    pub fn deny_bridge(&mut self, account_id: AccountId, connector_type: ConnectorType) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            *self.connector_from_type(connector_type),
            "Only corresponding connector can add accounts to the deny list"
        );
        return match connector_type {
            ConnectorType::FT => self.deny_listed_accounts_for_bridging_fts.insert(&account_id),
            ConnectorType::NFT => self.deny_listed_accounts_for_bridging_nfts.insert(&account_id),
            ConnectorType::XSC => self.deny_listed_accounts_for_cross_shard_calls.insert(&account_id),
        }

    }

    /// removes the account id from the denied list per connector type, can only be called by the corresponding connector
    pub fn allow_bridge(&mut self, account_id: AccountId, connector_type: ConnectorType) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            *self.connector_from_type(connector_type),
            "Only corresponding connector can remove accounts from the deny list"
        );
        return match connector_type {
            ConnectorType::FT => self.deny_listed_accounts_for_bridging_fts.remove(&account_id),
            ConnectorType::NFT => self.deny_listed_accounts_for_bridging_nfts.remove(&account_id),
            ConnectorType::XSC => self.deny_listed_accounts_for_cross_shard_calls.remove(&account_id),
        }
    }

    /// adds the account_id|contract_id to a denied list, can only be called by cross shard calls connector contract
    pub fn deny_cross_shard_call_per_contract(&mut self, account_id: AccountId, contract_id: AccountId) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.xsc_connector_account,
            "Only cross shard connector can add accounts to the deny list for specific contract id"
        );
        self.deny_listed_account_per_contract_for_cross_shard_calls.insert(&format!("{}|{}", account_id, contract_id))
    }

    /// removes the account_id|contract_id from the denied list, can only be called by cross shard calls connector contract
    pub fn allow_cross_shard_call_per_contract(&mut self, account_id: AccountId, contract_id: AccountId) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.xsc_connector_account,
            "Only cross shard connector can remove accounts from the deny list for specific contract id"
        );
        self.deny_listed_account_per_contract_for_cross_shard_calls.remove(&format!("{}|{}", account_id, contract_id))
    }

    /// checks if account_id is denied to make cross shard contract calls and whether the account_id is denied from calling a specific contract id
    pub fn can_make_cross_shard_call_for_contract(&self, account_id: &AccountId, contract_id: AccountId) -> bool {
        if !self.can_bridge(&account_id, ConnectorType::XSC) {
            return false;
        }
        return !self.deny_listed_account_per_contract_for_cross_shard_calls.contains(&format!("{}|{}", account_id, contract_id))
    }
}