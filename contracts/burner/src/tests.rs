#[cfg(test)]
mod tests {

    use cosmwasm_std::{
        from_binary, from_slice,
        testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage},
        to_binary, Addr, ContractResult, DepsMut, Empty, Env, MessageInfo, OwnedDeps, Querier,
        QuerierResult, QueryRequest, StdError, SystemError, SystemResult, WasmQuery,
    };
    use cw721::{Cw721QueryMsg, NftInfoResponse};
    use cw_storage_plus::U32Key;
    use rand::{distributions::Alphanumeric, Rng};

    use crate::{
        contract::{execute_record_migration_result, get_user_last_req_id},
        types::{Status, TxResultStatusCode},
    };
    use crate::{
        contract::{
            execute_remove_token, execute_request_migrations, instantiate, query_supported_tokens,
            query_unprocessed_migration_requests, query_user_migration, query_user_migrations,
            RELAYER_TX_HANDLE_LIMIT_DEFAULT,
        },
        msg::{InstantiateMsg, NftExtension, NftExtensionDisplay, SupportedToken},
        state::{UserReqInfo, USER_TXS},
        types::{MigrationReq, TokenType},
        ContractError,
    };

    struct TestContext {
        deps: OwnedDeps<MockStorage, MockApi, NftQuerier>,
        env: Env,
        info: MessageInfo,
        instantiate_msg: InstantiateMsg,
        tokens: Vec<SupportedToken>,
        user_addr: Addr,
    }
    struct NftQuerier {
        base: MockQuerier,
    }
    impl Querier for NftQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            let request: QueryRequest<Empty> = match from_slice(bin_request) {
                Ok(v) => v,
                Err(e) => {
                    return SystemResult::Err(SystemError::InvalidRequest {
                        error: format!("Parsing query request: {}", e),
                        request: bin_request.into(),
                    })
                }
            };
            self.handle_query(&request)
        }
    }

    impl NftQuerier {
        pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
            match &request {
                QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: _,
                    msg,
                }) => match from_binary(msg) {
                    Ok(Cw721QueryMsg::NftInfo { token_id: _ }) => {
                        let attrs: Vec<NftExtensionDisplay> = vec![NftExtensionDisplay {
                            display_type: None,
                            trait_type: None,
                            value: Some("https://example.com".to_string()),
                        }];
                        let v = NftInfoResponse {
                            token_uri: Some(_gen_string(10)),
                            extension: Some(NftExtension {
                                image: Some(_gen_string(10)),
                                image_data: Some(_gen_string(10)),
                                external_url: Some(_gen_string(10)),
                                description: Some(_gen_string(10)),
                                name: Some(_gen_string(10)),
                                attributes: attrs,
                                background_color: Some(_gen_string(10)),
                                animation_url: Some(_gen_string(10)),
                                youtube_url: Some(_gen_string(10)),
                            }),
                        };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&v).unwrap()))
                    }

                    Ok(_) => SystemResult::Err(SystemError::InvalidRequest {
                        error: "Unsupported Query".to_string(),
                        request: msg.as_slice().into(),
                    }),

                    Err(_) => SystemResult::Err(SystemError::InvalidRequest {
                        error: "Bad Query message".to_string(),
                        request: msg.as_slice().into(),
                    }),
                },
                _ => self.base.handle_query(request),
            }
        }
    }

    fn _gen_string(len: usize) -> String {
        let addr_str: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();

        Addr::unchecked(addr_str.to_lowercase()).to_string()
    }

    fn _gen_token_type() -> TokenType {
        if let 0 = rand::thread_rng().gen_range(0..=1) {
            TokenType::Cw20
        } else {
            TokenType::Cw721
        }
    }

    fn _gen_test_context() -> TestContext {
        let mut rng = rand::thread_rng();
        let user_addr = Addr::unchecked(_gen_string(32));
        let mut tokens: Vec<SupportedToken> = vec![];
        for _ in 0..=rng.gen_range(2..10) {
            tokens.push(SupportedToken {
                burner_token_addr: _gen_string(32),
                minter_token_addr: _gen_string(32),
                token_type: _gen_token_type(),
            });
        }
        tokens.sort_by(|a, b| a.burner_token_addr.cmp(&b.burner_token_addr));

        let _deps = mock_dependencies(&[]);
        TestContext {
            deps: OwnedDeps {
                storage: _deps.storage,
                api: _deps.api,
                querier: NftQuerier {
                    base: _deps.querier,
                },
            },
            info: mock_info(user_addr.as_str(), &vec![]),
            env: mock_env(),
            tokens: tokens.clone(),
            instantiate_msg: InstantiateMsg {
                owner: None,
                supported_tokens: tokens,
                tx_limit: Some(10),
                burn_contract: user_addr.to_string(),
            },
            user_addr: user_addr,
        }
    }

    fn _gen_requests(tokens: &Vec<SupportedToken>, to: String) -> Vec<MigrationReq> {
        let mut rng = rand::thread_rng();
        let mut reqs: Vec<MigrationReq> = vec![];
        for _ in 0..=rng.gen_range(0..RELAYER_TX_HANDLE_LIMIT_DEFAULT) {
            let idx = rng.gen_range(0..tokens.len());
            reqs.push(match tokens[idx].token_type {
                TokenType::Cw20 => {
                    let amount: u64 = rng.gen();
                    MigrationReq {
                        asset: tokens[idx].burner_token_addr.to_string(),
                        amount: Some(amount.to_string()),
                        nft_id: None,
                        to: to.clone(),
                    }
                }
                TokenType::Cw721 => MigrationReq {
                    asset: tokens[idx].burner_token_addr.to_string(),
                    amount: None,
                    nft_id: Some(_gen_string(10)),
                    to: to.clone(),
                },
            });
        }
        reqs
    }

    fn _update_user_txs(
        deps: DepsMut,
        env: &Env,
        user_addr: &Addr,
        req_idx: u32,
    ) -> Result<UserReqInfo, StdError> {
        USER_TXS.update(
            deps.storage,
            (&user_addr, U32Key::new(req_idx)),
            |req_info_opt| match req_info_opt {
                Some(_) => Err(StdError::generic_err("already exist")),
                None => Ok(UserReqInfo {
                    tx_ids: vec![1, 2, 3],
                    block_num: env.block.height,
                    timestamp: env.block.time.seconds() * 1000,
                    fail: 0,
                    success: 0,
                    in_progress: 3,
                }),
            },
        )
    }

    fn setup_test() -> TestContext {
        let mut ctx = _gen_test_context();
        let res = instantiate(
            ctx.deps.as_mut(),
            ctx.env.clone(),
            ctx.info.clone(),
            ctx.instantiate_msg.clone(),
        )
        .unwrap();
        assert_eq!(res.attributes.len(), 2 + ctx.tokens.len() * 2);
        ctx
    }

    #[test]
    fn test_instantiate() {
        let ctx = setup_test();

        let tokens = query_supported_tokens(ctx.deps.as_ref(), None).unwrap();
        assert_eq!(tokens.len(), ctx.tokens.len());
        assert_eq!(tokens[0].burner_token_addr, ctx.tokens[0].burner_token_addr);

        let tokens = query_supported_tokens(
            ctx.deps.as_ref(),
            Some(tokens.last().unwrap().burner_token_addr.to_string()),
        )
        .unwrap();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_user_txs() {
        let mut ctx = _gen_test_context();
        let user_addr = ctx.user_addr.clone();
        let target_id = 10u32;
        let fake_user_info = UserReqInfo {
            tx_ids: vec![1, 2, 3],
            block_num: ctx.env.clone().block.height,
            timestamp: ctx.env.clone().block.time.seconds() * 1000,
            fail: 0,
            success: 0,
            in_progress: 3,
        };

        for i in 0..=target_id {
            _update_user_txs(ctx.deps.as_mut(), &ctx.env, &user_addr, i).unwrap();
        }

        let last_id = get_user_last_req_id(&ctx.deps.as_mut(), &user_addr);

        let result = USER_TXS.update(
            ctx.deps.as_mut().storage,
            (&user_addr, U32Key::new(target_id)),
            |req_info_opt| match req_info_opt {
                Some(_) => Err(StdError::generic_err("already exist")),
                None => Ok(fake_user_info.clone()),
            },
        );
        assert!(result.is_err());
        assert_eq!(last_id, target_id);
        USER_TXS.remove(
            ctx.deps.as_mut().storage,
            (&user_addr, U32Key::new(last_id)),
        );
        let new_last = get_user_last_req_id(&ctx.deps.as_mut(), &user_addr);
        assert_eq!(last_id - 1, new_last);
    }

    #[test]
    fn test_requests_migrations() {
        let ctx = setup_test();
        let tokens = ctx.tokens;
        let reqs = _gen_requests(&tokens, ctx.user_addr.to_string());
        let mut deps = ctx.deps;
        let expected_len = reqs.len();

        let res =
            execute_request_migrations(deps.as_mut(), ctx.info.clone(), ctx.env.clone(), reqs);
        assert!(res.is_ok());

        let txs =
            query_unprocessed_migration_requests(deps.as_ref(), RELAYER_TX_HANDLE_LIMIT_DEFAULT, 0);
        assert!(txs.is_ok());
        assert_eq!(txs.unwrap().items.len(), expected_len);

        for _ in 0..=5 {
            execute_request_migrations(
                deps.as_mut(),
                ctx.info.clone(),
                ctx.env.clone(),
                _gen_requests(&tokens, ctx.user_addr.to_string()),
            )
            .unwrap();
        }

        let user_migrations =
            query_user_migrations(deps.as_ref(), ctx.user_addr.to_string(), 0, false).unwrap();
        assert_eq!(user_migrations.migrations.len(), 7);
        let user_migrations =
            query_user_migrations(deps.as_ref(), ctx.user_addr.to_string(), 6, false).unwrap();
        assert_eq!(user_migrations.migrations.len(), 1);

        let user_migrations =
            query_user_migrations(deps.as_ref(), ctx.user_addr.to_string(), 5, true).unwrap();
        assert_eq!(user_migrations.migrations.len(), 4);

        let migrations = query_user_migration(deps.as_ref(), ctx.user_addr.to_string(), 1).unwrap();
        assert_eq!(migrations.txs.len(), expected_len);
    }

    #[test]
    fn test_record_migration() {
        let ctx = setup_test();
        let tokens = ctx.tokens;
        let reqs = _gen_requests(&tokens, ctx.user_addr.to_string());
        let mut deps = ctx.deps;

        let reqs_len = reqs.len();

        assert!(
            execute_request_migrations(deps.as_mut(), ctx.info.clone(), ctx.env.clone(), reqs)
                .is_ok()
        );

        let txs =
            query_unprocessed_migration_requests(deps.as_ref(), RELAYER_TX_HANDLE_LIMIT_DEFAULT, 0)
                .unwrap();
        let idx = rand::thread_rng().gen_range(0..txs.items.len());
        let target = txs.items[idx].clone();

        execute_record_migration_result(
            deps.as_mut(),
            ctx.info.clone(),
            target.id,
            TxResultStatusCode::Success as i16,
            Some(1),
            Some(_gen_string(10)),
            Some("Success".to_string()),
        )
        .unwrap();

        assert_eq!(
            execute_record_migration_result(
                deps.as_mut(),
                ctx.info,
                target.id,
                TxResultStatusCode::Success as i16,
                Some(1),
                Some(_gen_string(10)),
                Some("Success".to_string()),
            )
            .unwrap_err()
            .to_string(),
            ContractError::CustomError {
                status: (http::StatusCode::CONFLICT.as_u16()),
                message: ("tx already processed".to_string()),
            }
            .to_string()
        );

        let migrations = query_user_migration(deps.as_ref(), ctx.user_addr.to_string(), 1).unwrap();
        assert_eq!(migrations.txs.len(), reqs_len);
        let mut target_it = migrations.txs.iter().filter(|it| it.id == target.id);
        assert_eq!(target_it.next().unwrap().status, Status::Swapped);
    }

    #[test]
    fn test_execute_remove_token() {
        let ctx = setup_test();
        let mut deps = ctx.deps;
        let mut tokens = ctx.tokens;

        let idx = rand::thread_rng().gen_range(0..tokens.len());

        let target_token = tokens.remove(idx);
        let response = execute_remove_token(
            deps.as_mut(),
            ctx.info.clone(),
            target_token.burner_token_addr.to_string(),
        );
        assert!(response.is_ok());
        assert_eq!(response.unwrap().attributes.len(), 2);

        let supported_tokens = query_supported_tokens(deps.as_ref(), None).unwrap();
        let mut filtered = supported_tokens
            .iter()
            .filter(|t| t.burner_token_addr == target_token.burner_token_addr);
        assert_eq!(tokens.len(), supported_tokens.len());
        assert_eq!(filtered.next(), None);

        let reqs = _gen_requests(&tokens, ctx.user_addr.to_string());
        let req_len = reqs.len();
        execute_request_migrations(deps.as_mut(), ctx.info.clone(), ctx.env.clone(), reqs).unwrap();
        let migrations = query_user_migration(deps.as_ref(), ctx.user_addr.to_string(), 1).unwrap();
        assert_eq!(migrations.txs.len(), req_len);

        let response = execute_remove_token(
            deps.as_mut(),
            ctx.info,
            tokens.last().unwrap().burner_token_addr.to_string(),
        );
        assert!(response.is_err());
        assert_eq!(
            response.unwrap_err().to_string(),
            ContractError::BadRequest {
                message: "there are unprocessed txs".to_string(),
            }
            .to_string()
        );
    }
}
