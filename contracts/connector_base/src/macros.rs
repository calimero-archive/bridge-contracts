#[macro_export]
macro_rules! impl_deployer_aware {
    ($contract: ident, $deploy_event: literal) => {
        #[near_bindgen]
        impl DeployerAware for $contract {
            #[payable]
            fn set_deployer(&mut self, deployer_account: AccountId) {
                near_sdk::assert_self();
                require!(self.deployer_account.is_none());
                let initial_storage = env::storage_usage() as u128;
                self.deployer_account = Some(deployer_account);
                let current_storage = env::storage_usage() as u128;
                require!(
                    env::attached_deposit()
                        >= env::storage_byte_cost() * (current_storage - initial_storage),
                    "Not enough attached deposit to complete initialization"
                );
            }

            #[payable]
            fn deploy_bridge_token(&mut self, source_address: String) {
                near_sdk::assert_self();
                self.assert_not_paused_flags(PAUSE_DEPLOY_TOKEN);

                let initial_storage = env::storage_usage();
                // TODO calculate future storage usage
                let required_deposit = Balance::from(initial_storage - initial_storage)
                    * env::storage_byte_cost()
                    + BRIDGE_TOKEN_INIT_BALANCE;
                require!(
                    env::attached_deposit() >= required_deposit,
                    "Deposit too low"
                );

                let promise = env::promise_create(
                    self.deployer_account.clone().unwrap(),
                    "deploy_bridge_token",
                    &serde_json::to_vec(&(source_address.clone(),)).unwrap(),
                    required_deposit,
                    DEPLOY_GAS + BRIDGE_TOKEN_NEW,
                );

                env::promise_return(env::promise_then(
                    promise,
                    env::current_account_id(),
                    "complete_deployment",
                    &serde_json::to_vec(&(source_address,)).unwrap(),
                    NO_DEPOSIT,
                    BRIDGE_TOKEN_COMPLETE,
                ));
            }

            fn complete_deployment(&mut self, source_address: AccountId) {
                near_sdk::assert_self();
                require!(env::promise_results_count() == 1);

                let bridge_token_address = match env::promise_result(0) {
                    PromiseResult::Successful(x) => {
                        serde_json::from_slice::<Vec<AccountId>>(&x).unwrap()[0].clone()
                    }
                    _ => env::panic_str("Deploy bridge token failed"),
                };

                env::log_str(&format!(
                    "{}:{}:{}",
                    $deploy_event, &source_address, bridge_token_address
                ));
                self.contracts_mapping
                    .insert(&source_address, &bridge_token_address);
                self.all_contracts.insert(&bridge_token_address);

                env::value_return(&serde_json::to_vec(&(bridge_token_address,)).unwrap());
            }
        }
    };
}

#[macro_export]
macro_rules! impl_other_network_token_aware {
    ($contract: ident, $deploy_event: literal) => {
        #[near_bindgen]
        #[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
        pub struct $contract {
            /// The account of the prover that we can use to prove
            prover_account: AccountId,
            /// The contract account which can deny certain accounts from initiating a bridge action
            connector_permissions_account: AccountId,
            /// The account of the locker on other network that is used to lock FT
            locker_account: Option<AccountId>,
            /// The account of the deployer for bridge token
            deployer_account: Option<AccountId>,
            /// Hashes of the events that were already used.
            used_events: LookupSet<Hash>,
            /// Public key of the account deploying connector.
            owner_pk: PublicKey,
            /// Mappings between FT contract on main network and FT contract on this network
            contracts_mapping: LookupMap<AccountId, AccountId>,
            /// All FT contracts that were deployed by this account
            all_contracts: LookupSet<AccountId>,
            /// Mask determining all paused functions
            paused: Mask,
            /// duration in nanoseconds for which proof is considered valid
            /// not used if not provided
            proof_validity_ns: Option<u64>,
        }

        #[near_bindgen]
        impl OtherNetworkTokenAware for $contract {
            /// Initializes the contract.
            /// `prover_account`: NEAR account of the Near Prover contract;
            #[init]
            fn new(
                prover_account: AccountId,
                connector_permissions_account: AccountId,
                proof_validity_ns: Option<u64>,
            ) -> Self {
                require!(!env::state_exists(), "Already initialized");
                Self {
                    prover_account,
                    connector_permissions_account,
                    used_events: LookupSet::new(b"u".to_vec()),
                    contracts_mapping: LookupMap::new(b"c".to_vec()),
                    all_contracts: LookupSet::new(b"a".to_vec()),
                    locker_account: None,
                    deployer_account: None,
                    owner_pk: env::signer_account_pk(),
                    paused: Mask::default(),
                    proof_validity_ns,
                }
            }

            fn view_mapping(&self, source_account: AccountId) -> Option<AccountId> {
                self.contracts_mapping.get(&source_account)
            }

            #[payable]
            fn map_contracts(
                &mut self,
                source_contract: AccountId,
                destination_contract: AccountId,
                proof: FullOutcomeProof,
            ) {
                near_sdk::assert_self();
                require!(env::promise_results_count() == 1);

                let verification_success = match env::promise_result(0) {
                    PromiseResult::Successful(x) => {
                        serde_json::from_slice::<bool>(&x).unwrap()
                    }
                    _ => env::panic_str("Prover failed"),
                };
                require!(verification_success, "Failed to verify the proof");

                let remaining_deposit = self.record_proof(&proof);
                let initial_storage = env::storage_usage() as u128;
                self.contracts_mapping
                    .insert(&destination_contract, &source_contract);
                let current_storage = env::storage_usage() as u128;
                require!(
                    remaining_deposit
                        >= env::storage_byte_cost() * (current_storage - initial_storage),
                    "Not enough attached deposit to complete mapping"
                );
            }

            #[payable]
            fn register_on_other(&mut self, proof: FullOutcomeProof, height: u64) {
                require!(self.locker_account.is_some());
                require!(
                    proof.outcome_proof.outcome_with_id.outcome.executor_id
                        == self.locker_account.as_ref().unwrap().to_string(),
                    "Untrusted prover account, deploy_bridge_token receipt proof required"
                );
                let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
                let params: Vec<&str> = std::str::from_utf8(&event_log)
                    .unwrap()
                    .split(":")
                    .collect();
                require!(
                    params.len() == 3 && params[0] == $deploy_event,
                    "Untrusted proof, deploy_bridge_token receipt proof required"
                );

                let token_contract_account_source: AccountId = params[1].clone().parse().unwrap();
                let token_contract_account_destination: AccountId =
                    params[2].clone().parse().unwrap();

                // check that account deployment was done by locker_account
                let promise_prover = env::promise_create(
                    self.prover_account.clone(),
                    "prove_outcome",
                    &serde_json::to_vec(&(proof.clone(), height)).unwrap(),
                    NO_DEPOSIT,
                    VERIFY_LOG_ENTRY_GAS,
                );

                let promise_result = env::promise_then(
                    promise_prover,
                    env::current_account_id(),
                    "map_contracts",
                    &serde_json::to_vec(&(
                        token_contract_account_source,
                        token_contract_account_destination,
                        proof,
                    ))
                    .unwrap(),
                    env::attached_deposit(),
                    FINISH_DEPOSIT_GAS,
                );

                env::promise_return(promise_result)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_other_network_aware {
    ($contract: ident) => {
        #[near_bindgen]
        impl OtherNetworkAware for $contract {
            #[payable]
            fn set_locker(&mut self, locker_account: AccountId) {
                near_sdk::assert_self();
                require!(self.locker_account.is_none());
                let initial_storage = env::storage_usage() as u128;
                self.locker_account = Some(locker_account);
                let current_storage = env::storage_usage() as u128;
                require!(
                    env::attached_deposit()
                        >= env::storage_byte_cost() * (current_storage - initial_storage),
                    "Not enough attached deposit to complete network connection"
                );
            }

            /// Record proof if it is valid to make sure it is not re-used later for another deposit.
            fn record_proof(&mut self, proof: &FullOutcomeProof) -> Balance {
                near_sdk::assert_self();
                let initial_storage = env::storage_usage();

                require!(
                    self.proof_validity_ns.is_none()
                        || env::block_timestamp()
                            <= proof.block_header_lite.inner_lite.timestamp
                                + self.proof_validity_ns.unwrap(),
                    "Proof expired"
                );

                let proof_key = proof.block_header_lite.hash();
                require!(
                    !self.used_events.contains(&proof_key),
                    "Event cannot be reused for depositing."
                );
                self.used_events.insert(&proof_key);
                let current_storage = env::storage_usage();
                let required_deposit =
                    Balance::from(current_storage - initial_storage) * env::storage_byte_cost();

                env::log_str(&format!("RecordProof:{}", hashes::encode_hex(&proof_key)));

                require!(
                    env::attached_deposit() >= required_deposit,
                    "Deposit too low"
                );
                env::attached_deposit() - required_deposit
            }
        }
    };
}

#[macro_export]
macro_rules! impl_token_mint {
    ($contract: ident) => {
        #[near_bindgen]
        impl TokenMint for $contract {
            /// Used when receiving Token from other network
            #[payable]
            fn mint(&mut self, proof: FullOutcomeProof, height: u64) {
                self.assert_not_paused(PAUSE_MINT);
                require!(self.locker_account.is_some());
                require!(
                    proof.outcome_proof.outcome_with_id.outcome.executor_id
                        == self.locker_account.as_ref().unwrap().to_string(),
                    "Untrusted proof, lock receipt proof required"
                );
                let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
                let params: Vec<String> = std::str::from_utf8(&event_log)
                    .unwrap()
                    .split(":")
                    .map(String::from)
                    .collect();

                $contract::verify_mint_params(params.clone());

                let token_contract_account = params[1].clone();

                let promise_prover = env::promise_create(
                    self.prover_account.clone(),
                    "prove_outcome",
                    &serde_json::to_vec(&(proof.clone(), height)).unwrap(),
                    NO_DEPOSIT,
                    PROVE_OUTCOME_GAS,
                );

                let promise_result = env::promise_then(
                    promise_prover,
                    env::current_account_id(),
                    "finish_mint",
                    &serde_json::to_vec(&(
                        env::predecessor_account_id(),
                        token_contract_account,
                        params,
                        proof,
                    ))
                    .unwrap(),
                    env::attached_deposit(),
                    FINISH_DEPOSIT_GAS,
                );

                env::promise_return(promise_result)
            }

            /// Finish depositing once the proof was successfully validated. Can only be called by the contract
            /// itself.
            #[payable]
            fn finish_mint(
                &mut self,
                caller_id: AccountId,
                token_contract_account: String,
                params: Vec<String>,
                proof: FullOutcomeProof,
            ) {
                near_sdk::assert_self();
                require!(env::promise_results_count() == 1);

                let verification_success = match env::promise_result(0) {
                    PromiseResult::Successful(x) => {
                        serde_json::from_slice::<bool>(&x).unwrap()
                    }
                    _ => env::panic_str("Prover failed"),
                };
                require!(verification_success, "Failed to verify the proof");

                let remaining_deposit = self.record_proof(&proof);
                let transfer_promise = if let Some(token_contract) = self
                    .contracts_mapping
                    .get(&token_contract_account.parse().unwrap())
                {
                    let refund_promise = env::promise_batch_create(&caller_id);
                    env::promise_batch_action_transfer(refund_promise, remaining_deposit);

                    env::promise_then(
                        refund_promise,
                        token_contract,
                        "mint",
                        &serde_json::to_vec(&$contract::token_mint_params(params)).unwrap(),
                        near_sdk::ONE_NEAR,
                        MINT_GAS,
                    )
                } else {
                    env::panic_str("Token is not yet mapped")
                };

                env::promise_return(transfer_promise)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_token_unlock {
    ($contract: ident, $burn_event: literal, $transferable: ident, $transfer_function: literal) => {
        #[near_bindgen]
        impl TokenUnlock<$transferable> for $contract {
            /// used when burning Token on this network
            fn burn(&mut self, burner_id: AccountId, transferable: $transferable) {
                require!(
                    self.all_contracts.contains(&env::predecessor_account_id()),
                    "Untrusted burn"
                );
                env::log_str(&format!(
                    "{}:{}:{}:{}",
                    $burn_event,
                    env::predecessor_account_id(),
                    burner_id,
                    $contract::transform_transferable(transferable),
                ));
            }

            /// Used when receiving Token from other network
            #[payable]
            fn unlock(&mut self, proof: FullOutcomeProof, height: u64) {
                require!(self.locker_account.is_some());
                require!(
                    proof.outcome_proof.outcome_with_id.outcome.executor_id
                        == self.locker_account.as_ref().unwrap().to_string(),
                    "Untrusted prover account, burn receipt proof required"
                );
                let event_log = proof.outcome_proof.outcome_with_id.outcome.logs[0].clone();
                let params: Vec<&str> = std::str::from_utf8(&event_log)
                    .unwrap()
                    .split(":")
                    .collect();
                require!(
                    params.len() == 4 && params[0] == $burn_event,
                    "Untrusted proof, burn receipt proof required"
                );
                let destination_contract = &params[1];
                let token_receiver_account = &params[2];
                let transferable = $contract::parse_transferable(params[3].clone().to_owned());

                let token_contract_account: AccountId = self
                    .contracts_mapping
                    .get(&destination_contract.parse().unwrap())
                    .unwrap();

                let promise_prover = env::promise_create(
                    self.prover_account.clone(),
                    "prove_outcome",
                    &serde_json::to_vec(&(proof.clone(), height)).unwrap(),
                    NO_DEPOSIT,
                    VERIFY_LOG_ENTRY_GAS,
                );

                let promise_result = env::promise_then(
                    promise_prover,
                    env::current_account_id(),
                    "finish_unlock",
                    &serde_json::to_vec(&(
                        env::predecessor_account_id(),
                        token_contract_account,
                        token_receiver_account,
                        transferable,
                        proof,
                    ))
                    .unwrap(),
                    env::attached_deposit(),
                    FINISH_UNLOCK_GAS + MINT_GAS + TRANSFER_CALL_GAS,
                );

                env::promise_return(promise_result)
            }

            /// Finish depositing once the proof was successfully validated. Can only be called by the contract
            /// itself.
            #[payable]
            fn finish_unlock(
                &mut self,
                caller_id: AccountId,
                token_contract_account: AccountId,
                token_receiver_account: AccountId,
                transferable: $transferable,
                proof: FullOutcomeProof,
            ) {
                near_sdk::assert_self();
                require!(env::promise_results_count() == 1);

                let verification_success = match env::promise_result(0) {
                    PromiseResult::Successful(x) => {
                        serde_json::from_slice::<bool>(&x).unwrap()
                    }
                    _ => env::panic_str("Prover failed"),
                };
                require!(verification_success, "Failed to verify the proof");

                let remaining_deposit = self.record_proof(&proof);

                let refund_promise = env::promise_batch_create(&caller_id);
                env::promise_batch_action_transfer(refund_promise, remaining_deposit);

                let memo = String::from(format!(
                    "Transfer from {}",
                    self.locker_account.as_ref().unwrap().to_string()
                ));

                env::promise_return(env::promise_then(
                    refund_promise,
                    token_contract_account,
                    $transfer_function,
                    &serde_json::to_vec(&$contract::token_unlock_params(
                        token_receiver_account,
                        transferable,
                        memo,
                    ))
                    .unwrap(),
                    near_sdk::ONE_YOCTO,
                    MINT_GAS,
                ))
            }
        }
    };
}
