use crate::state::{BetaInvitation, Config, GameTokenDistributions, GameTokenDistribution, InvitationInfo};
use crate::msgs::{InstantiateMsg, Distribution, ExecuteMsg, Cw20Info, Cw20InstantiateMsg};

use cosmwasm_std::{to_binary, Addr, Decimal, StdError, SubMsg, WasmMsg, ReplyOn, Timestamp, Uint128};
use cosmwasm_std::testing::{mock_dependencies, mock_info, mock_env};
use cw20::MinterResponse;

#[test]
fn instantiate_test() {
  let contract = BetaInvitation::default();

  let mut deps = mock_dependencies(&[]);
  let info = mock_info("owner", &[]);
  let env = mock_env();


  // case1. sum of rate < 1
  let mut instantiate_msg = InstantiateMsg {
    token_code_id: 123,
    main_token: Addr::unchecked("main_token"),
    owner: Addr::unchecked("owner"),
    main_token_distributions: vec![
      Distribution {
        address: Addr::unchecked("account_a"),
        rate: Decimal::from_ratio(5u128, 10u128)
      },
      Distribution {
        address: Addr::unchecked("account_b"),
        rate: Decimal::from_ratio(3u128, 10u128)
      }
    ]
  };

  let res = contract.instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg.clone());
  assert_eq!(res, Err(StdError::generic_err("sum of rate must be queal to 1")));

  // case2. sum of rate > 1
  instantiate_msg.main_token_distributions[0].rate = Decimal::from_ratio(10u128, 10u128);
  let res = contract.instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg.clone());
  assert_eq!(res, Err(StdError::generic_err("sum of rate must be queal to 1")));

  // case3. sum of rate == 1
  instantiate_msg.main_token_distributions[0].rate = Decimal::from_ratio(7u128, 10u128);
  let _res = contract.instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg.clone()).unwrap();
  let config = contract.config.load(&deps.storage).unwrap();
  assert_eq!(
    config,
    Config {
      owner: Addr::unchecked("owner"),
      main_token: Addr::unchecked("main_token"),
      token_code_id: 123,
      main_token_distributions: vec![
        Distribution {
          address: Addr::unchecked("account_a"),
          rate: Decimal::from_ratio(7u128, 10u128)
        },
        Distribution {
          address: Addr::unchecked("account_b"),
          rate: Decimal::from_ratio(3u128, 10u128)
        }
      ]
    }
  )
}



#[test]
fn register_test() {
  let contract = BetaInvitation::default();

  let mut deps = mock_dependencies(&[]);
  let info = mock_info("owner", &[]);
  let mut env = mock_env();
  env.block.time = Timestamp::from_seconds(99);


  // instantiate
  let instantiate_msg = InstantiateMsg {
    token_code_id: 123,
    main_token: Addr::unchecked("main_token"),
    owner: Addr::unchecked("owner"),
    main_token_distributions: vec![
      Distribution {
        address: Addr::unchecked("account_a"),
        rate: Decimal::from_ratio(5u128, 10u128)
      },
      Distribution {
        address: Addr::unchecked("account_b"),
        rate: Decimal::from_ratio(5u128, 10u128)
      }
    ]
  };

  let _res = contract.instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg.clone()).unwrap();

  let register_msg = ExecuteMsg::RegisterBetaInvitation {
    soft_cap: 10,
    hard_cap: 20,
    user_cap: 10,
    invitation_price: Uint128::from(100u128),
    start_time: 100,
    end_time: 200,
    game_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: Some(6),
      total_supply: Some(Uint128::from(1000u128))
    },
    fan_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: None,
      total_supply: None,
    },
    game_token_distributions: GameTokenDistributions {
      invitation_buyer: Uint128::from(500u128),
      others: vec![
        GameTokenDistribution { address: Addr::unchecked("treasury"), amount: Uint128::from(500u128) }
      ]
    }
  };

  // case1. start time is older than current time
  env.block.time = Timestamp::from_seconds(150);
  let res = contract.execute(deps.as_mut(), env.clone(), info.clone(), register_msg);
  assert_eq!(res, Err(StdError::generic_err("start_time msut be smaller than current time")));

  env.block.time = Timestamp::from_seconds(50);

  // case2. start time is larger thant end time
  let register_msg = ExecuteMsg::RegisterBetaInvitation {
    soft_cap: 10,
    hard_cap: 20,
    user_cap: 10,
    invitation_price: Uint128::from(100u128),
    start_time: 300,
    end_time: 200,
    game_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: Some(6),
      total_supply: Some(Uint128::from(1000u128))
    },
    fan_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: None,
      total_supply: None,
    },
    game_token_distributions: GameTokenDistributions {
      invitation_buyer: Uint128::from(500u128),
      others: vec![
        GameTokenDistribution { address: Addr::unchecked("treasury"), amount: Uint128::from(500u128) }
      ]
    }
  };

  let res = contract.execute(deps.as_mut(), env.clone(), info.clone(), register_msg);
  assert_eq!(res, Err(StdError::generic_err("end_time msut be larger than start_time")));

  // case3. soft_cap is larger than hard_cap
  let register_msg = ExecuteMsg::RegisterBetaInvitation {
    soft_cap: 30,
    hard_cap: 20,
    user_cap: 10,
    invitation_price: Uint128::from(100u128),
    start_time: 100,
    end_time: 200,
    game_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: Some(6),
      total_supply: Some(Uint128::from(1000u128))
    },
    fan_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: None,
      total_supply: None,
    },
    game_token_distributions: GameTokenDistributions {
      invitation_buyer: Uint128::from(500u128),
      others: vec![
        GameTokenDistribution { address: Addr::unchecked("treasury"), amount: Uint128::from(500u128) }
      ]
    }
  };

  let res = contract.execute(deps.as_mut(), env.clone(), info.clone(), register_msg);
  assert_eq!(res, Err(StdError::generic_err("hard_cap msut be larger than soft_cap")));

  // case4. distribute sum mismatch
  let register_msg = ExecuteMsg::RegisterBetaInvitation {
    soft_cap: 10,
    hard_cap: 20,
    user_cap: 10,
    invitation_price: Uint128::from(100u128),
    start_time: 100,
    end_time: 200,
    game_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: Some(6),
      total_supply: Some(Uint128::from(1000u128))
    },
    fan_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: None,
      total_supply: None,
    },
    game_token_distributions: GameTokenDistributions {
      invitation_buyer: Uint128::from(600u128),
      others: vec![
        GameTokenDistribution { address: Addr::unchecked("treasury"), amount: Uint128::from(500u128) }
      ]
    }
  };

  let res = contract.execute(deps.as_mut(), env.clone(), info.clone(), register_msg);
  assert_eq!(res, Err(StdError::generic_err("distribution sum is not equal to total_supply")));

  // case5. game token's total_supply is not given
  let register_msg = ExecuteMsg::RegisterBetaInvitation {
    soft_cap: 10,
    hard_cap: 20,
    user_cap: 10,
    invitation_price: Uint128::from(100u128),
    start_time: 100,
    end_time: 200,
    game_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: Some(6),
      total_supply: None
    },
    fan_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: None,
      total_supply: None,
    },
    game_token_distributions: GameTokenDistributions {
      invitation_buyer: Uint128::from(500u128),
      others: vec![
        GameTokenDistribution { address: Addr::unchecked("treasury"), amount: Uint128::from(500u128) }
      ]
    }
  };

  let res = contract.execute(deps.as_mut(), env.clone(), info.clone(), register_msg);
  assert_eq!(res, Err(StdError::generic_err("total_supply of game_token must be given")));



  let register_msg = ExecuteMsg::RegisterBetaInvitation {
    soft_cap: 10,
    hard_cap: 20,
    user_cap: 10,
    invitation_price: Uint128::from(100u128),
    start_time: 100,
    end_time: 200,
    game_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: Some(6),
      total_supply: Some(Uint128::from(1000u128))
    },
    fan_token_info: Cw20Info {
      name: "game".to_string(),
      symbol: "GAME".to_string(),
      decimals: None,
      total_supply: None,
    },
    game_token_distributions: GameTokenDistributions {
      invitation_buyer: Uint128::from(500u128),
      others: vec![
        GameTokenDistribution { address: Addr::unchecked("treasury"), amount: Uint128::from(500u128) }
      ]
    }
  };

  let res = contract.execute(deps.as_mut(), env.clone(), info.clone(), register_msg.clone()).unwrap();

  const DEFAULT_DECIMALS: u8 = 6;

  let (game_token_info, fan_token_info, hard_cap) = match register_msg {
    ExecuteMsg::RegisterBetaInvitation {
      soft_cap: _,
      hard_cap,
      user_cap: 10,
      invitation_price: _,
      start_time: _,
      end_time: _,
      game_token_distributions: _,
      game_token_info,
      fan_token_info,
    } => (game_token_info, fan_token_info, hard_cap),
    _ => panic!("should not enter"),
  };

  let config = contract.config.load(&deps.storage).unwrap();

  let reply_msg_0 = SubMsg {
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
      }).unwrap()
    }.into(),
    reply_on: ReplyOn::Success
  };

  let reply_msg_1 = SubMsg {
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
      }).unwrap()
    }.into(),
    reply_on: ReplyOn::Success
  };

  assert_eq!(res.messages , vec![reply_msg_0, reply_msg_1]);

  let temp_invitation_info = contract.temp_invitation_info.load(&deps.storage).unwrap();

  assert_eq!(
    temp_invitation_info,
    InvitationInfo {
      game_token: Addr::unchecked("owner"),
      fan_token: Addr::unchecked("owner"),
      soft_cap: 10,
      hard_cap: 20,
      user_cap: 10,
      sold_amount: 0,
      start_time:100,
      end_time: 200,
      invitation_price: Uint128::from(100u128),
      game_token_distributions: GameTokenDistributions {
        invitation_buyer: Uint128::from(500u128),
        others: vec![
          GameTokenDistribution { address: Addr::unchecked("treasury"), amount: Uint128::from(500u128) }
        ]
      },
      main_token_distributed: false
    }
  )
}

