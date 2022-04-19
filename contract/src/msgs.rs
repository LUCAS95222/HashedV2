use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw20::{Cw20Coin, Logo, MinterResponse, Cw20ReceiveMsg};

use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::state::GameTokenDistributions;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
  pub token_code_id: u64,
  pub main_token: String,
  pub owner: String,
  pub main_token_distributions: Vec<Distribution>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Distribution {
  pub address: Addr,
  pub rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
  Receive(Cw20ReceiveMsg),

  UpdateConfig {
    token_code_id: Option<u64>,
    owner: Option<String>,
    main_token_distributions: Option<Vec<Distribution>>,
  },

  RegisterBetaInvitation {
    soft_cap: u64,
    hard_cap: u64,
    user_cap: u64,
    invitation_price: Uint128,
    invitation_price_decimals: u8,
    start_time: u64,
    end_time: u64,
    game_token_info: Cw20Info,
    fan_token_info: Cw20Info,
    game_token_distributions: GameTokenDistributions,
  },

  Claim{
    game_token: Addr,
  },

  Refund{
    game_token: Addr,
    refund_amount: u64,
  },

  TokenDistribute{
    game_token: Addr,
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
  BuyBetaInvitation {
    game_token: Addr,
    beta_invitation_amount: u64,
  },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  Config {},
  BetaInvitationInfo{
    game_token: Addr,
  },
  UserState{
    user_addr: Addr,
    game_token: Addr,
  }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20Info {
  pub name: String,
  pub symbol: String,
  pub decimals: Option<u8>,
  pub total_supply: Option<Uint128>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<Logo>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct Cw20InstantiateMsg {
  pub name: String,
  pub symbol: String,
  pub decimals: u8,
  pub initial_balances: Vec<Cw20Coin>,
  pub mint: Option<MinterResponse>,
  pub marketing: Option<InstantiateMarketingInfo>,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
