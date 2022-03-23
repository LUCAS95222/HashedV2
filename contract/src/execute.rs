use cosmwasm_std::{to_binary, from_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, StdError, StdResult, SubMsg, Response, Reply, ReplyOn, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};

use crate::state::{BetaInvitation, Config, InvitationInfo, UserState,  GameTokenDistributions};
use crate::msgs::{InstantiateMsg, ExecuteMsg, Cw20Info, Cw20InstantiateMsg, Distribution, Cw20HookMsg};

use crate::response::MsgInstantiateContractResponse;
use protobuf::Message;

const DEFAULT_DECIMALS:u8 = 6;

impl<'a> BetaInvitation<'a> {
  pub fn instantiate(
    &self,
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg
  ) -> StdResult<Response> {
    let config = Config {
      owner: msg.owner,
      main_token: msg.main_token,
      token_code_id: msg.token_code_id,
      main_token_distributions: msg.main_token_distributions.clone(),
    };

    //check rate sum
    let mut sum = Decimal::zero();
    
    for distribution in msg.main_token_distributions {
      sum = sum + distribution.rate
    }

    if sum != Decimal::one() {
      return Err(StdError::generic_err("sum of rate must be queal to 1"));
    }

    self.config.save(deps.storage, &config)?;

    Ok(Response::new())
  }

  pub fn execute(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
  ) -> StdResult<Response> {
    match msg {
      ExecuteMsg::Receive(msg) => self.receive_cw20(deps, env, info, msg),
      ExecuteMsg::UpdateConfig { 
        token_code_id,
        main_token,
        owner,
        main_token_distributions
      } => self.update_config(deps, env, info, token_code_id, main_token, owner, main_token_distributions),
      ExecuteMsg::RegisterBetaInvitation {
        soft_cap,
        hard_cap,
        invitation_price,
        start_time,
        end_time,
        game_token_info,
        fan_token_info,
        game_token_distributions,
      } => self.register_beta_invitation(
        deps,
        env,
        info,
        soft_cap,
        hard_cap,
        invitation_price,
        start_time,
        end_time,
        game_token_info,
        fan_token_info,
        game_token_distributions,
      ),
      ExecuteMsg::Claim {
        game_token,
      } => self.claim(deps, env, info, game_token),
      ExecuteMsg::Refund {
        game_token,
        refund_amount
      } => self.refund(deps, env, info, game_token, refund_amount),
      ExecuteMsg::TokenDistribute {
        game_token
      } => self.token_distribute(deps, env, info, game_token)
    }
  }
}

impl<'a> BetaInvitation<'a> {
  pub fn receive_cw20(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
  ) -> StdResult<Response> {
    let contract_addr = info.sender.clone();
    let amount = cw20_msg.amount;
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;

    match from_binary(&cw20_msg.msg) {
      Ok(Cw20HookMsg::BuyBetaInvitation{
        game_token,
        beta_invitation_amount,
      }) => {
        let config = self.config.load(deps.storage)?;

        if config.main_token != contract_addr {
          return Err(StdError::generic_err("wrong token given"));
        }
        
        self.buy_beta_invitation(deps, env, sender, game_token, beta_invitation_amount, amount)
      }
      Err(err) => Err(err),
    }

  }

  pub fn update_config(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_code_id: Option<u64>,
    main_token: Option<Addr>,
    owner: Option<Addr>,
    main_token_distributions: Option<Vec<Distribution>>,
  ) -> StdResult<Response> {
    let mut config = self.config.load(deps.storage)?;
    // only owner can execute
    if config.owner != info.sender {
      return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(token_code_id) = token_code_id {
      config.token_code_id = token_code_id
    }

    if let Some(main_token) = main_token {
      config.main_token = main_token
    }

    if let Some(owner) = owner {
      config.owner = owner
    }

    if let Some(main_token_distributions) = main_token_distributions {
      config.main_token_distributions = main_token_distributions
    }

    Ok(Response::new()
      .add_attribute("action", "update_config")
    )
  }

  pub fn register_beta_invitation(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    soft_cap: u64,
    hard_cap: u64,
    invitation_price: Uint128,
    start_time: u64,
    end_time: u64,
    game_token_info: Cw20Info,
    fan_token_info: Cw20Info,
    game_token_distributions: GameTokenDistributions,
  ) -> StdResult<Response> {
    // validations check
    if end_time <= start_time {
      return Err(StdError::generic_err("end_time must be larger than start_time"));
    }

    if start_time < env.block.time.seconds() {
      return Err(StdError::generic_err("start_time must be smaller than current time"));
    }

    if hard_cap <= soft_cap {
      return Err(StdError::generic_err("hard_cap must be larger than soft_cap"));
    }

    let mut game_token_distribution_sum = game_token_distributions.invitation_buyer.clone();
    for distribution in game_token_distributions.others.clone() {
      game_token_distribution_sum = game_token_distribution_sum + distribution.amount;
    }

    if let Some(total_supply) = game_token_info.total_supply {
      if game_token_distribution_sum != total_supply {
        return Err(StdError::generic_err("distribution sum is not equal to total_supply"));
      }
    } else {
      return Err(StdError::generic_err("total_supply of game_token must be given"));
    }



    let temp_invitation_info = InvitationInfo {
      // set all address to sender for temporary.
      game_token: info.sender.clone(),
      fan_token: info.sender.clone(),
      soft_cap,
      hard_cap,
      sold_amount: 0u64,
      start_time,
      end_time,
      invitation_price,
      game_token_distributions,
      main_token_distributed: false
    };

    // save temp data
    self.temp_invitation_info.save(deps.storage, &temp_invitation_info)?;

    let config = self.config.load(deps.storage)?;

    Ok(Response::new()
      .add_attribute("action", "register_beta_invitation")
      .add_attribute("sender", info.sender)
      // instantiate game token
      .add_submessage(SubMsg {
        id: 1,
        gas_limit: None,
        msg: WasmMsg::Instantiate {
          code_id: config.token_code_id,
          funds: vec![],
          admin: None,
          label: "".to_string(),
          msg: to_binary(&Cw20InstantiateMsg {
            name: game_token_info.name,
            symbol: game_token_info.symbol,
            decimals: game_token_info.decimals.unwrap_or(DEFAULT_DECIMALS),
            mint: Some(MinterResponse {
              minter: env.contract.address.to_string(),
              cap: Some(Uint128::from(game_token_info.total_supply.unwrap())),
            }),
            initial_balances: vec![],
            marketing: None
          })?
        }.into(),
        reply_on: ReplyOn::Success
      })
      // instantiate fan token
      .add_submessage(SubMsg {
        id: 2,
        gas_limit: None,
        msg: WasmMsg::Instantiate {
          code_id: config.token_code_id,
          funds: vec![],
          admin: None,
          label: "".to_string(),
          msg: to_binary(&Cw20InstantiateMsg {
            name: fan_token_info.name,
            symbol: fan_token_info.symbol,
            decimals: 0,
            mint: Some(MinterResponse {
              minter: env.contract.address.to_string(),
              cap: Some(Uint128::from(hard_cap)),
            }),
            initial_balances: vec![],
            marketing: None
          })?
        }.into(),
        reply_on: ReplyOn::Success
      })
    )
  }

  pub fn buy_beta_invitation(
    &self,
    deps: DepsMut,
    env: Env,
    sender: Addr,
    game_token: Addr,
    buy_amount: u64,
    main_token_amount: Uint128,
  ) -> StdResult<Response> {
    let user_state_key = self.gen_user_state_key(game_token.clone(), sender.clone());
    let user_state = self.user_states.may_load(deps.storage, user_state_key.clone())?;

    let mut invitation_info = self.invitation_info.load(deps.storage, game_token.clone())?;

    // check sent amount
    if invitation_info.invitation_price * Uint128::from(buy_amount) != main_token_amount {
      return Err(StdError::generic_err("worng amount given"));
    }

    if invitation_info.hard_cap < invitation_info.sold_amount + buy_amount {
      return Err(StdError::generic_err("exceed hard cap"));
    }

    if invitation_info.start_time > env.block.time.seconds() {
      return Err(StdError::generic_err("invitation is not started"));
    }

    if invitation_info.end_time < env.block.time.seconds() {
      return Err(StdError::generic_err("invitation ended"));
    }

    if let Some(mut user_state) = user_state {
      user_state.bought_invitation_amount = user_state.bought_invitation_amount + buy_amount;
      self.user_states.save(deps.storage, user_state_key, &user_state)?;
    } else {
      let new_user_state = UserState {
        address: sender.clone(),
        game_token: game_token.clone(),
        bought_invitation_amount: buy_amount,
        claimed: false,
      };
      self.user_states.save(deps.storage, user_state_key, &new_user_state)?;
    }

    // update sold_amount
    invitation_info.sold_amount = invitation_info.sold_amount + buy_amount;

    self.invitation_info.save(deps.storage, game_token.clone(), &invitation_info)?;

    Ok(
      Response::new()
      .add_attribute("action", "buy_beta_invitation")
      .add_attribute("sender", sender)
      .add_attribute("game_token", game_token)
      .add_attribute("amount", buy_amount.to_string())
    )
  }

  pub fn claim(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    game_token: Addr,
  ) -> StdResult<Response> {
    let user_state_key = self.gen_user_state_key(game_token.clone(), info.sender.clone());
    let mut user_state = self.user_states.load(deps.storage, user_state_key.clone())?;
    let invitation_info = self.invitation_info.load(deps.storage, game_token.clone())?;

    if invitation_info.end_time > env.block.time.seconds() {
      return Err(StdError::generic_err("invitation is not ended, can't claimed"));
    }

    let passed = if invitation_info.soft_cap < invitation_info.sold_amount {
      true
    } else {
      false
    };

    let mut messages: Vec<CosmosMsg> = vec![];

    // if invitation is not passed refund main token
    if !passed {
      if user_state.claimed == true {
        return Err(StdError::generic_err("already refunded"));
      }

      let refund_amount = Uint128::from(user_state.bought_invitation_amount) * invitation_info.invitation_price;
      let config = self.config.load(deps.storage)?;
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute{
        contract_addr: config.main_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
          recipient: info.sender.to_string(),
          amount: refund_amount,
        })?,
        funds: vec![]
      }));

      user_state.claimed = true;
    // else mint fan token and transfer game token
    } else {
      let game_token_amount = invitation_info.game_token_distributions.invitation_buyer
        * Decimal::from_ratio(user_state.bought_invitation_amount, invitation_info.sold_amount);
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute{
        contract_addr: game_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
          recipient: info.sender.to_string(),
          amount: game_token_amount,
        })?,
        funds: vec![]
      }));

      messages.push(CosmosMsg::Wasm(WasmMsg::Execute{
        contract_addr: invitation_info.fan_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
          recipient: info.sender.to_string(),
          amount: Uint128::from(user_state.bought_invitation_amount),
        })?,
        funds: vec![]
      }));
    }

    self.user_states.save(deps.storage, user_state_key, &user_state)?;
    

    Ok(
      Response::new()
      .add_attribute("action", "claim")
      .add_attribute("sender", info.sender.to_string())
      .add_attribute("game_token", game_token)
      .add_messages(messages)
    )
  }

  pub fn refund(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    game_token: Addr,
    refund_amount: u64,
  ) -> StdResult<Response> {
    let user_state_key = self.gen_user_state_key(game_token.clone(), info.sender.clone());
    let mut user_state = self.user_states.load(deps.storage, user_state_key.clone())?;
    let mut invitation_info = self.invitation_info.load(deps.storage, game_token.clone())?;

    if invitation_info.end_time < env.block.time.seconds() {
      return Err(StdError::generic_err("invitation is ended, use claim to refund"));
    }

    if user_state.bought_invitation_amount < refund_amount {
      return Err(StdError::generic_err("can not refund more than you bought"));
    }

    let mut messages: Vec<CosmosMsg> = vec![];


    let refund_main_token_amount = Uint128::from(refund_amount) * invitation_info.invitation_price;
    let config = self.config.load(deps.storage)?;
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute{
      contract_addr: config.main_token.to_string(),
      msg: to_binary(&Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: refund_main_token_amount,
      })?,
      funds: vec![]
    }));

    user_state.bought_invitation_amount = user_state.bought_invitation_amount - refund_amount ;

    invitation_info.sold_amount = invitation_info.sold_amount - refund_amount;
    
    if user_state.bought_invitation_amount == 1 {
      self.user_states.remove(deps.storage, user_state_key)?;
    } else {
      self.user_states.save(deps.storage, user_state_key, &user_state)?;
    }
    self.invitation_info.save(deps.storage, game_token.clone(), &invitation_info)?;

    Ok(
      Response::new()
      .add_attribute("action", "refund")
      .add_attribute("sender", info.sender.to_string())
      .add_attribute("game_token", game_token)
      .add_attribute("amount", refund_amount.to_string())
      .add_messages(messages)
    )
  }

  pub fn token_distribute(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    game_token: Addr,
  ) -> StdResult<Response> {
    let config = self.config.load(deps.storage)?;
    if info.sender != config.owner {
      return Err(StdError::generic_err("unauthorized"));
    }

    let mut invitation_info = self.invitation_info.load(deps.storage, game_token.clone())?;

    if invitation_info.end_time > env.block.time.seconds() {
      return Err(StdError::generic_err("invitation is not ended, can't distributed"));
    }
    if invitation_info.main_token_distributed {
      return Err(StdError::generic_err("already distributed"));
    }

    let total_distribute_amount = invitation_info.invitation_price * Uint128::from(invitation_info.sold_amount);
    let mut messages: Vec<CosmosMsg> = vec![];

    for distribution in config.main_token_distributions {
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute{
        contract_addr: config.main_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
          recipient: distribution.address.to_string(),
          amount: total_distribute_amount * distribution.rate ,
        })?,
        funds: vec![]
      }));
    }

    for distribution in invitation_info.game_token_distributions.others.clone() {
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute{
        contract_addr: game_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
          recipient: distribution.address.to_string(),
          amount: distribution.amount ,
        })?,
        funds: vec![]
      }));
    }

    invitation_info.main_token_distributed = true;
    self.invitation_info.save(deps.storage, game_token.clone(), &invitation_info)?;

    Ok(
      Response::new()
      .add_attribute("action", "main_token_distribute")
      .add_attribute("game_token", game_token)
      .add_messages(messages)
    )
  }
}

impl<'a> BetaInvitation<'a> {
  pub fn reply(&self, deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let mut temp_invitation_info = self.temp_invitation_info.load(deps.storage)?;
    let res: MsgInstantiateContractResponse =
    Message::parse_from_bytes(msg.result.unwrap().data.unwrap().as_slice()).map_err(|_| {
      StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
    })?;

    match msg.id {
      1 => {
        let gmae_token_addr = res.get_contract_address();
        temp_invitation_info.game_token = deps.api.addr_validate(gmae_token_addr)?;
        self.temp_invitation_info.save(deps.storage, &temp_invitation_info)?;
        Ok(Response::new())
      },
      2 => {
        let fan_token_addr = res.get_contract_address();
        temp_invitation_info.fan_token = deps.api.addr_validate(fan_token_addr)?;
        // self.temp_invitation_info.save(deps.storage, &temp_invitation_info)?; //do not need to save the temp at last

        // save it to invitation_info
        self.invitation_info.save(deps.storage, temp_invitation_info.game_token.clone(), &temp_invitation_info)?;
        Ok(Response::new())
      }
      _ => return Err(StdError::generic_err("never_get_this"))
    }
  }
}
