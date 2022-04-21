#[cfg(not(feature = "library"))]
use std::convert::TryFrom;
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Attribute, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdError, StdResult, Uint128, WasmMsg,
};

use serde_json::to_string;

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Denom};
use cw_storage_plus::Bound;

use crate::msg::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, VestingAccountResponse, VestingData,
    VestingSchedule,
};
use crate::state::{denom_to_key, VestingAccount, VESTING_ACCOUNTS};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::RegisterVestingAccount {
            master_address,
            addresses,
            vesting_key,
            vesting_schedule,
        } => {
            // deposit validation
            if info.funds.len() != 1 {
                return Err(StdError::generic_err("must deposit only one type of token"));
            }

            let deposit_coin = info.funds[0].clone();
            register_vesting_account(
                deps,
                env,
                master_address,
                addresses,
                vesting_key,
                Denom::Native(deposit_coin.denom),
                deposit_coin.amount,
                vesting_schedule,
            )
        }
        ExecuteMsg::DeregisterVestingAccount {
            addresses,
            vesting_key,
            vested_token_recipient,
            left_vesting_token_recipient,
        } => deregister_vesting_account(
            deps,
            env,
            info,
            addresses,
            vesting_key,
            vested_token_recipient,
            left_vesting_token_recipient,
        ),
        ExecuteMsg::Claim {
            vesting_keys, recipient
        } => claim(
            deps, env, info, vesting_keys, recipient
        ),
    }
}

fn register_vesting_account(
    deps: DepsMut,
    env: Env,
    master_address: Option<String>,
    addresses: Vec<String>,
    vesting_key: String,
    deposit_denom: Denom,
    deposit_amount: Uint128,
    vesting_schedule: VestingSchedule,
) -> StdResult<Response> {

    // vesting_account existence check
    for address in addresses.iter() {
        if VESTING_ACCOUNTS.has(deps.storage, (address.as_str(), vesting_key.as_str())) {
            return Err(StdError::generic_err(format!(
                "{} of \"{}\" already exists",
                address.to_string(), vesting_key.to_string()
            )));
        }
    }

    // validate vesting schedule
    let count = Uint128::from(u128::try_from(addresses.len()).unwrap());
    match vesting_schedule.clone() {
        VestingSchedule::LinearVesting {
            start_time,
            end_time,
            vesting_amount,
        } => {
            if vesting_amount.is_zero() {
                return Err(StdError::generic_err("assert(vesting_amount > 0)"));
            }

            let start_time = start_time
                .parse::<u64>()
                .map_err(|_| StdError::generic_err("invalid start_time"))?;

            let end_time = end_time
                .parse::<u64>()
                .map_err(|_| StdError::generic_err("invalid end_time"))?;

            if start_time < env.block.time.seconds() {
                return Err(StdError::generic_err(format!("assert(start_time < block_time), block_time = {}", env.block.time.seconds().to_string())));
            }

            if end_time <= start_time {
                return Err(StdError::generic_err("assert(end_time > start_time)"));
            }

            let vesting_amount = vesting_amount.checked_mul(count)?;
            if vesting_amount != deposit_amount {
                return Err(StdError::generic_err(format!(
                    "assert(deposit_amount == vesting_amount), required deposit_amount = {}",
                    vesting_amount.to_string()
                )));
            }
        }
        VestingSchedule::PeriodicVesting {
            start_time,
            end_time,
            vesting_interval,
            amount,
        } => {
            if amount.is_zero() {
                return Err(StdError::generic_err(
                    "cannot make zero token vesting account",
                ));
            }

            let start_time = start_time
                .parse::<u64>()
                .map_err(|_| StdError::generic_err("invalid start_time"))?;

            let end_time = end_time
                .parse::<u64>()
                .map_err(|_| StdError::generic_err("invalid end_time"))?;

            let vesting_interval = vesting_interval
                .parse::<u64>()
                .map_err(|_| StdError::generic_err("invalid vesting_interval"))?;

            if start_time < env.block.time.seconds() {
                return Err(StdError::generic_err(format!(
                    "invalid start_time, block_time = {}",
                    env.block.time.seconds().to_string()
                )));
            }

            if end_time <= start_time {
                return Err(StdError::generic_err("assert(end_time > start_time)"));
            }

            if vesting_interval == 0 {
                return Err(StdError::generic_err("assert(vesting_interval != 0)"));
            }

            let time_period = end_time - start_time;
            if time_period != (time_period / vesting_interval) * vesting_interval {
                return Err(StdError::generic_err(
                    "assert((end_time - start_time) % vesting_interval == 0)",
                ));
            }

            let num_interval = 1 + time_period / vesting_interval;
            let vesting_amount = amount.checked_mul(Uint128::from(num_interval))?.checked_mul(count)?;
            if vesting_amount != deposit_amount {
                return Err(StdError::generic_err(format!(
                    "assert(deposit_amount = amount * ((end_time - start_time) / vesting_interval + 1) * number of addresses), required deposit_amount = {}",
                    vesting_amount.to_string()
                )));
            }
        },
        VestingSchedule::ConditionalVesting {
            start_time,
            end_time,
            amount,
            condition,
        } => {
            if amount.is_zero() {
                return Err(StdError::generic_err(
                    "cannot make zero token vesting account",
                ));
            }

            let start_time = start_time
                .parse::<u64>()
                .map_err(|_| StdError::generic_err("invalid start_time"))?;

            let end_time = end_time
                .parse::<u64>()
                .map_err(|_| StdError::generic_err("invalid end_time"))?;

            if start_time < env.block.time.seconds() {
                return Err(StdError::generic_err(format!("invalid start_time, block_time = {}", env.block.time.seconds().to_string())));
            }

            if end_time <= start_time {
                return Err(StdError::generic_err("assert(end_time > start_time)"));
            }

            let passed_count = vesting_schedule.vested_count(end_time);
            match condition.style.as_str() {
                "daily" | "weekly" | "monthly" | "yearly" => {
                    if passed_count.is_zero() {
                        return Err(StdError::generic_err("no condition matched from start_time to end_time"));
                    }
                },
                _ => {
                    return Err(StdError::generic_err("assert(condition.style == \"daily\" or \"weekly\" or \"monthly\" or \"yearly\")"));
                },
            }

            let vesting_amount = amount.checked_mul(passed_count)?.checked_mul(count)?;
            if vesting_amount != deposit_amount {
                return Err(StdError::generic_err(format!(
                    "assert(deposit_amount == amount * number of matched condition * number of addresses), required deposit_amount = {} * {} * {} = {})",
                    amount.to_string(), passed_count.to_string(), count.to_string(), vesting_amount.to_string()
                )));
            }
        }
    }

    let vesting_amount = deposit_amount.checked_div(count)?;
    for address in addresses.iter() {
        VESTING_ACCOUNTS.save(
            deps.storage,
            (address.as_str(), vesting_key.as_str()),
            &VestingAccount {
                master_address: master_address.clone(),
                address: address.to_string(),
                vesting_key: vesting_key.to_string(),
                vesting_denom: deposit_denom.clone(),
                vesting_amount: vesting_amount.clone(),
                vesting_schedule: vesting_schedule.clone(),
                claimed_amount: Uint128::zero(),
            },
        )?;
    }

    Ok(Response::new().add_attributes(vec![
        ("action", "register_vesting_account"),
        (
            "master_address",
            master_address.unwrap_or_default().as_str(),
        ),
        ("addresses", &to_string(&addresses).unwrap()),
        ("vesting_key", vesting_key.as_str()),
        ("vesting_denom", &to_string(&deposit_denom).unwrap()),
        ("vesting_amount", &deposit_amount.to_string()),
    ]))
}

fn deregister_vesting_account(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    addresses: Vec<String>,
    vesting_key: String,
    vested_token_recipient: Option<String>,
    left_vesting_token_recipient: Option<String>,
) -> StdResult<Response> {
    let sender = info.sender;

    let count = u32::try_from(addresses.len()).unwrap();
    let mut results: Vec<String> = vec![];
    let mut messages: Vec<CosmosMsg> = vec![];

    for address in addresses.iter() {
        // vesting_account existence check
        let account = VESTING_ACCOUNTS.may_load(deps.storage, (address.as_str(), vesting_key.as_str()))?;
        if account.is_none() {
            if count < 2 {
                return Err(StdError::generic_err(format!(
                    "vesting entry is not found the key \"{}\"",
                    vesting_key.to_string(),
                )));
            }
            results.push(String::from("Err: vesting entry is not found"));
            continue;
        }

        let account = account.unwrap();
        if account.master_address.is_none() || account.master_address.unwrap() != sender {
            if count < 2 {
                return Err(StdError::generic_err("unauthorized"));
            }
            results.push(String::from("Err: unauthorized"));
            continue;
        }

        // remove vesting account
        VESTING_ACCOUNTS.remove(deps.storage, (address.as_str(), vesting_key.as_str()));

        let vested_amount = account
            .vesting_schedule
            .vested_amount(env.block.time.seconds())?;
        let claimed_amount = account.claimed_amount;

        // transfer already vested but not claimed amount to
        // a account address or the given `vested_token_recipient` address
        let claimable_amount = vested_amount.checked_sub(claimed_amount)?;
        if !claimable_amount.is_zero() {
            let recipient = vested_token_recipient.clone().unwrap_or_else(|| address.to_string());
            let message: CosmosMsg = match account.vesting_denom.clone() {
                Denom::Native(denom) => BankMsg::Send {
                    to_address: recipient,
                    amount: vec![Coin {
                        denom,
                        amount: claimable_amount,
                    }],
                }
                .into(),
                Denom::Cw20(contract_addr) => WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient,
                        amount: claimable_amount,
                    })?,
                    funds: vec![],
                }
                .into(),
            };

            messages.push(message);
        }

        // transfer left vesting amount to owner or
        // the given `left_vesting_token_recipient` address
        let left_vesting_amount = account.vesting_amount.checked_sub(vested_amount)?;
        if !left_vesting_amount.is_zero() {
            let recipient = left_vesting_token_recipient.clone().unwrap_or_else(|| sender.to_string());
            let message: CosmosMsg = match account.vesting_denom.clone() {
                Denom::Native(denom) => BankMsg::Send {
                    to_address: recipient,
                    amount: vec![Coin {
                        denom,
                        amount: left_vesting_amount,
                    }],
                }
                .into(),
                Denom::Cw20(contract_addr) => WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient,
                        amount: left_vesting_amount,
                    })?,
                    funds: vec![],
                }
                .into(),
            };

            messages.push(message);
        }

        results.push(String::from(format!(
            "Ok: denom = \"{}\", vesting amount = {}, vested amount = {}, left vesting amount = {}",
            &to_string(&account.vesting_denom).unwrap(),
            &account.vesting_amount.to_string(),
            &vested_amount.to_string(),
            &left_vesting_amount.to_string()
        )));
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "deregister_vesting_account"),
        ("addresses", &to_string(&addresses).unwrap()),
        ("results", &to_string(&results).unwrap()),
        ("vesting_key", vesting_key.as_str()),
    ]))
}

fn claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vesting_keys: Vec<String>,
    recipient: Option<String>,
) -> StdResult<Response> {
    let sender = info.sender;
    let recipient = recipient.unwrap_or_else(|| sender.to_string());
    let mut count = 0;

    for vest_key in vesting_keys.iter() {
        // vesting_account existence check
        let account = VESTING_ACCOUNTS.may_load(deps.storage, (sender.as_str(), vest_key.as_str()))?;
        if account.is_none() {
            return Err(StdError::generic_err(format!(
                "vesting entry is not found the key \"{}\"",
                vest_key.to_string(),
            )));
        }

        let account = account.unwrap();
        let vested_amount = account
            .vesting_schedule
            .vested_amount(env.block.time.seconds())?;
        let claimed_amount = account.claimed_amount;

        let claimable_amount = vested_amount.checked_sub(claimed_amount)?;
        if claimable_amount.is_zero() {
            continue;
        }

        count += 1;
    }
    if count < 1 {
        return Err(StdError::generic_err("no claimable amount for now"));
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut attrs: Vec<Attribute> = vec![];
    for vest_key in vesting_keys.iter() {
        // vesting_account existence check
        let account = VESTING_ACCOUNTS.may_load(deps.storage, (sender.as_str(), vest_key.as_str()))?;
        if account.is_none() {
            return Err(StdError::generic_err(format!(
                "vesting entry is not found for denomthe key {}",
                vest_key.to_string(),
            )));
        }

        let mut account = account.unwrap();
        let vested_amount = account
            .vesting_schedule
            .vested_amount(env.block.time.seconds())?;
        let claimed_amount = account.claimed_amount;

        let claimable_amount = vested_amount.checked_sub(claimed_amount)?;
        if claimable_amount.is_zero() {
            continue;
        }

        account.claimed_amount = vested_amount;
        if account.claimed_amount == account.vesting_amount {
            VESTING_ACCOUNTS.remove(deps.storage, (sender.as_str(), vest_key.as_str()));
        } else {
            VESTING_ACCOUNTS.save(deps.storage, (sender.as_str(), vest_key.as_str()), &account)?;
        }

        let message: CosmosMsg = match account.vesting_denom.clone() {
            Denom::Native(denom) => BankMsg::Send {
                to_address: recipient.clone(),
                amount: vec![Coin {
                    denom,
                    amount: claimable_amount,
                }],
            }
            .into(),
            Denom::Cw20(contract_addr) => WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.clone(),
                    amount: claimable_amount,
                })?,
                funds: vec![],
            }
            .into(),
        };

        messages.push(message);
        attrs.extend(
            vec![
                Attribute::new("vesting_key", &vest_key.to_string()),
                Attribute::new("vesting_denom", &to_string(&account.vesting_denom).unwrap()),
                Attribute::new("vesting_amount", &account.vesting_amount.to_string()),
                Attribute::new("vested_amount", &vested_amount.to_string()),
                Attribute::new("claim_amount", &claimable_amount.to_string()),
            ]
            .into_iter(),
        );
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![("action", "claim"), ("address", sender.as_str())])
        .add_attributes(attrs))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let amount = cw20_msg.amount;
    let _sender = cw20_msg.sender;
    let contract = info.sender;

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::RegisterVestingAccount {
            master_address,
            addresses,
            vesting_key,
            vesting_schedule,
        }) => register_vesting_account(
            deps,
            env,
            master_address,
            addresses,
            vesting_key,
            Denom::Cw20(contract),
            amount,
            vesting_schedule,
        ),
        Err(_) => Err(StdError::generic_err("invalid cw20 hook message")),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VestingAccount {
            address,
            start_after,
            limit,
        } => to_binary(&vesting_account(deps, env, address, start_after, limit)?),
    }
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
fn vesting_account(
    deps: Deps,
    env: Env,
    address: String,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<VestingAccountResponse> {
    let mut vestings: Vec<VestingData> = vec![];
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    for item in VESTING_ACCOUNTS
        .prefix(address.as_str())
        .range(
            deps.storage,
            start_after
                .map(denom_to_key)
                .map(|v| v.as_bytes().to_vec())
                .map(Bound::Exclusive),
            None,
            Order::Ascending,
        )
        .take(limit)
    {
        let (_, account) = item?;
        let vested_amount = account
            .vesting_schedule
            .vested_amount(env.block.time.seconds())?;

        vestings.push(VestingData {
            master_address: account.master_address,
            vesting_key: account.vesting_key,
            vesting_denom: account.vesting_denom,
            vesting_amount: account.vesting_amount,
            vested_amount,
            vesting_schedule: account.vesting_schedule,
            claimable_amount: vested_amount.checked_sub(account.claimed_amount)?,
        })
    }

    Ok(VestingAccountResponse { address, vestings })
}
