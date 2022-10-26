use std::convert::TryInto;
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Attribute, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::{Bound, Endian, U32Key, U64Key};

use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, NftInfoResponse};

use crate::error::{ContractError, ContractResult};
use crate::msg::{
    ContractMigrateMsg, ExecuteMsg, InstantiateMsg, NftExtension, NftInfo, QueryMsg,
    SupportedToken, TxResponse, UnprocessedMigrationRequestResponse, UserMigrationResponse,
    UserMigrationsItem, UserMigrationsResponse,
};

use crate::state::{
    Config, Tx, UserReqInfo, CONFIG, SUPPORTED_TOKEN_MAP, TXS, UNPROCESSED_NFT_ID_SET,
    UNPROCESSED_TX_IDX, USER_TXS,
};
use crate::types::{MigrationReq, Status, TokenInfo, TokenType, TxResultStatusCode};

pub const RELAYER_TX_HANDLE_LIMIT_DEFAULT: u8 = 10;
pub const RELAYER_TX_HANDLE_LIMIT_MAX: u8 = 20;

pub const USER_INFO_LIMIT: u8 = 20;

const CONTRACT_NAME: &str = "crates.io:burner";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let mut attrs: Vec<Attribute> = vec![];
    for token in msg.supported_tokens {
        let addr = deps.api.addr_validate(&token.burner_token_addr)?;
        if token.minter_token_addr.is_empty() {
            return Err(ContractError::BadRequest {
                message: "minter_token is empty".to_string(),
            });
        };

        SUPPORTED_TOKEN_MAP.save(
            deps.storage,
            &addr,
            &TokenInfo {
                addr: token.minter_token_addr,
                token_type: token.token_type,
            },
        )?;
        attrs.push(Attribute {
            key: "burner_token".to_string(),
            value: token.burner_token_addr.to_string(),
        });
        attrs.push(Attribute {
            key: "token_type".to_string(),
            value: token.token_type.to_string(),
        });
    }

    let tx_limit = msg.tx_limit.unwrap_or(RELAYER_TX_HANDLE_LIMIT_DEFAULT);
    if tx_limit > RELAYER_TX_HANDLE_LIMIT_MAX {
        return Err(ContractError::BadRequest {
            message: format!("Max Tx Limit is {:?}", RELAYER_TX_HANDLE_LIMIT_MAX),
        });
    }

    let config = Config {
        owner: msg
            .owner
            .map(|v| deps.api.addr_validate(&v))
            .transpose()?
            .unwrap_or(info.sender),
        tx_idx: 0,
        tx_limit,
        burn_contract: deps.api.addr_validate(&msg.burn_contract)?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", config.owner.clone())
        .add_attributes(attrs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: ExecuteMsg) -> ContractResult {
    match _msg {
        ExecuteMsg::AddToken {
            burner_token_addr,
            minter_token_addr,
            token_type,
        } => execute_add_token(
            _deps,
            _info,
            burner_token_addr,
            minter_token_addr,
            token_type,
        ),
        ExecuteMsg::RemoveToken { burner_token_addr } => {
            execute_remove_token(_deps, _info, burner_token_addr)
        }
        ExecuteMsg::UpdateTxLimit { count } => execute_update_tx_limit(_deps, _info, count),
        ExecuteMsg::RequestMigrations(requests) => {
            execute_request_migrations(_deps, _info, _env, requests)
        }
        ExecuteMsg::RecordMigrationResult {
            id,
            status,
            minter_id,
            minter_tx_hash,
            message,
        } => execute_record_migration_result(
            _deps,
            _info,
            id,
            status,
            minter_id,
            minter_tx_hash,
            message,
        ),
        ExecuteMsg::UpdateOwner { new_owner } => execute_update_owner(_deps, _info, new_owner),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    match _msg {
        QueryMsg::UserMigrations {
            addr,
            start_after,
            descending,
        } => to_binary(&query_user_migrations(
            _deps,
            addr,
            start_after.unwrap_or_default(),
            descending.unwrap_or(false),
        )?),
        QueryMsg::UserMigration { addr, req_id } => {
            to_binary(&query_user_migration(_deps, addr, req_id)?)
        }
        QueryMsg::MigrationRequest { id } => to_binary(&query_tx_response(_deps, id)?),
        QueryMsg::UnprocessedMigrationRequest {
            items_per_req,
            start_after,
        } => {
            let mut limit = items_per_req.unwrap_or_default();
            limit = if limit > RELAYER_TX_HANDLE_LIMIT_MAX {
                RELAYER_TX_HANDLE_LIMIT_MAX
            } else if limit == 0 {
                CONFIG.load(_deps.storage)?.tx_limit
            } else {
                limit
            };
            to_binary(&query_unprocessed_migration_requests(
                _deps,
                limit,
                start_after.unwrap_or_default(),
            )?)
        }
        QueryMsg::SupportedTokens { start_after } => {
            to_binary(&query_supported_tokens(_deps, start_after)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: ContractMigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

pub fn execute_request_migrations( // TODO: no need verifying owner?
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    reqs: Vec<MigrationReq>,
) -> ContractResult {
    let mut config = CONFIG.load(deps.storage)?;

    if reqs.len() > USER_INFO_LIMIT.into() {
        return Err(ContractError::BadRequest {
            message: format!("too many requests, tx limit is{}", USER_INFO_LIMIT),
        });
    }

    let mut tx_idx = config.tx_idx;
    let user_addr = info.sender;
    let contract_addr = env.contract.address;
    let user_req_id = get_user_last_req_id(&deps, &user_addr) + 1;

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    let mut user_req_info = UserReqInfo {
        tx_ids: vec![],
        block_num: env.block.height,
        timestamp: env.block.time.nanos() / 1_000_000,
        fail: 0,
        success: 0,
        in_progress: 0,
    };

    for req in reqs {
        let ti = SUPPORTED_TOKEN_MAP.load(deps.storage, &deps.api.addr_validate(&req.asset)?)?;
        let token_addr = req.asset.clone();
        let id_or_amount = match ti.token_type {
            TokenType::Cw721 => {
                if req.nft_id.is_none() || req.nft_id.clone().unwrap().is_empty() {
                    return Err(ContractError::BadRequest {
                        message: "nft_id is required for Cw721 token".to_string(),
                    });
                }
                let nft_id = req.nft_id.clone().unwrap();
                UNPROCESSED_NFT_ID_SET.update(
                    deps.storage,
                    (token_addr.clone(), nft_id.clone()),
                    |old| match old {
                        Some(_) => Err(ContractError::BadRequest {
                            message: format!("nft_id {} is already in use", nft_id),
                        }),
                        None => Ok(true),
                    },
                )?;
                nft_id
            }
            TokenType::Cw20 => {
                if req.amount.is_none() || req.amount.clone().unwrap() == "0" {
                    return Err(ContractError::BadRequest {
                        message: "amount is required for cw20 token".to_string(),
                    });
                };
                req.amount.clone().unwrap()
            }
        };

        if req.to.is_empty() {
            return Err(ContractError::BadRequest {
                message: "to is required".to_string(),
            });
        }

        cosmos_msgs.push(match ti.token_type {
            TokenType::Cw20 => get_cw20_transfer_from_msg(
                token_addr,
                user_addr.to_string(),
                contract_addr.to_string(),
                Uint128::from_str(&id_or_amount)?,
            )?,
            TokenType::Cw721 => {
                get_cw721_transfer_msg(token_addr, contract_addr.to_string(), id_or_amount)?
            }
        });

        tx_idx += 1;
        user_req_info.tx_ids.push(tx_idx);

        UNPROCESSED_TX_IDX.update(deps.storage, U64Key::from(tx_idx), |old| match old {
            Some(_) => Err(ContractError::InternalServerError {
                message: format!("tx id {} is already in use", tx_idx),
            }),
            None => Ok(true),
        })?;

        let item = Tx {
            id: tx_idx,
            status: crate::types::Status::Created,
            from: user_addr.clone(),
            to: req.to,
            user_req_id,
            token_addr: deps.api.addr_validate(&req.asset)?,
            minter_token_addr: ti.addr.clone(),
            amount: if ti.token_type == TokenType::Cw20 {
                Uint128::from_str(&req.amount.unwrap())?
            } else {
                Uint128::zero()
            },
            nft_id: req.nft_id.unwrap_or_default().to_string(),
            msg: None,
            minter_id: None,
            minter_tx_hash: None,
        };
        TXS.update(
            deps.storage,
            U64Key::new(tx_idx),
            |option_tx| match option_tx {
                Some(_) => Err(ContractError::InternalServerError {
                    message: ("tx_idx duplicated".to_string()),
                }),
                None => Ok(item),
            },
        )?;
    }

    user_req_info.in_progress = user_req_info.tx_ids.len() as u8;
    USER_TXS.save(
        deps.storage,
        (&user_addr, U32Key::new(user_req_id)),
        &user_req_info,
    )?;
    config.tx_idx = tx_idx;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "request_migrations")
        .add_messages(cosmos_msgs))
}

pub fn execute_record_migration_result(
    deps: DepsMut,
    info: MessageInfo,
    tx_id: u64,
    status: i16,
    minter_id: Option<u64>,
    minter_tx_hash: Option<String>,
    message: Option<String>,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let tx = TXS.update(deps.storage, U64Key::from(tx_id), |tx| match tx {
        Some(mut tx) => {
            if tx.status != Status::Created {
                return Err(ContractError::CustomError {
                    status: (http::StatusCode::CONFLICT.as_u16()),
                    message: ("tx already processed".to_string()),
                });
            }

            tx.status = if status == TxResultStatusCode::Success as i16 {
                Status::Swapped
            } else {
                Status::PaidBack
            };
            tx.minter_id = minter_id; // : addr_validate &str
            tx.minter_tx_hash = minter_tx_hash;
            tx.msg = message;
            Ok(tx)
        }
        None => Err(ContractError::InternalServerError {
            message: ("tx_id not found".to_string()),
        }),
    })?;

    let ti = SUPPORTED_TOKEN_MAP.load(deps.storage, &tx.token_addr)?;

    USER_TXS.update(
        deps.storage,
        (&tx.from, U32Key::new(tx.user_req_id)),
        |user_req| match user_req {
            Some(mut user_req) => {
                user_req.in_progress -= 1;
                if status == TxResultStatusCode::Success as i16 {
                    user_req.success += 1;
                } else {
                    user_req.fail += 1;
                }
                Ok(user_req)
            }
            None => Err(ContractError::InternalServerError {
                message: ("user_req_id not found".to_string()),
            }),
        },
    )?;
    UNPROCESSED_TX_IDX.remove(deps.storage, U64Key::from(tx_id));

    let msg = match ti.token_type {
        TokenType::Cw20 => {
            if status == TxResultStatusCode::Success as i16 { // 1. succeed => burn cw20
                get_cw20_burn_msg(tx.token_addr.to_string(), tx.amount)?
            } else {                                          // 2. else => send cw20 to `from`
                get_cw20_transfer_msg(tx.token_addr.to_string(), tx.from.to_string(), tx.amount)?
            }
        }
        TokenType::Cw721 => {
            UNPROCESSED_NFT_ID_SET
                .remove(deps.storage, (tx.token_addr.to_string(), tx.nft_id.clone()));
            if status == TxResultStatusCode::Success as i16 { // 1. succeed => send NFT to `burn_contract`
                get_cw721_transfer_msg(
                    tx.token_addr.to_string(),
                    config.burn_contract.to_string(),
                    tx.nft_id,
                )?
            } else {                                          // 2. else => send NFT to `from`
                get_cw721_transfer_msg(tx.token_addr.to_string(), tx.from.to_string(), tx.nft_id)?
            }
        }
    };

    Ok(Response::new()
        .add_attribute("action", "record_migration_result")
        .add_message(msg))
}

pub fn execute_add_token(
    deps: DepsMut,
    info: MessageInfo,
    burner_token_addr: String,
    minter_token_addr: String,
    token_type: TokenType,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if minter_token_addr.is_empty() {
        return Err(ContractError::BadRequest {
            message: "minter_token_addr is empty".to_string(),
        });
    }

    let addr = deps.api.addr_validate(&burner_token_addr)?;

    SUPPORTED_TOKEN_MAP.update(deps.storage, &addr, |old| match old {
        Some(_) => Err(ContractError::BadRequest {
            message: "already exist".to_string(),
        }),
        None => Ok(TokenInfo {
            addr: minter_token_addr.clone(),
            token_type,
        }),
    })?;

    Ok(Response::new()
        .add_attribute("action", "add_token")
        .add_attribute("burner_token", burner_token_addr)
        .add_attribute("minter_token", minter_token_addr)
        .add_attribute("token_type", token_type.to_string()))
}

pub fn execute_remove_token(
    deps: DepsMut,
    info: MessageInfo,
    burner_token_addr: String,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let addr = deps.api.addr_validate(&burner_token_addr)?; // TODO: `remove` why need validate?
    match UNPROCESSED_TX_IDX
        .keys(deps.storage, None, None, Order::Descending)
        .next()
    {
        Some(_) => {
            return Err(ContractError::BadRequest {
                message: "there are unprocessed txs".to_string(),
            })
        }
        None => (),
    };
    SUPPORTED_TOKEN_MAP.remove(deps.storage, &addr);

    Ok(Response::new()
        .add_attribute("action", "remove_token")
        .add_attribute("burner_token", burner_token_addr))
}

pub fn execute_update_tx_limit(deps: DepsMut, info: MessageInfo, tx_limit: u8) -> ContractResult {
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if tx_limit > RELAYER_TX_HANDLE_LIMIT_MAX || tx_limit == 0 {
        return Err(ContractError::BadRequest {
            message: format!(
                "tx_limit must be 0 < tx_limit ≤ {:?}",
                RELAYER_TX_HANDLE_LIMIT_MAX
            ),
        });
    }

    config.tx_limit = tx_limit;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_tx_limit")
        .add_attribute("tx_limit", tx_limit.to_string()))
}

pub fn execute_update_owner(deps: DepsMut, info: MessageInfo, new_owner: String) -> ContractResult {
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    config.owner = deps.api.addr_validate(&new_owner)?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_owner")
        .add_attribute("owner", new_owner))
}

pub fn get_user_last_req_id(deps: &DepsMut, user_addr: &Addr) -> u32 {
    match USER_TXS
        .prefix(user_addr)
        .keys(deps.storage, None, None, Order::Descending) // : why Descending?
        .next()
    {
        Some(v) => u32::from_be_bytes(v.try_into().expect("Invalid user_txs key")),
        None => 0,
    }
}
pub fn get_cw20_transfer_from_msg(
    token_addr: String,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom { // TODO: when do `approve`?
            owner,
            recipient,
            amount,
        })?,
        funds: vec![],
    }))
}

pub fn get_cw20_transfer_msg(
    token_addr: String,
    recipient: String,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&Cw20ExecuteMsg::Transfer { recipient, amount })?,
        funds: vec![],
    }))
}

pub fn get_cw20_burn_msg(token_addr: String, amount: Uint128) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
        funds: vec![],
    }))
}

pub fn get_cw721_transfer_msg(
    token_addr: String,
    recipient: String,
    token_id: String,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute { // TODO: who is the `sender`? anyone can send any specified NFT?
        contract_addr: token_addr,
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            recipient,
            token_id,
        })?,
        funds: vec![],
    }))
}

pub fn query_unprocessed_migration_requests(
    deps: Deps,
    items_per_request: u8,
    start_after: u64,
) -> StdResult<UnprocessedMigrationRequestResponse> {
    let mut res = UnprocessedMigrationRequestResponse { items: vec![] };
    for last_key_vec in UNPROCESSED_TX_IDX
        .prefix(())
        .keys(
            deps.storage,
            Option::Some(Bound::exclusive(U64Key::from(start_after))),
            None,
            Order::Ascending,
        )
        .take(items_per_request as usize)
    {
        let tx_id =
            u64::from_be_bytes(last_key_vec.try_into().expect("Invalid unprocessed_tx key"));
        let mut tx = query_tx_response(deps, tx_id)?;
        let ti =
            SUPPORTED_TOKEN_MAP.load(deps.storage, &deps.api.addr_validate(&tx.token_addr)?)?;
        if ti.token_type == TokenType::Cw721 {
            let nft_id = tx.nft_info.unwrap().id.clone();
            let res: NftInfoResponse<Option<NftExtension>> = deps.querier.query_wasm_smart(
                tx.token_addr.clone(),
                &Cw721QueryMsg::NftInfo {
                    token_id: nft_id.clone(),
                },
            )?;
            tx.nft_info = Some(NftInfo {
                extension: res.extension,
                uri: res.token_uri,
                id: nft_id,
            });
        };
        res.items.push(tx);
    }
    Ok(res)
}

pub fn query_user_migrations(
    deps: Deps,
    target_addr: String,
    start_after: u32,
    descending: bool,
) -> StdResult<UserMigrationsResponse> {
    let addr = deps.api.addr_validate(&target_addr)?;
    let mut res = UserMigrationsResponse { migrations: vec![] };
    let mut max = None;
    let mut min = None;
    let order = if descending {
        max = Option::Some(Bound::exclusive(U32Key::from(start_after)));
        Order::Descending
    } else {
        min = Option::Some(Bound::exclusive(U32Key::from(start_after)));
        Order::Ascending
    };

    for iter in USER_TXS
        .prefix(&addr)
        .keys(deps.storage, min, max, order)
        .take(USER_INFO_LIMIT as usize)
    {
        let req_id = u32::from_be_bytes(iter.try_into().expect("Invalid user_txs key"));
        let req = USER_TXS.load(deps.storage, (&addr, U32Key::from(req_id)))?;
        res.migrations.push(UserMigrationsItem {
            req_id,
            block_num: req.block_num,
            timestamp: req.timestamp,
            success: req.success,
            fail: req.fail,
            in_progress: req.in_progress,
        });
    }
    Ok(res)
}

pub fn query_user_migration(
    deps: Deps,
    target_addr: String,
    req_id: u32,
) -> StdResult<UserMigrationResponse> {
    let addr = deps.api.addr_validate(&target_addr)?;
    let mut res = UserMigrationResponse { txs: vec![] };

    for tid in USER_TXS
        .load(deps.storage, (&addr, U32Key::from(req_id)))?
        .tx_ids
    {
        res.txs.push(query_tx_response(deps, tid)?);
    }
    Ok(res)
}

pub fn query_tx_response(deps: Deps, id: u64) -> StdResult<TxResponse> {
    let tx = TXS.load(deps.storage, U64Key::from(id))?;
    Ok(TxResponse {
        id: tx.id,
        status: tx.status,
        msg: tx.msg,
        from: tx.from.to_string(),
        to: tx.to,
        user_req_id: tx.user_req_id,
        token_addr: tx.token_addr.to_string(),
        amount: if tx.amount.is_zero() {
            None
        } else {
            Some(tx.amount.to_string())
        },
        nft_info: if tx.nft_id.is_empty() {
            None
        } else {
            Some(NftInfo {
                id: tx.nft_id,
                uri: None,
                extension: None,
            })
        },
        minter_id: tx.minter_id,
        minter_tx_hash: tx.minter_tx_hash,
    })
}

pub fn query_supported_tokens(
    deps: Deps,
    start_after: Option<String>,
) -> StdResult<Vec<SupportedToken>> {
    let mut res: Vec<SupportedToken> = vec![];
    let start_after = if start_after.is_some() {
        let addr = deps.api.addr_validate(start_after.as_ref().unwrap())?;
        Some(Bound::exclusive(addr.as_ref().as_bytes()))
    } else {
        None
    };

    for iter in SUPPORTED_TOKEN_MAP.range(deps.storage, start_after, None, Order::Ascending) {
        let (key, value) = iter?;

        res.push(SupportedToken {
            burner_token_addr: String::from_utf8(key)?,
            minter_token_addr: value.addr,
            token_type: value.token_type,
        });
    }
    Ok(res)
}
