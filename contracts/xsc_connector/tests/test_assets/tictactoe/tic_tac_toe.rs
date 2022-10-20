//! Tic tac toe contract built during hackathon

extern crate near_sdk;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault, Gas, Balance};
use near_sdk::serde_json::{json, self};

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum BoardField {
    X,
    O,
    U
}

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
#[serde(crate = "near_sdk::serde")]
pub enum GameStatus {
    InProgress,
    PlayerAWon,
    PlayerBWon,
    Tie
}

#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Game {
    board: Vec<Vec<BoardField>>,
    player_a: AccountId, // player A is always O
    player_b: AccountId, // player B is always X
    status: GameStatus,
    player_a_turn: bool, // if true, player A's turn, else player B's turn
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct TicTacToe {
    games: LookupMap<usize, Game>,
    player_awaiting_for_opponent: Option<AccountId>,
}

const CROSS_SHARD_CALL_CONTRACT_ID: &str = "xscc.90.apptest-development.testnet";
const DESTINATION_CONTRACT_ID: &str = "testtictactoe.90.calimero.testnet"; 
const DESTINATION_CONTRACT_METHOD: &str = "start_game";
const DESTINATION_GAS: Gas = Gas(20_000_000_000_000);
const DESTINATION_DEPOSIT: Balance = 0;
const NO_DEPOSIT: Balance = 0;
const CROSS_CALL_GAS: Gas = Gas(20_000_000_000_000);

#[near_bindgen]
impl TicTacToe {
    #[init]
    pub fn new() -> Self {
        Self {
            games: LookupMap::new(b"m"),
            player_awaiting_for_opponent: None,
        }
    }

    pub fn get_game(&self, game_id: usize) -> Option<Game> {
        if !self.games.contains_key(&game_id) {
            None
        } else {
            Some(self.games.get(&game_id).unwrap().clone())
        }
    }

    pub fn register_player(&mut self) {
        if let Some(first_player) = self.player_awaiting_for_opponent.clone() {

            self.player_awaiting_for_opponent = None;

            env::promise_return(env::promise_create(
                AccountId::new_unchecked(CROSS_SHARD_CALL_CONTRACT_ID.to_string()
            ),
                "cross_call",
                &serde_json::to_vec(&(
                    DESTINATION_CONTRACT_ID, 
                    DESTINATION_CONTRACT_METHOD, 
                    json!({"player_a":first_player,"player_b":env::predecessor_account_id()}).to_string(), 
                    DESTINATION_GAS, 
                    DESTINATION_DEPOSIT, 
                    "game_started")).unwrap(),
                NO_DEPOSIT,
                CROSS_CALL_GAS,
            ));
        } else {
            self.player_awaiting_for_opponent = Some(env::predecessor_account_id());
        }
    }

    pub fn game_started(&self, response: Option<Vec<u8>>) -> Option<usize> {
        if response.is_none() {
            // Call failed
            return None;
        } else {
            let deserialized_reponse: usize = near_sdk::serde_json::from_slice::<usize>(&response.unwrap()).unwrap();
            env::log_str(&format!("GOT THE CALLBACK WITH EXEC RESULT {}", deserialized_reponse));
            return Some(deserialized_reponse)
        }
    }

    pub fn game_ended(&mut self, game_id: usize, game: Game) {
        env::log_str(&format!("game ended, called by the connector {}", env::predecessor_account_id()));
        self.games.insert(&game_id, &game); 
    }
    
}
