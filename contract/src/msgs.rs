use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw721::Cw721ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
  pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
  ReceiveNft(Cw721ReceiveMsg),

  UpdateConfig {
    owner: Option<String>,
  },

  Unlock {
    nft_address: String,
    token_id: String
  },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  Config {},
  
  // find owner of nft
  OwnerOf {
    nft_address: String,
    token_id: String
  },

  // get nfts
  Tokens {
    owner: String, 
    start_after: Option<NftInfo>,
    limit: Option<u8>
  }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct OwnerOfResponse {
  pub owner: String,
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct NftInfo {
  pub nft_address: String,
  pub token_id: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721ReceiveHook {
  Lock {
    // some game data (base64 encoded json)
    lock_info: String
  },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}