use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

use crate::msg::{NftMigrationReq, TokenInfo, TokenMigrationReq};

pub const CONFIG: Item<Config> = Item::new("config");

pub const SUPPORTED_TOKEN_MAP: Map<String, TokenInfo> = Map::new("supported_token_map");

pub const BURNER_MINTER_IDX: Map<u64, u64> = Map::new("burner_minter_idx");

pub const TXS: Map<u64, Tx> = Map::new("txs");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub tx_idx: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Tx {
    pub id: u64,
    pub burner_id: u64,
    pub recipient: Addr,
    pub asset: String,
    pub token_req: Option<TokenMigrationReq>,
    pub nft_req: Option<NftMigrationReq>,
}
