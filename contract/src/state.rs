use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Map, MultiIndex, Item, Index, IndexedMap, IndexList};
use crate::msgs::Distribution;

pub struct BetaInvitation<'a> {
  pub config: Item<'a, Config>,
  pub invitation_info: Map<'a, Addr, InvitationInfo>,
  pub user_states: IndexedMap<'a, Vec<u8>, UserState, UserIndexes<'a>>,
  pub temp_invitation_info: Item<'a, InvitationInfo>
}

impl Default for BetaInvitation<'static> {
  fn default() -> Self {
    Self::new(
      "config",
      "invitation_info",
      "user_states",
      "user_address",
      "temp_invitation_info"
    )
  }
}

impl<'a> BetaInvitation<'a> {
  fn new(
    config_key: &'a str,
    invitation_info_key: &'a str,
    user_states_key: &'a str,
    user_address_key: &'a str,
    temp_invitation_info_key: &'a str,
  ) -> Self {
    let user_indexes = UserIndexes {
      address: MultiIndex::new(user_idx, user_states_key, user_address_key),
    };
    Self {
      config: Item::new(config_key),
      invitation_info: Map::new(invitation_info_key),
      user_states: IndexedMap::new(user_states_key, user_indexes),
      temp_invitation_info: Item::new(temp_invitation_info_key),
    }
  }
}

impl<'a> BetaInvitation<'a> {
  pub fn gen_user_state_key(&self, game_token_addr: Addr, user_addr: Addr) -> Vec<u8> {
    return [game_token_addr.as_bytes(), user_addr.as_bytes()].concat()
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
  pub owner: Addr,
  pub main_token: Addr,
  pub token_code_id: u64,
  pub main_token_distributions: Vec<Distribution>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InvitationInfo {
  pub game_token: Addr,
  pub fan_token: Addr,
  pub soft_cap: u64,
  pub hard_cap: u64,
  pub user_cap: u64,
  pub sold_amount: u64,
  pub start_time: u64,
  pub end_time: u64,
  pub invitation_price: Uint128,
  pub game_token_distributions: GameTokenDistributions,
  pub main_token_distributed: bool
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameTokenDistributions {
  pub invitation_buyer: Uint128,
  pub others: Vec<GameTokenDistribution>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameTokenDistribution {
  pub address: Addr,
  pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserState {
  pub address: Addr,
  pub game_token: Addr,
  pub bought_invitation_amount: u64,
  // claim or refund amount after invitation end
  pub claimed: bool,
}

pub struct UserIndexes<'a> {
  pub address: MultiIndex<'a, (Addr, Vec<u8>), UserState>
}

pub fn user_idx(d: &UserState, k: Vec<u8>) -> (Addr, Vec<u8>) {
  (d.address.clone(), k)
}


impl<'a> IndexList<UserState> for UserIndexes<'a> {
  fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UserState>> + '_> {
    let v: Vec<&dyn Index<UserState>> = vec![&self.address];
    Box::new(v.into_iter())
  }
}