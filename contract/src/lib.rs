mod state;
mod helpers;
mod execute;
mod msgs;
mod error;
mod query;

pub use crate::msgs::{InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg};
pub use crate::state::NftLockContract;
pub use crate::error::ContractError;

#[cfg(not(feature = "library"))]
pub mod entry {
  use super::*;

  use cosmwasm_std::entry_point;
  use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

  #[entry_point]
  pub fn instantiate(
      deps: DepsMut,
      env: Env,
      info: MessageInfo,
      msg: InstantiateMsg,
  ) -> StdResult<Response> {
      let tract = NftLockContract::default();
      tract.instantiate(deps, env, info, msg)
  }

  #[entry_point]
  pub fn execute(
      deps: DepsMut,
      env: Env,
      info: MessageInfo,
      msg: ExecuteMsg,
  ) -> Result<Response, ContractError> {
    let tract = NftLockContract::default();
      tract.execute(deps, env, info, msg)
  }

  #[entry_point]
  pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let tract = NftLockContract::default();
      tract.query(deps, msg)
  }

  #[cfg_attr(not(feature = "library"), entry_point)]
  pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
  }
}
