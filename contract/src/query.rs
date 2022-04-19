use cosmwasm_std::{to_binary, Binary, Deps, Order, StdResult};

use cw_storage_plus::Bound;

use crate::state::{NftLockContract, Config, TokenInfo};
use crate::msgs::{QueryMsg, OwnerOfResponse, NftInfo};

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;

impl<'a> NftLockContract<'a> {
  pub fn query(&self, deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
      QueryMsg::Config {} => to_binary(&self.config(deps)?),
      QueryMsg::OwnerOf { nft_address, token_id } => to_binary(&self.owner_of(deps, nft_address, token_id)?), 
      QueryMsg::Tokens { owner, start_after, limit } => {
        to_binary(&self.tokens(deps, owner, start_after, limit)?)
      },
    }
  }
}


impl<'a> NftLockContract<'a> {
  fn owner_of(&self, deps: Deps, nft_address: String, token_id: String) -> StdResult<OwnerOfResponse> {
    let key = self.gen_key(deps.api.addr_validate(&nft_address)?, token_id);
    let token = self.tokens.may_load(deps.storage, key)?;
    if token.is_some() {
      Ok(OwnerOfResponse {
        owner: token.unwrap().owner.to_string()
      })
    } else {
      Ok(OwnerOfResponse {
        owner: "There is no owner".to_string()
      })
    }
  }

  fn config(&self, deps: Deps) -> StdResult<Config> {
    self.config.load(deps.storage)
  }


  fn tokens(
    &self,
    deps: Deps,
    owner: String,
    start_after: Option<NftInfo>,
    limit: Option<u8>,
  ) -> StdResult<Vec<TokenInfo>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start: Option<Bound>;
    if let Some(start_after) = start_after {
      let key = self.gen_key(deps.api.addr_validate(&start_after.nft_address)?, start_after.token_id);
      start = Some(Bound::exclusive(key))
    } else {
      start = None
    }

    let owner_addr = deps.api.addr_validate(&owner)?;
    let pks: Vec<_> = self
      .tokens
      .idx
      .owner
      .prefix(owner_addr)
      .keys(deps.storage, start, None, Order::Ascending)
      .take(limit)
      .collect();

    let res: Vec<TokenInfo> = pks.iter().map(|v| self.tokens.load(deps.storage, v.clone()).unwrap()).collect();

    Ok(res)
  }
}