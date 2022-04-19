use cosmwasm_std::{to_binary, Binary, Deps, StdResult, QuerierWrapper, QueryRequest, WasmQuery};
use cw20::{TokenInfoResponse, Cw20QueryMsg};

use crate::state::BetaInvitation;
use crate::msgs::QueryMsg;

impl<'a> BetaInvitation<'a> {
  pub fn query(&self, deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
      QueryMsg::Config {} => to_binary(&self.config.load(deps.storage)?),
      QueryMsg::BetaInvitationInfo { game_token } => to_binary(&self.invitation_info.load(deps.storage, game_token)?),
      QueryMsg::UserState { user_addr, game_token } => {
        let key = self.gen_user_state_key(game_token, user_addr);
        to_binary(&self.user_states.load(deps.storage, key)?)
      }
    }
  }

  pub fn query_decimals(
    &self, 
    querier: &QuerierWrapper,
    token_contract: String,
  ) -> StdResult<u8> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
      contract_addr: token_contract,
      msg: to_binary(&Cw20QueryMsg::TokenInfo {
      })?,
    }))?;

    Ok(res.decimals)
  }
}