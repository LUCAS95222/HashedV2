use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, CosmosMsg, Decimal256, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdError, StdResult, Uint128, Uint256, WasmMsg, QuerierWrapper, QueryRequest, WasmQuery
};
use cw2::set_contract_version;
use cw20::{BalanceResponse as CwBalanceRespose, Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20QueryMsg};
use cw_storage_plus::U64Key;

use crate::constants::WEEK_SECONDS;
use crate::msg::{
    Cw20HookMsg, EstimatedRewardResponse, ExecuteMsg, InstantiateMsg, LockUpInfoResponse,
    LockdropInfoResponse, MigrateMsg, QueryMsg, UserInfoResponse,
};
use crate::state::{Config, LockupInfo, State, CONFIG, LOCKUP_INFO, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "c2x-lp-lockdrop";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // CHECK :: init_timestamp needs to be valid
    if _env.block.time.seconds() > msg.init_timestamp {
        return Err(StdError::generic_err(format!(
            "Invalid init_timestamp. Current timestamp : {}",
            _env.block.time.seconds()
        )));
    }

    // CHECK :: min_lock_duration , max_lock_duration need to be valid (min_lock_duration < max_lock_duration)
    if msg.max_lock_duration < msg.min_lock_duration || msg.min_lock_duration == 0u16 {
        return Err(StdError::generic_err("Invalid Lockup durations"));
    }

    // CHECK ::multiplier cannot be 0
    if msg.weekly_multiplier == 0u16 {
        return Err(StdError::generic_err("Lockdrop multiplier cannot be 0"));
    }

    // CHECK ::divider cannot be 0
    if msg.weekly_divider == 0u64 {
        return Err(StdError::generic_err("Lockdrop divider cannot be 0"));
    }

    if msg.reward_vesting_duration == 0u64 {
        return Err(StdError::generic_err("Invalid reward release duration"));
    }

    if msg.event_duration == 0u32 {
        return Err(StdError::generic_err("Event duration cannot be 0"));
    }

    let config = Config {
        owner: info.sender,
        reward_token_addr: deps.api.addr_validate(&msg.reward_token_addr)?,
        lockable_token_addr: deps.api.addr_validate(&msg.lockable_token_addr)?,
        event_start_second: msg.init_timestamp,
        event_end_second: msg.init_timestamp + u64::from(msg.event_duration),
        reward_vesting_duration: msg.reward_vesting_duration,
        min_lock_duration: u64::from(msg.min_lock_duration),
        max_lock_duration: u64::from(msg.max_lock_duration),
        lock_duration_second: u64::from(msg.lock_duration_second.unwrap_or(WEEK_SECONDS)),
        weekly_multiplier: u64::from(msg.weekly_multiplier),
        weekly_divider: msg.weekly_divider,
        lockdrop_reward: Uint128::zero(),
        total_reward: msg.total_reward, //2022-04-25
    };

    let state = State {
        total_locked_lp_token: Uint128::zero(),
        total_weighted_score: Uint256::zero(),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", config.owner)
        .add_attribute("ctx_token_addr", config.reward_token_addr)
        .add_attribute("lp_token_addr", config.lockable_token_addr)
        .add_attribute("event_start_second", config.event_start_second.to_string())
        .add_attribute("event_end_second", config.event_end_second.to_string())
        .add_attribute("weekly_multiplier", config.weekly_multiplier.to_string())
        .add_attribute("weekly_divider", config.weekly_divider.to_string())
        .add_attribute("min_lock_duration", config.min_lock_duration.to_string())
        .add_attribute("max_lock_duration", config.max_lock_duration.to_string())
        .add_attribute("lockdrop_rewards", config.lockdrop_reward)
        .add_attribute("total_reward", config.total_reward))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, _env, info, msg),
        ExecuteMsg::Claim { duration } => handle_claim(deps, _env, info, duration),
        ExecuteMsg::Unlock { duration } => handle_unlock(deps, _env, info, duration),
        ExecuteMsg::UpdateReward { total_reward } => handle_update_reward(deps, _env, info, total_reward),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, StdError> {
    let user_address = deps.api.addr_validate(&cw20_msg.sender)?;
    let amount = cw20_msg.amount;

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::IncreaseReward {} => handle_increase_reward(deps, env, info, amount),
        Cw20HookMsg::IncreaseLockup { duration } => {
            handle_increase_lockup(deps, env, info, user_address, duration, amount)
        }
    }
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LockdropInfo {} => to_binary(&query_lockdrop_info(deps)?),
        QueryMsg::UserInfo { addr } => to_binary(&query_user_info(deps, &env, addr)?),
        QueryMsg::Estimate { amount, duration } => {
            to_binary(&query_estimate(deps, amount, duration)?)
        }
    }
}

/// @dev Facilitates increasing rewards that are to be distributed as Lockdrop participation reward
/// @params amount : Number of tokens which are to be added to current rewards
pub fn handle_increase_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.reward_token_addr {
        return Err(StdError::generic_err(format!(
            "Only {} tokens are received!",
            config.reward_token_addr
        )));
    }

    if env.block.time.seconds() >= config.event_end_second {
        return Err(StdError::generic_err("Distribution already started"));
    };

    config.lockdrop_reward = config.lockdrop_reward.checked_add(amount)?;

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "increase_lockdrop_reward"),
        attr("reward_token_addr", config.reward_token_addr),
        attr("token_amount", amount),
        attr("total_lockdrop_reward_amount", config.lockdrop_reward),
    ]))
}

pub fn handle_increase_lockup(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user_address: Addr,
    duration: u64,
    amount: Uint128,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let lp_token = info.sender;

    if config.lockable_token_addr != lp_token {
        return Err(StdError::generic_err(format!(
            "only {} token is allowed",
            config.lockable_token_addr
        )));
    }

    let current_time = env.block.time.seconds();
    if current_time < config.event_start_second || current_time >= config.event_end_second {
        return Err(StdError::generic_err("Staking event closed"));
    }

    if duration < config.min_lock_duration || duration > config.max_lock_duration {
        return Err(StdError::generic_err(format!(
            "Lockup duration needs to be between {} and {}",
            config.min_lock_duration, config.max_lock_duration
        )));
    }

    let lockup_key = (&user_address, U64Key::new(duration));
    let mut weighted_score = Uint256::zero();

    let li = LOCKUP_INFO.update::<_, StdError>(deps.storage, lockup_key, |li| {
        if let Some(mut li) = li {
            li.locked_lp_token = li.locked_lp_token.checked_add(amount)?;
            state.total_locked_lp_token = state.total_locked_lp_token.checked_add(amount)?;

            let prev_score = li.weighted_score;
            weighted_score = calculate_weighted_score(li.locked_lp_token, duration, &config);
            state.total_weighted_score = state.total_weighted_score.checked_sub(prev_score)?;
            state.total_weighted_score = state.total_weighted_score.checked_add(weighted_score)?;
            li.weighted_score = weighted_score;

            Ok(li)
        } else {
            weighted_score = calculate_weighted_score(amount, duration, &config);
            state.total_locked_lp_token = state.total_locked_lp_token.checked_add(amount)?;
            state.total_weighted_score = state.total_weighted_score.checked_add(weighted_score)?;

            Ok(LockupInfo {
                locked_lp_token: amount,
                claimed: Uint128::zero(),
                unlock_timestamp: config.event_end_second
                    + (duration * config.lock_duration_second),
                weighted_score,
            })
        }
    })?;

    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "increase_lockup_position"),
        attr("user_addr", user_address),
        attr("duration", duration.to_string()),
        attr("locked_token_addr", lp_token),
        attr("token_amount", amount),
        attr("total_weighted_score", state.total_weighted_score),
        attr("user_weighted_score", li.weighted_score),
    ]))
}

pub fn handle_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    let user_address = info.sender;
    let lockup_key = (&user_address, U64Key::new(duration));
    let mut li = LOCKUP_INFO.load(deps.storage, lockup_key.clone())?;

    if current_time <= li.unlock_timestamp {
        return Err(StdError::generic_err(format!(
            "{} seconds to claim",
            li.unlock_timestamp - current_time
        )));
    }

    //2022-04-25
    let reward_token_balance = query_balance(&deps.querier, &env, config.reward_token_addr.to_string())?;
    if reward_token_balance < config.total_reward {
        return Err(StdError::generic_err(format!(
            "Insufficient lockdrop reward balance:{}",
            (config.total_reward - reward_token_balance)
        )));
    }

    let lockup_reward = calculate_reward_for_lockup(
        state.total_weighted_score,
        li.weighted_score,
        config.total_reward,    //config.lockdrop_reward,
    );

    if lockup_reward == Uint128::zero() {
        return Err(StdError::generic_err("Nothing to Claim"));
    }

    let claimable = calculate_claimable(
        current_time,
        li.unlock_timestamp,
        config.reward_vesting_duration,
        lockup_reward,
        li.claimed,
    );

    if claimable == Uint128::zero() {
        return Err(StdError::generic_err("Nothing to Claim"));
    }

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.reward_token_addr.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: user_address.to_string(),
            amount: claimable,
        })?,
        funds: vec![],
    });
    li.claimed = li.claimed.checked_add(claimable)?;

    LOCKUP_INFO.save(deps.storage, lockup_key, &li)?;

    Ok(Response::new().add_message(msg).add_attributes(vec![
        attr("action", "claim"),
        attr("user_addr", user_address),
        attr("duration", duration.to_string()),
        attr("claimed_token_addr", config.reward_token_addr),
        attr("token_amount", claimable),
    ]))
}

pub fn handle_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
) -> StdResult<Response> {
    let current_time = env.block.time.seconds();
    let user_address = info.sender;
    let lockup_key = (&user_address, U64Key::new(duration));
    let mut li = LOCKUP_INFO.load(deps.storage, lockup_key.clone())?;

    if current_time <= li.unlock_timestamp {
        return Err(StdError::generic_err(format!(
            "{} seconds to unlock",
            li.unlock_timestamp - env.block.time.seconds()
        )));
    }

    if li.locked_lp_token == Uint128::zero() {
        return Err(StdError::generic_err("Already Unlocked"));
    }

    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let amount = li.locked_lp_token;

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.lockable_token_addr.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: user_address.to_string(),
            amount,
        })?,
        funds: vec![],
    });

    state.total_locked_lp_token = state.total_locked_lp_token.checked_sub(amount)?;
    li.locked_lp_token = Uint128::zero();

    LOCKUP_INFO.save(deps.storage, lockup_key, &li)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_message(msg).add_attributes(vec![
        attr("action", "unlock"),
        attr("user_addr", user_address),
        attr("duration", duration.to_string()),
        attr("unlocked_token_addr", config.lockable_token_addr),
        attr("token_amount", amount),
    ]))
}

pub fn handle_update_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    total_reward: Uint128,
) -> StdResult<Response> { 

    let mut config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(StdError::generic_err("UnAuthorized"));
    }

    if env.block.time.seconds() >= config.event_end_second {
        return Err(StdError::generic_err("Distribution already started"));
    };

    config.total_reward = total_reward;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
      .add_attribute("action", "update_reward")
      .add_attribute("sender", info.sender)
      .add_attribute("total_reward", total_reward.clone()))

}

fn calculate_weighted_score(amount: Uint128, duration: u64, config: &Config) -> Uint256 {
    let lock_weight = Decimal256::one()
        + Decimal256::from_ratio(
            (duration - 1) * config.weekly_multiplier,
            config.weekly_divider,
        );
    lock_weight * amount.into()
}

pub fn calculate_reward_for_lockup(
    total_weighted_score: Uint256,
    lockup_weighted_score: Uint256,
    total_lockdrop_reward: Uint128,
) -> Uint128 {
    if total_weighted_score == Uint256::zero() {
        Uint128::zero()
    } else {
        (Decimal256::from_ratio(lockup_weighted_score, total_weighted_score)
            * total_lockdrop_reward.into())
        .try_into()
        .unwrap()
    }
}

pub fn calculate_claimable(
    current_time: u64,
    unlock_timestamp: u64,
    reward_vesting_duration: u64,
    reward: Uint128,
    claimed: Uint128,
) -> Uint128 {
    if current_time <= unlock_timestamp {
        return Uint128::zero();
    }

    let time_elapsed = current_time - unlock_timestamp;
    let ratio = if time_elapsed >= reward_vesting_duration {
        Decimal256::one()
    } else {
        Decimal256::from_ratio(time_elapsed, reward_vesting_duration)
    };

    let reward = Uint256::from(reward) * ratio;
    let claimable = reward - Uint256::from(claimed);

    claimable.try_into().unwrap()
}

pub fn query_lockdrop_info(deps: Deps) -> StdResult<LockdropInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    Ok(LockdropInfoResponse {
        ctx_token_addr: config.reward_token_addr,
        lp_token_addr: config.lockable_token_addr,
        event_start_second: config.event_start_second,
        event_end_second: config.event_end_second,
        reward_vesting_duration: config.reward_vesting_duration,
        min_lock_duration: config.min_lock_duration,
        max_lock_duration: config.max_lock_duration,
        total_locked_lp_token: state.total_locked_lp_token,
        total_lockdrop_reward: config.total_reward, //config.lockdrop_reward,
        total_weighted_score: state.total_weighted_score,
    })
}

pub fn query_user_info(deps: Deps, env: &Env, user: String) -> StdResult<UserInfoResponse> {
    let user_address = deps.api.addr_validate(&user)?;

    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    let mut lockup_infos = vec![];
    let mut user_total_locked_lp_token = Uint128::zero();
    let mut user_total_reward = Uint128::zero();

    for duration in LOCKUP_INFO
        .prefix(&user_address)
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|v| u64::from_be_bytes(v.try_into().expect("Duration deserialization error!")))
    {
        let li = LOCKUP_INFO.load(deps.storage, (&user_address, U64Key::from(duration)))?;
        let current_time = env.block.time.seconds();
        let lockup_reward = calculate_reward_for_lockup(
            state.total_weighted_score,
            li.weighted_score,
            config.total_reward,    //config.lockdrop_reward,
        );

        user_total_locked_lp_token = user_total_locked_lp_token.checked_add(li.locked_lp_token)?;
        user_total_reward = user_total_reward.checked_add(lockup_reward)?;

        lockup_infos.push(LockUpInfoResponse {
            unlock_second: li.unlock_timestamp,
            claimed: li.claimed,
            duration,
            claimable: calculate_claimable(
                current_time,
                li.unlock_timestamp,
                config.reward_vesting_duration,
                lockup_reward,
                li.claimed,
            ),
            weighted_score: li.weighted_score,
            locked_lp_token: li.locked_lp_token,
            total_reward: lockup_reward,
        });
    }

    Ok(UserInfoResponse {
        lockdrop_total_reward: config.total_reward, //config.lockdrop_reward,
        lockdrop_total_locked_lp_token: state.total_locked_lp_token,
        lp_token_addr: config.lockable_token_addr,
        lockup_infos,
        user_total_locked_lp_token,
        user_total_reward,
    })
}

pub fn query_estimate(
    deps: Deps,
    amount: Uint128,
    duration: u64,
) -> StdResult<EstimatedRewardResponse> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    if duration < config.min_lock_duration || duration > config.max_lock_duration {
        return Err(StdError::generic_err(format!(
            "Lockup duration needs to be between {} and {}",
            config.min_lock_duration, config.max_lock_duration
        )));
    }

    let total_lockdrop_reward = config.total_reward;    //config.lockdrop_reward;
    let weighted_score = calculate_weighted_score(amount, duration, &config);
    let estimated_reward = calculate_reward_for_lockup(
        state.total_weighted_score.checked_add(weighted_score)?,
        weighted_score,
        total_lockdrop_reward,
    );

    Ok(EstimatedRewardResponse {
        lockdrop_total_reward: total_lockdrop_reward,
        estimated_reward,
    })
}

//2022-04-25
pub fn query_balance(
    querier: &QuerierWrapper, 
    env: &Env,
    token_contract: String,
  ) -> StdResult<Uint128> {
    let res: CwBalanceRespose = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
      contract_addr: token_contract,
      msg: to_binary(&Cw20QueryMsg::Balance {
        address: env.contract.address.to_string(),
      })?,
    }))?;

    Ok(res.balance)
  }

#[cfg(test)]
mod unit_tests {

    use super::*;
    use crate::constants::DAY_SECONDS;

    #[test]
    fn test_calculate_claimable() {
        struct TestSet {
            pub current_time: u64,
            pub unlock_timestamp: u64,
            pub reward_vesting_duration: u64,
            pub total_reward: Uint128,
            pub claimed: Uint128,
            pub expect: Uint128,
        }

        let sets = [
            TestSet {
                current_time: 0,
                unlock_timestamp: 0,
                reward_vesting_duration: 10,
                claimed: Uint128::from(0u64),
                total_reward: Uint128::from(10u64),
                expect: Uint128::zero(),
            },
            TestSet {
                current_time: 200,
                unlock_timestamp: 0,
                reward_vesting_duration: 10,
                claimed: Uint128::from(0u64),
                total_reward: Uint128::from(10u64),
                expect: Uint128::from(10u64),
            },
            TestSet {
                current_time: DAY_SECONDS as u64,
                unlock_timestamp: 0,
                reward_vesting_duration: WEEK_SECONDS as u64,
                claimed: Uint128::from(0u64),
                total_reward: Uint128::from(10000000u64),
                expect: Uint128::from(1428571u64),
            },
        ];

        for (idx, set) in sets.iter().enumerate() {
            let claimable = calculate_claimable(
                set.current_time,
                set.unlock_timestamp,
                set.reward_vesting_duration,
                set.total_reward,
                set.claimed,
            );
            assert_eq!(claimable, set.expect, "case {}: failed", idx);
        }
    }
}
