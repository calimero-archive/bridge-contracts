use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};
use regex::Regex;
use types::ConnectorType;

const ALLOW_REGEX_RULES_GETTER_SIZE: usize = 10;
const DENY_REGEX_ACCOUNT_PER_CONTRACT_GETTER_SIZE: usize = 20;

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
            deny_regex_account_per_regex_contract_for_cross_shard_calls: UnorderedSet::new(
                b"dxscc".to_vec(),
            ),
        }
    }

    fn require_enough_deposit(&self, current_storage: u128, initial_storage: u128) {
        require!(
            env::attached_deposit()
                >= env::storage_byte_cost() * (current_storage - initial_storage),
            "Not enough attached deposit to complete action"
        );
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

    // restarts permissions for a given connector, for FT, NFT and XSC all allow rules are removed,
    // and for XSC additionally all deny rules per contract per account are also removed. Permissions are returned to the
    // initial state (deny all).
    pub fn reset_permissions(&mut self, connector_type: ConnectorType) {
        assert_eq!(
            env::predecessor_account_id(),
            *self.connector_from_type(connector_type),
            "Only corresponding connector can reset regex rules"
        );
        match connector_type {
            ConnectorType::FT => self.allow_regex_rules_for_bridging_fts.clear(),
            ConnectorType::NFT => self.allow_regex_rules_for_bridging_nfts.clear(),
            ConnectorType::XSC => {
                self.allow_regex_rules_for_cross_shard_calls.clear();
                self.deny_regex_account_per_regex_contract_for_cross_shard_calls.clear();
            }
        }
    }

    /// returns true if account_id is not denied per connector type
    pub fn can_bridge(&self, account_id: &AccountId, connector_type: ConnectorType) -> bool {
        match connector_type {
            ConnectorType::FT => self.account_matches_rule(
                account_id.as_str(),
                &self.allow_regex_rules_for_bridging_fts,
            ),
            ConnectorType::NFT => self.account_matches_rule(
                account_id.as_str(),
                &self.allow_regex_rules_for_bridging_nfts,
            ),
            ConnectorType::XSC => self.account_matches_rule(
                account_id.as_str(),
                &self.allow_regex_rules_for_cross_shard_calls,
            ),
        }
    }

    fn get_rules_from_unordered_set(&self, rules_set: &UnorderedSet<String>) -> Vec<String> {
        let mut rules: Vec<String> = Vec::new();
        for rule in rules_set.iter() {
            rules.push(rule);
            if rules.len() == ALLOW_REGEX_RULES_GETTER_SIZE {
                break;
            }
        }

        rules
    }

    pub fn get_allow_regex_rules(&self, connector_type: ConnectorType) -> Vec<String> {
        match connector_type {
            ConnectorType::FT => self.get_rules_from_unordered_set(&self.allow_regex_rules_for_bridging_fts),
            ConnectorType::NFT => self.get_rules_from_unordered_set(&self.allow_regex_rules_for_bridging_nfts),
            ConnectorType::XSC => self.get_rules_from_unordered_set(&self.allow_regex_rules_for_cross_shard_calls)
        }
    }

    // adds new regex rule to the list of regex rules per connector type, can only be called by the corresponding connector
    #[payable]
    pub fn add_allow_regex_rule(
        &mut self,
        regex_rule: &String,
        connector_type: ConnectorType,
    ) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            *self.connector_from_type(connector_type),
            "Only corresponding connector can add regex rules to the rule list"
        );
        let initial_storage = env::storage_usage() as u128;

        let action_success = match connector_type {
            ConnectorType::FT => self.allow_regex_rules_for_bridging_fts.insert(regex_rule),
            ConnectorType::NFT => self.allow_regex_rules_for_bridging_nfts.insert(regex_rule),
            ConnectorType::XSC => self
                .allow_regex_rules_for_cross_shard_calls
                .insert(regex_rule),
        };

        let current_storage = env::storage_usage() as u128;
        self.require_enough_deposit(current_storage, initial_storage);

        action_success
    }

    // removes regex rule from the list of regex rules per connector type, can only be called by the corresponding connector
    pub fn remove_allowed_regex_rule(
        &mut self,
        regex_rule: &String,
        connector_type: ConnectorType,
    ) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            *self.connector_from_type(connector_type),
            "Only corresponding connector can remove regex rules from the rule list"
        );
        match connector_type {
            ConnectorType::FT => self.allow_regex_rules_for_bridging_fts.remove(regex_rule),
            ConnectorType::NFT => self.allow_regex_rules_for_bridging_nfts.remove(regex_rule),
            ConnectorType::XSC => self
                .allow_regex_rules_for_cross_shard_calls
                .remove(regex_rule),
        }
    }

    // adds the account regex rule and the contract regex rule to a denied list, can only be called by cross shard calls connector contract
    #[payable]
    pub fn deny_cross_shard_call_per_contract(
        &mut self,
        account_regex: &String,
        contract_regex: &String,
    ) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.xsc_connector_account,
            "Only cross shard connector can add accounts regex rules to the deny list for contract regex rule"
        );
        let initial_storage = env::storage_usage() as u128;

        let action_success = self.deny_regex_account_per_regex_contract_for_cross_shard_calls
            .insert(&(contract_regex.to_string(), account_regex.to_string()));

        let current_storage = env::storage_usage() as u128;
        self.require_enough_deposit(current_storage, initial_storage);

        action_success
    }

    // removes the account regex rule and the contract regex rule from a denied list, can only be called by cross shard calls connector contract
    pub fn remove_denied_cross_shard_call_per_contract(
        &mut self,
        account_regex: &String,
        contract_regex: &String,
    ) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.xsc_connector_account,
            "Only cross shard connector can remove accounts regex rules from the deny list for contract regex rule"
        );
        self.deny_regex_account_per_regex_contract_for_cross_shard_calls
            .remove(&(contract_regex.to_string(), account_regex.to_string()))
    }

    // checks if account_id is denied to make cross shard contract calls and whether account_id and contract_id pair is denied by any of the regex rules
    pub fn can_make_cross_shard_call_for_contract(
        &self,
        account_id: AccountId,
        contract_id: AccountId,
    ) -> bool {
        if !self.can_bridge(&account_id, ConnectorType::XSC) {
            return false;
        }

        for (contract_rule, account_rule) in self
            .deny_regex_account_per_regex_contract_for_cross_shard_calls
            .iter()
        {
            let contract_regex = Regex::new(&contract_rule).unwrap();
            let account_regex = Regex::new(&account_rule).unwrap();
            if contract_regex.is_match(contract_id.as_str())
                && account_regex.is_match(account_id.as_str())
            {
                return false;
            }
        }

        true
    }

    pub fn get_regex_account_per_contract_for_xsc(&self) -> Vec<(String, String)> {
        let mut rules: Vec<(String, String)> = Vec::new();
        for rule in self.deny_regex_account_per_regex_contract_for_cross_shard_calls.iter() {
            rules.push(rule);
            if rules.len() == DENY_REGEX_ACCOUNT_PER_CONTRACT_GETTER_SIZE {
                break;
            }
        }

        rules
    }
}
