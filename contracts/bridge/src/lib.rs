use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, near_bindgen, Gas, PanicOnDefault, PromiseOrValue};

use near_primitives::types::EpochId;
use near_primitives::views::validator_stake_view::ValidatorStakeView;
use near_crypto::{PublicKey, Signature};


near_sdk::setup_alloc!();

type AccountId = String;

// Current assumptions is that private shard only run max 100 block producers
const MAX_BLOCK_PRODUCERS: u32 = 100;

// Bitmask representing different pause actions on the bridge
const UNPAUSE_ALL: Mask = 0;
const PAUSED_DEPOSIT: Mask = 1;
const PAUSED_WITHDRAW: Mask = 2;
const PAUSED_ADD_BLOCK: Mask = 4;
const PAUSED_CHALLENGE: Mask = 8;
const PAUSED_VERIFY: Mask = 16;

pub struct Epoch {
    epoch_id: EpochId,
    block_producer_count: u32,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct RelayerBridgeContract {
    paused: Mask,

}

impl RelayerBridgeContract {
    //#[init]
    //pub fn init() -> Self {}

}

admin_controlled::impl_admin_controlled!(RelayerBridgeContract, paused);
