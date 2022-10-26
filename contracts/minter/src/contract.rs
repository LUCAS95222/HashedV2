#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use cw20::Cw20ExecuteMsg;
use cw721_base::{ExecuteMsg as Cw721ExecuteMsg, MintMsg};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    ContractMigrateMsg, CustomCw721ExecuteMsg, ExecuteMsg, InstantiateMsg, MigrationResultResponse,
    NftExtension, NftMigrationReq, QueryMsg, QueryResponse, SupportedToken, TokenInfo,
    TokenMigrationReq, TokenType,
};
use crate::state::{Config, Tx, BURNER_MINTER_IDX, CONFIG, SUPPORTED_TOKEN_MAP, TXS};

const CONTRACT_NAME: &str = "crates.io:minter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const NATIVE_TOKEN: &str = "axpla";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let mut attrs: Vec<Attribute> = vec![];
    for token in msg.supported_tokens {
        if token.burner_token_addr.is_empty() {
            Err(ContractError::BadRequest {
                msg: "burner_token_addr is required".to_string(),
            })?
        }
        if token.minter_token_addr != NATIVE_TOKEN {
            deps.api.addr_validate(&token.minter_token_addr)?;
        } else if token.token_type != TokenType::Native {
            return Err(ContractError::BadRequest {
                msg: "token_type is not native token".to_string(),
            });
        }

        SUPPORTED_TOKEN_MAP.save(
            deps.storage,
            token.burner_token_addr,
            &TokenInfo {
                addr: token.minter_token_addr.clone(),
                token_type: token.token_type.clone(),
            },
        )?;

        attrs.push(Attribute {
            key: "minter_token".to_string(),
            value: token.minter_token_addr.to_string(),
        });
        attrs.push(Attribute {
            key: "token_type".to_string(),
            value: token.token_type.to_string(),
        });
    }

    let config = Config {
        owner: msg
            .owner
            .map(|v| deps.api.addr_validate(&v))
            .transpose()?
            .unwrap_or(info.sender),
        tx_idx: 0,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", config.owner.clone())
        .add_attributes(attrs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match _msg {
        ExecuteMsg::ExecuteMigration(request) => execute_migration(
            _deps,
            _info,
            request.burner_id,
            request.asset,
            request.token_req,
            request.nft_req,
            request.to,
        ),
        ExecuteMsg::UpdateMinter { asset, new_minter } => {
            execute_update_minter(_deps, _info, asset, new_minter)
        }
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
        ExecuteMsg::UpdateOwner { new_owner } => execute_update_owner(_deps, _info, new_owner),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: ContractMigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

fn execute_migration(
    deps: DepsMut,
    info: MessageInfo,
    burner_id: u64, // TODO: what is `burner_id` meaning?
    burner_token_str: String,
    token_req: Option<TokenMigrationReq>,
    nft_req: Option<NftMigrationReq>,
    to: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let ti = SUPPORTED_TOKEN_MAP.load(deps.storage, burner_token_str.clone())?;

    config.tx_idx += 1;
    BURNER_MINTER_IDX.update(deps.storage, burner_id, |old| match old {
        Some(_) => Err(ContractError::BadRequest {
            msg: "burner_id already exists".to_string(),
        }),
        None => Ok(config.tx_idx),
    })?;

    let recipient = deps.api.addr_validate(&to)?;
    let tx = Tx {
        id: config.tx_idx,
        burner_id,
        recipient,
        asset: burner_token_str,
        token_req: token_req.clone(),
        nft_req: nft_req.clone(),
    };

    TXS.save(deps.storage, config.tx_idx, &tx)?;
    CONFIG.save(deps.storage, &config)?; // TODO: config is `mut`, why need save manually?

    let cosmos_msg = match ti.token_type {
        TokenType::Native => {
            let req = token_req.ok_or_else(|| ContractError::BadRequest {
                msg: "token_req is required".to_string(),
            })?;
            get_bank_send_msg(ti.addr.clone(), tx.recipient.to_string(), req.amount)?
        }

        TokenType::Cw20 => {
            let req = token_req.ok_or_else(|| ContractError::BadRequest {
                msg: "token_req is required".to_string(),
            })?;
            get_cw20_mint_msg(ti.addr.clone(), tx.recipient.to_string(), req.amount)?
        }

        TokenType::Cw721 => {
            let req = nft_req.ok_or_else(|| ContractError::BadRequest {
                msg: "nft_req is required".to_string(),
            })?;
            get_cw721_mint_msg(ti.addr.clone(), tx.recipient.to_string(), req)?
        }
    };

    Ok(Response::new()
        .add_message(cosmos_msg)
        .add_attribute("action", "execute_migration")
        .add_attribute("tx_id", config.tx_idx.to_string())
        .add_attribute("burner_tx_id", tx.burner_id.to_string())
        .add_attribute("token", ti.addr.to_string())
        .add_attribute("token_type", ti.token_type.to_string()))
}

fn execute_update_minter(
    deps: DepsMut,
    info: MessageInfo,
    asset: String,
    new_minter: String,
) -> Result<Response, ContractError> {
    if CONFIG.load(deps.storage)?.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    deps.api.addr_validate(&new_minter)?;

    let ti = SUPPORTED_TOKEN_MAP.load(deps.storage, asset)?;

    let cosmos_msg = match ti.token_type {
        TokenType::Native => {
            return Err(ContractError::BadRequest {
                msg: "cannot update native token minter".to_string(),
            })?;
        }
        TokenType::Cw20 => get_cw20_update_minter_msg(ti.addr.clone(), new_minter.clone())?,
        TokenType::Cw721 => get_cw721_update_minter_msg(ti.addr.clone(), new_minter.clone())?,
    };

    Ok(Response::new()
        .add_message(cosmos_msg)
        .add_attribute("action", "update_minter")
        .add_attribute("token", ti.addr)
        .add_attribute("minter", new_minter))
}

pub fn execute_add_token(
    deps: DepsMut,
    info: MessageInfo,
    burner_token_addr: String,
    minter_token_addr: String,
    token_type: TokenType,
) -> Result<Response, ContractError> {
    if CONFIG.load(deps.storage)?.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if burner_token_addr.is_empty() {
        return Err(ContractError::BadRequest {
            msg: "burner_token_addr is empty".to_string(),
        });
    }
    if token_type != TokenType::Native {
        deps.api.addr_validate(&minter_token_addr)?;
    } else if minter_token_addr != NATIVE_TOKEN {
        return Err(ContractError::BadRequest {
            msg: "minter_token_addr is not native token".to_string(),
        });
    }

    SUPPORTED_TOKEN_MAP.update(deps.storage, burner_token_addr.clone(), |old| match old {
        Some(_) => Err(ContractError::BadRequest {
            msg: "already exist".to_string(),
        }),
        None => Ok(TokenInfo {
            addr: minter_token_addr.clone(),
            token_type: token_type.clone(),
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
) -> Result<Response, ContractError> {
    if CONFIG.load(deps.storage)?.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if burner_token_addr.is_empty() {
        return Err(ContractError::BadRequest {
            msg: "burner_token_addr is empty".to_string(),
        });
    }
    SUPPORTED_TOKEN_MAP.remove(deps.storage, burner_token_addr.clone());

    Ok(Response::new()
        .add_attribute("action", "remove_token")
        .add_attribute("burner_token", burner_token_addr))
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    config.owner = deps.api.addr_validate(&new_owner)?;
    CONFIG.save(deps.storage, &config)?; // TODO
    Ok(Response::new()
        .add_attribute("action", "update_owner")
        .add_attribute("owner", new_owner))
}

fn get_bank_send_msg(
    denom: String,
    recipient: String,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient,
        amount: vec![Coin { denom, amount }],
    }))
}

fn get_cw20_mint_msg(
    token_addr: String,
    recipient: String,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
        funds: vec![], // TODO: why empty
    }))
}

fn get_cw721_mint_msg(
    token_addr: String,
    recipient: String,
    req: NftMigrationReq,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&Cw721ExecuteMsg::Mint(MintMsg::<Option<NftExtension>> {
            token_id: req.id, // Unique ID of the NFT
            owner: recipient,
            token_uri: req.uri,
            extension: req.extension,
        }))?,
        funds: vec![],
    }))
}

fn get_cw20_update_minter_msg(
    token_addr: String,
    new_minter: String,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&Cw20ExecuteMsg::UpdateMinter {
            new_minter: Some(new_minter),
        })?,
        funds: vec![],
    }))
}

fn get_cw721_update_minter_msg(
    token_addr: String,
    new_minter: String,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&CustomCw721ExecuteMsg::UpdateMinter { new_minter })?,
        funds: vec![],
    }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    match _msg {
        QueryMsg::MigrationResult { burner_id } => {
            to_binary(&query_migration_result(_deps, burner_id)?)
        }
        QueryMsg::SupportedTokens { start_after } => {
            to_binary(&query_supported_tokens(_deps, start_after)?)
        }
    }
}

fn query_migration_result(deps: Deps, burner_id: u64) -> Result<QueryResponse, StdError> {
    let idx = BURNER_MINTER_IDX.load(deps.storage, burner_id)?;

    Ok(QueryResponse {
        migration_result: Some(MigrationResultResponse {
            burner_id,
            minter_id: idx,
        }),
    })
}

fn query_supported_tokens(
    deps: Deps,
    start_after: Option<String>,
) -> StdResult<Vec<SupportedToken>> {
    let mut res: Vec<SupportedToken> = vec![];
    let start_after = match start_after {
        Some(start_after) => {
            if start_after.is_empty() {
                None
            } else {
                Some(Bound::exclusive(start_after))
            }
        }
        None => None,
    };
    for iter in SUPPORTED_TOKEN_MAP.range(deps.storage, start_after, None, Order::Ascending) {
        let (key, value) = iter?;
        res.push(SupportedToken {
            burner_token_addr: key,
            minter_token_addr: value.addr,
            token_type: value.token_type,
        });
    }
    Ok(res)
}

#[cfg(test)]
mod tests {

    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Uint128,
    };
    use rand::{distributions::Alphanumeric, Rng};

    use crate::{
        contract::{execute_migration, execute_remove_token, instantiate, query_supported_tokens},
        msg::{InstantiateMsg, NftMigrationReq, SupportedToken, TokenMigrationReq, TokenType},
        ContractError,
    };

    struct MigrationReq {
        burner_id: u64,
        burner_token_str: String,
        token_req: Option<TokenMigrationReq>,
        nft_req: Option<NftMigrationReq>,
    }

    fn _gen_token_type() -> TokenType {
        let i = rand::thread_rng().gen_range(0..=2);
        match i {
            0 => TokenType::Native,
            1 => TokenType::Cw20,
            2 => TokenType::Cw721,
            _ => TokenType::Cw20,
        }
    }

    fn _gen_string(len: usize) -> String {
        let addr_str: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();

        addr_str.to_lowercase()
    }

    fn _gen_request(token: SupportedToken) -> MigrationReq {
        let mut rng = rand::thread_rng();
        let mut req = MigrationReq {
            burner_id: rng.gen(),
            burner_token_str: token.burner_token_addr,
            token_req: None,
            nft_req: None,
        };
        match token.token_type {
            TokenType::Cw20 => {
                req.token_req = Some(TokenMigrationReq {
                    amount: Uint128::from(102030u64),
                })
            }
            TokenType::Cw721 => {
                req.nft_req = Some(NftMigrationReq {
                    extension: None,
                    id: _gen_string(10),
                    uri: Some(_gen_string(10)),
                })
            }
            TokenType::Native => {
                req.token_req = Some(TokenMigrationReq {
                    amount: Uint128::from(102030u64),
                })
            }
        };
        req
    }

    #[test]
    fn test_contracts() {
        let user_addr = "xpla11x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v";
        let mut rng = rand::thread_rng();
        const MAX_TOKENS: i32 = 10;
        let mut tokens: Vec<SupportedToken> = vec![];
        for _ in 0..=rng.gen_range(2..MAX_TOKENS) {
            tokens.push(SupportedToken {
                burner_token_addr: _gen_string(32),
                minter_token_addr: _gen_string(32),
                token_type: _gen_token_type(),
            });
        }
        tokens.sort_by(|a, b| a.burner_token_addr.cmp(&b.burner_token_addr));

        let msg: InstantiateMsg = InstantiateMsg {
            owner: Some(user_addr.to_string()),
            supported_tokens: tokens.clone(),
        };
        let mut deps = mock_dependencies();
        let info = mock_info(user_addr, &vec![]);
        let _env = mock_env();

        // instantiate
        let res = instantiate(deps.as_mut(), _env, info.clone(), msg).unwrap();
        assert_eq!(res.attributes.len(), 2 + 2 * tokens.len());
        let last_address = &tokens.iter().last().clone().unwrap().burner_token_addr;
        // query instantiated state
        assert_eq!(
            query_supported_tokens(deps.as_ref(), Some(last_address.to_string()))
                .unwrap()
                .len(),
            0
        );
        let idx = rng.gen_range(0..tokens.len());
        assert_eq!(
            query_supported_tokens(
                deps.as_ref(),
                Some(tokens[idx].burner_token_addr.to_string())
            )
            .unwrap()
            .len(),
            tokens.len() - 1 - idx
        );

        // execute_migration
        let req = _gen_request(tokens[idx].clone());
        assert!(execute_migration(
            deps.as_mut(),
            info.clone(),
            req.burner_id.clone(),
            req.burner_token_str.clone(),
            req.token_req.clone(),
            req.nft_req.clone(),
            user_addr.to_string(),
        )
        .is_ok());

        assert_eq!(
            execute_migration(
                deps.as_mut(),
                info.clone(),
                req.burner_id,
                req.burner_token_str.clone(),
                req.token_req.clone(),
                req.nft_req.clone(),
                user_addr.to_string(),
            )
            .unwrap_err()
            .to_string(),
            ContractError::BadRequest {
                msg: "burner_id already exists".to_string(),
            }
            .to_string()
        );

        execute_remove_token(
            deps.as_mut(),
            info.clone(),
            tokens[idx].burner_token_addr.to_string(),
        )
        .unwrap();
        assert_eq!(
            execute_migration(
                deps.as_mut(),
                info.clone(),
                rng.gen(),
                req.burner_token_str,
                req.token_req,
                req.nft_req,
                user_addr.to_string(),
            )
            .unwrap_err()
            .to_string(),
            "minter::msg::TokenInfo not found".to_string(),
        );
    }
}
