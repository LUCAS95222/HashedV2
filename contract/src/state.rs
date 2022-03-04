use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;

use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
  pub owner: Addr
}

pub struct NftLockContract<'a> {
  pub config: Item<'a, Config>,
  pub tokens: IndexedMap<'a, Vec<u8>, TokenInfo, TokenIndexes<'a>>
}

impl Default for NftLockContract<'static> {
  fn default() -> Self {
    Self::new(
      "config",
      "tokens",
      "tokens_owner"
    )
  }
}

impl<'a> NftLockContract<'a> {
  fn new(
    config_key: &'a str,
    tokens_key: &'a str,
    tokens_owner_key: &'a str,
  ) -> Self {
    let indexes = TokenIndexes {
      owner: MultiIndex::new(token_owner_idx, tokens_key, tokens_owner_key),
    };
    Self {
      config: Item::new(config_key),
      tokens: IndexedMap::new(tokens_key, indexes)
    }
  }
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInfo {
  pub owner: Addr,
  pub nft_address: Addr,
  pub token_id: String,
  pub lock_info: String,
}

pub struct TokenIndexes<'a> {
  pub owner: MultiIndex<'a, (Addr, Vec<u8>), TokenInfo>,
}

impl<'a> IndexList<TokenInfo> for TokenIndexes<'a> {
  fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TokenInfo>> + '_> {
    let v: Vec<&dyn Index<TokenInfo>> = vec![&self.owner];
    Box::new(v.into_iter())
  }
}

pub fn token_owner_idx(d: &TokenInfo, k: Vec<u8>) -> (Addr, Vec<u8>) {
  (d.owner.clone(), k)
}