use cosmwasm_std::{from_binary, to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response,
  StdResult, WasmMsg};

use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg};

use crate::state::{Config, NftLockContract, TokenInfo};
use crate::msgs::{InstantiateMsg, ExecuteMsg, Cw721ReceiveHook};
use crate::error::ContractError;


impl<'a> NftLockContract<'a> {
  pub fn instantiate(
    &self,
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
  ) -> StdResult<Response> {
    let config = Config {
      owner: deps.api.addr_validate(&msg.owner)?
    };

    self.config.save(deps.storage, &config)?;

    Ok(Response::new())
  }

  pub fn execute(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
  ) -> Result<Response, ContractError> {
    match msg{
      ExecuteMsg::ReceiveNft(msg) => self.receive_cw721(deps, env, info, msg),
      ExecuteMsg::Unlock{ nft_address, token_id } => self.unlock(deps, env, info, nft_address, token_id),
      ExecuteMsg::UpdateConfig{ owner } => self.update_config(deps, env, info, owner),
    }
  }
}

// execute functions
impl<'a> NftLockContract<'a> {
  pub fn receive_cw721(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    cw721_msg: Cw721ReceiveMsg,
  ) -> Result<Response, ContractError> {
    let nft_address = info.sender.clone();
    let token_id = cw721_msg.token_id.clone();
    let sender = cw721_msg.sender.clone();

    match from_binary(&cw721_msg.msg) {
      Ok(Cw721ReceiveHook::Lock {
        lock_info
      }) => {
        let owner = sender.clone();
        let token_info = TokenInfo {
          owner: deps.api.addr_validate(&owner)?,
          nft_address: nft_address.clone(),
          token_id: token_id.clone(),
          lock_info: lock_info.clone()
        };

        let key = self.gen_key(nft_address.clone(), token_id.clone());

        self.tokens.save(deps.storage, key, &token_info)?;

        Ok(
          Response::new()
          .add_attribute("sender", sender)
          .add_attribute("action", "lock")
          .add_attribute("nft_address", nft_address)
          .add_attribute("token_id", token_id)
          .add_attribute("lock_info", lock_info)
        )
      }
      Err(err) => Err(ContractError::Std(err)),
    }
  }

  pub fn unlock(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    nft_address: String,
    token_id: String
  ) -> Result<Response, ContractError> {
    let config = self.config.load(deps.storage)?;

    // only owner can unlock nft
    if info.sender != config.owner {
      return Err(ContractError::Unauthorized {})
    }

    let key = self.gen_key(deps.api.addr_validate(&nft_address)?, token_id.clone());

    let token = self.tokens.load(deps.storage, key.clone())?;

    // remove data
    self.tokens.remove(deps.storage, key)?;

    // add transfer nft msg
    let message = CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: token.nft_address.to_string() ,
      msg: to_binary(&Cw721ExecuteMsg::TransferNft { 
        recipient: token.owner.to_string(),
        token_id: token_id.clone() 
      })?,
      funds: vec![],
    });

    Ok(
      Response::new()
      .add_message(message)
      .add_attribute("sender", info.sender)
      .add_attribute("action", "unlock")
      .add_attribute("nft_address", nft_address)
      .add_attribute("token_id", token_id)
    )
  }

  pub fn update_config(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
  ) -> Result<Response, ContractError> {
    let mut config: Config = self.config.load(deps.storage)?;
    
    if info.sender != config.owner {
      return Err(ContractError::Unauthorized {})
    }

    if let Some(owner) = owner {
      config.owner = deps.api.addr_validate(&owner)?;
    }

    self.config.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
  }
}