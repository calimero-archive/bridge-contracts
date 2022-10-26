pub mod macros;
pub use macros::*;

use near_sdk::{AccountId, Balance};
use types::FullOutcomeProof;

pub trait DeployerAware {
    fn set_deployer(&mut self, deployer_account: AccountId);
    fn deploy_bridge_token(&mut self, source_address: String);
    fn complete_deployment(&mut self, source_address: AccountId);
}

pub trait OtherNetworkAware {
    fn set_locker(&mut self, locker_account: AccountId);
    fn record_proof(&mut self, proof: &FullOutcomeProof) -> Balance;
}

pub trait OtherNetworkTokenAware {
    fn new(
        prover_account: AccountId,
        connector_permissions_account: AccountId,
        proof_validity_ns: Option<u64>,
    ) -> Self;
    fn view_mapping(&self, source_account: AccountId) -> Option<AccountId>;
    fn map_contracts(&mut self, source_contract: AccountId, destination_contract: AccountId);
    fn register_on_other(&mut self, proof: FullOutcomeProof, height: u64);
}

pub trait TokenUnlock<T> {
    fn burn(&mut self, burner_id: AccountId, transferable: T);
    fn unlock(&mut self, proof: FullOutcomeProof, height: u64);
    fn finish_unlock(
        &mut self,
        caller_id: AccountId,
        token_contract_account: AccountId,
        token_receiver_account: AccountId,
        transferable: T,
        proof: FullOutcomeProof,
    );
}

pub trait TokenMint {
    fn mint(&mut self, proof: FullOutcomeProof, height: u64);
    fn finish_mint(
        &mut self,
        caller_id: AccountId,
        token_contract_account: String,
        params: Vec<String>,
        proof: FullOutcomeProof,
    );
}
