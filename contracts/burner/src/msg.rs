use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{MigrationReq, Status, TokenType};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub supported_tokens: Vec<SupportedToken>,
    pub tx_limit: Option<u8>,
    pub burn_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SupportedToken {
    pub burner_token_addr: String,
    pub minter_token_addr: String,
    pub token_type: TokenType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // user migration request
    RequestMigrations(Vec<MigrationReq>),

    // contact owner actions
    AddToken {
        burner_token_addr: String,
        minter_token_addr: String,
        token_type: TokenType,
    },
    RemoveToken {
        burner_token_addr: String,
    },
    UpdateTxLimit {
        count: u8,
    },
    RecordMigrationResult {
        id: u64,
        // transaction result code from minter
        status: i16,
        minter_id: Option<u64>,
        minter_tx_hash: Option<String>,
        message: Option<String>,
    },
    UpdateOwner {
        new_owner: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // client
    UserMigrations {
        addr: String,
        start_after: Option<u32>,
        descending: Option<bool>,
    },
    UserMigration {
        addr: String,
        req_id: u32,
    },
    SupportedTokens {
        start_after: Option<String>,
    },

    // relayer
    MigrationRequest {
        id: u64,
    },
    UnprocessedMigrationRequest {
        items_per_req: Option<u8>,
        start_after: Option<u64>,
    },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractMigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserMigrationsResponse {
    pub migrations: Vec<UserMigrationsItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserMigrationsItem {
    pub req_id: u32,
    pub block_num: u64,
    pub timestamp: u64,
    pub success: u8,
    pub fail: u8,
    pub in_progress: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserMigrationResponse {
    pub txs: Vec<TxResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TxResponse {
    pub id: u64,
    pub status: Status,
    pub msg: Option<String>,
    pub from: String,
    pub to: String, // address on minter network
    pub user_req_id: u32,
    pub token_addr: String,
    pub amount: Option<String>,
    pub nft_info: Option<NftInfo>,
    pub minter_id: Option<u64>,
    pub minter_tx_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UnprocessedMigrationRequestResponse {
    pub items: Vec<TxResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftExtension {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Vec<NftExtensionDisplay>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftExtensionDisplay {
    pub display_type: Option<String>,
    pub trait_type: Option<String>,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftInfo {
    pub id: String,
    pub uri: Option<String>,
    pub extension: Option<NftExtension>,
}
