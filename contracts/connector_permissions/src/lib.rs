use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};
use near_sdk::collections::UnorderedSet;
use types::ConnectorType;
use regex::Regex;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ConnectorPermissions {
    /// The account of the ft connector
    pub ft_connector_account: AccountId,
    /// The account of the nft connector
    pub nft_connector_account: AccountId,
    /// The account of the cross shard connector
    pub xsc_connector_account: AccountId,
    /// set of allowed regex rules for bridging fts
    pub allow_regex_rules_for_bridging_fts: UnorderedSet<String>,
    /// set of allowed regex rules for bridging nfts
    pub allow_regex_rules_for_bridging_nfts: UnorderedSet<String>,
    /// set of allowed regex rules for making cross shard calls
    pub allow_regex_rules_for_cross_shard_calls: UnorderedSet<String>,
    /// set of regex rules for accounts denied for making cross shard calls per contract id
    /// defined by regex, the data in the set is in form (contract_rule_regex, account_rule_regex)
    pub deny_regex_account_per_regex_contract_for_cross_shard_calls: UnorderedSet<(String, String)>,
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
            allow_regex_rules_for_bridging_fts: UnorderedSet::new(b"dft".to_vec()),
            allow_regex_rules_for_bridging_nfts: UnorderedSet::new(b"dnft".to_vec()),
            allow_regex_rules_for_cross_shard_calls: UnorderedSet::new(b"dxsc".to_vec()),
            deny_regex_account_per_regex_contract_for_cross_shard_calls: UnorderedSet::new(b"dxscc".to_vec()),
        }
    }

    fn connector_from_type(&self, connector_type: ConnectorType) -> &AccountId {
        match connector_type {
            ConnectorType::FT => &self.ft_connector_account,
            ConnectorType::NFT => &self.nft_connector_account,
            ConnectorType::XSC => &self.xsc_connector_account,
        }
    }

    // returns true if account matches any of the regex rules defined within the set of rules
    fn account_matches_rule(&self, account: &str, rules_set: &UnorderedSet<String>) -> bool {
        for rule in rules_set.iter() {
            let re = Regex::new(&rule).unwrap();
            if re.is_match(account) {
                return true;
            }
        }

        false
    }

    /// returns true if account_id is not denied per connector type
    pub fn can_bridge(&self, account_id: &AccountId, connector_type: ConnectorType) -> bool {
        match connector_type {
            ConnectorType::FT => self.account_matches_rule(account_id.as_str(), &self.allow_regex_rules_for_bridging_fts),
            ConnectorType::NFT => self.account_matches_rule(account_id.as_str(), &self.allow_regex_rules_for_bridging_nfts),
            ConnectorType::XSC => self.account_matches_rule(account_id.as_str(), &self.allow_regex_rules_for_cross_shard_calls),
        }
    }

    // adds new regex rule to the list of regex rules per connector type, can only be called by the corresponding connector
    pub fn add_allow_regex_rule(&mut self, regex_rule: &String, connector_type: ConnectorType) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            *self.connector_from_type(connector_type),
            "Only corresponding connector can add regex rules to the rule list"
        );
        match connector_type {
            ConnectorType::FT => self.allow_regex_rules_for_bridging_fts.insert(regex_rule),
            ConnectorType::NFT => self.allow_regex_rules_for_bridging_nfts.insert(regex_rule),
            ConnectorType::XSC => self.allow_regex_rules_for_cross_shard_calls.insert(regex_rule),
        }
    }

    // removes regex rule from the list of regex rules per connector type, can only be called by the corresponding connector
    pub fn remove_allowed_regex_rule(&mut self, regex_rule: &String, connector_type: ConnectorType) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            *self.connector_from_type(connector_type),
            "Only corresponding connector can remove regex rules from the rule list"
        );
        match connector_type {
            ConnectorType::FT => self.allow_regex_rules_for_bridging_fts.remove(regex_rule),
            ConnectorType::NFT => self.allow_regex_rules_for_bridging_nfts.remove(regex_rule),
            ConnectorType::XSC => self.allow_regex_rules_for_cross_shard_calls.remove(regex_rule),
        }
    }

    // adds the account regex rule and the contract regex rule to a denied list, can only be called by cross shard calls connector contract
    pub fn deny_cross_shard_call_per_contract(&mut self, account_regex: &String, contract_regex: &String) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.xsc_connector_account,
            "Only cross shard connector can add accounts regex rules to the deny list for contract regex rule"
        );
        self.deny_regex_account_per_regex_contract_for_cross_shard_calls.insert(&(contract_regex.to_string(), account_regex.to_string()))
    }

    // removes the account regex rule and the contract regex rule from a denied list, can only be called by cross shard calls connector contract
    pub fn remove_denied_cross_shard_call_per_contract(&mut self, account_regex: &String, contract_regex: &String) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.xsc_connector_account,
            "Only cross shard connector can remove accounts regex rules from the deny list for contract regex rule"
        );
        self.deny_regex_account_per_regex_contract_for_cross_shard_calls.remove(&(contract_regex.to_string(), account_regex.to_string()))
    }

    // checks if account_id is denied to make cross shard contract calls and whether account_id and contract_id pair is denied by any of the regex rules
    pub fn can_make_cross_shard_call_for_contract(&self, account_id: &AccountId, contract_id: AccountId) -> bool {
        if !self.can_bridge(account_id, ConnectorType::XSC) {
            return false;
        }

        for (contract_rule, account_rule) in self.deny_regex_account_per_regex_contract_for_cross_shard_calls.iter() {
            let contract_regex = Regex::new(&contract_rule).unwrap();
            let account_regex = Regex::new(&account_rule).unwrap();
            if contract_regex.is_match(contract_id.as_str()) && account_regex.is_match(account_id.as_str()) {
                return false;
            }
        }

        true
    }
}
