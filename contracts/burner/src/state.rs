use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map, U32Key, U64Key};

use crate::types::{Status, TokenInfo};

pub const CONFIG: Item<Config> = Item::new("config");

pub const SUPPORTED_TOKEN_MAP: Map<&Addr, TokenInfo> = Map::new("supported_token_map");

pub const TXS: Map<U64Key, Tx> = Map::new("txs");

pub const UNPROCESSED_TX_IDX: Map<U64Key, bool> = Map::new("unprocessed_tx_idx");

pub const UNPROCESSED_NFT_ID_SET: Map<(String, String), bool> = Map::new("unprocessed_nft_id_set");

pub const USER_TXS: Map<(&Addr, U32Key), UserReqInfo> = Map::new("user_txs");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub burn_contract: Addr,
    pub tx_idx: u64,
    pub tx_limit: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Tx {
    pub id: u64,
    pub status: Status,
    pub from: Addr, // user on burner network
    pub to: String, // address on minter network
    pub user_req_id: u32,
    pub token_addr: Addr,
    pub minter_token_addr: String,
    pub amount: Uint128,
    pub nft_id: String,
    pub msg: Option<String>,
    pub minter_id: Option<u64>,
    pub minter_tx_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserReqInfo {
    pub tx_ids: Vec<u64>,
    pub block_num: u64,
    pub timestamp: u64,
    pub success: u8,
    pub fail: u8,
    pub in_progress: u8,
}
