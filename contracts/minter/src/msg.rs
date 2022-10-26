use std::fmt::{self};

use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    // Key: burner token address
    pub supported_tokens: Vec<SupportedToken>,
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
    UpdateMinter {
        asset: String,
        new_minter: String,
    },
    ExecuteMigration(Box<MigrationReq>),

    // contact owner actions
    AddToken {
        burner_token_addr: String,
        minter_token_addr: String,
        token_type: TokenType,
    },
    RemoveToken {
        burner_token_addr: String,
    },

    UpdateOwner {
        new_owner: String,
    },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractMigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrationReq {
    pub burner_id: u64,
    pub asset: String,
    pub token_req: Option<TokenMigrationReq>,
    pub nft_req: Option<NftMigrationReq>,
    pub to: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenMigrationReq {
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NftMigrationReq {
    pub id: String,
    pub uri: Option<String>,
    pub extension: Option<NftExtension>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftExtension {
    image: Option<String>,
    image_data: Option<String>,
    external_url: Option<String>,
    description: Option<String>,
    name: Option<String>,
    attributes: Vec<NftExtensionDisplay>,
    background_color: Option<String>,
    animation_url: Option<String>,
    youtube_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftExtensionDisplay {
    display_type: Option<String>,
    trait_type: Option<String>,
    value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    MigrationResult { burner_id: u64 },
    SupportedTokens { start_after: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryResponse {
    pub migration_result: Option<MigrationResultResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrationResultResponse {
    pub burner_id: u64,
    pub minter_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Native,
    Cw20,
    Cw721,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenType::Cw20 => write!(f, "cw20"),
            TokenType::Cw721 => write!(f, "cw721"),
            TokenType::Native => write!(f, "native"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    pub addr: String,
    pub token_type: TokenType,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CustomCw721ExecuteMsg {
    UpdateMinter { new_minter: String },
}
