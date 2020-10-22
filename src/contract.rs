use cosmwasm_std::{
    attr, from_binary, to_binary, Api, BankMsg, Binary, CosmosMsg, Env, Extern, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, Querier, StdResult, Storage, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{Balance, Cw20Coin, Cw20CoinHuman, Cw20HandleMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    CreateMsg, DetailsResponse, HandleMsg, InitMsg, ListResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{all_clawback_ids, clawbacks, clawbacks_read, Clawback, GenericBalance};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-clawback";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _info: MessageInfo,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // no setup
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Create(msg) => {
            try_create(deps, env, msg, Balance::from(info.sent_funds), &info.sender)
        }
        HandleMsg::TopUp { id } => try_top_up(deps, id, Balance::from(info.sent_funds)),
        HandleMsg::Receive(msg) => try_receive(deps, env, info, msg),
        HandleMsg::Withdraw { id } => try_withdraw(deps, env, info, id),
        HandleMsg::Refresh { id } => try_refresh(deps, env, info, id),
        HandleMsg::Burn { id } => try_burn(deps, env, info, id),
        HandleMsg::ClawbackTransfer {
            from_id,
            to_id,
            amount,
        } => try_transfer(deps, env, info, from_id, to_id, amount),
    }
}

pub fn try_withdraw<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<HandleResponse, ContractError> {
    todo!()
}

pub fn try_refresh<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<HandleResponse, ContractError> {
    todo!()
}

pub fn try_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<HandleResponse, ContractError> {
    todo!()
}

pub fn try_transfer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    from_id: String,
    to_id: String,
    amount: Balance,
) -> Result<HandleResponse, ContractError> {
    todo!()
}

pub fn try_receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let msg: ReceiveMsg = match wrapper.msg {
        Some(bin) => Ok(from_binary(&bin)?),
        None => Err(ContractError::NoData {}),
    }?;
    let balance = Balance::Cw20(Cw20Coin {
        address: deps.api.canonical_address(&info.sender)?,
        amount: wrapper.amount,
    });
    match msg {
        ReceiveMsg::Create(msg) => try_create(deps, env, msg, balance, &wrapper.sender),
        ReceiveMsg::TopUp { id } => try_top_up(deps, id, balance),
    }
}

pub fn try_create<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: CreateMsg,
    balance: Balance,
    sender: &HumanAddr,
) -> Result<HandleResponse, ContractError> {
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let mut cw20_whitelist = msg.canonical_whitelist(&deps.api)?;

    let clawback_balance = match balance {
        Balance::Native(balance) => GenericBalance {
            native: balance.0,
            cw20: vec![],
        },
        Balance::Cw20(token) => {
            // make sure the token sent is on the whitelist by default
            if !cw20_whitelist.iter().any(|t| t == &token.address) {
                cw20_whitelist.push(token.address.clone())
            }
            GenericBalance {
                native: vec![],
                cw20: vec![token],
            }
        }
    };

    let clawback = Clawback {
        backup: deps.api.canonical_address(&msg.backup)?,
        holder: deps.api.canonical_address(&msg.holder)?,
        clawback_period: msg.clawback_period,
        end_time: env.block.time + msg.clawback_period,
        balance: clawback_balance,
        cw20_whitelist,
    };

    // try to store it, fail if the id was already in use
    clawbacks(&mut deps.storage).update(msg.id.as_bytes(), |existing| match existing {
        None => Ok(clawback),
        Some(_) => Err(ContractError::AlreadyInUse {}),
    })?;

    let mut res = HandleResponse::default();
    res.attributes = vec![attr("action", "create"), attr("id", msg.id)];
    Ok(res)
}

pub fn try_top_up<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    id: String,
    balance: Balance,
) -> Result<HandleResponse, ContractError> {
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }
    // this fails is no clawback there
    let mut clawback = clawbacks_read(&deps.storage).load(id.as_bytes())?;

    if let Balance::Cw20(token) = &balance {
        // ensure the token is on the whitelist
        if !clawback.cw20_whitelist.iter().any(|t| t == &token.address) {
            return Err(ContractError::NotInWhitelist {});
        }
    };

    clawback.balance.add_tokens(balance);

    // and save
    clawbacks(&mut deps.storage).save(id.as_bytes(), &clawback)?;

    let mut res = HandleResponse::default();
    res.attributes = vec![attr("action", "top_up"), attr("id", id)];
    Ok(res)
}

// pub fn try_approve<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     info: MessageInfo,
//     id: String,
// ) -> Result<HandleResponse, ContractError> {
//     // this fails is no escrow there
//     let escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

//     if deps.api.canonical_address(&info.sender)? != escrow.arbiter {
//         Err(ContractError::Unauthorized {})
//     } else if escrow.is_expired(&env) {
//         Err(ContractError::Expired {})
//     } else {
//         // we delete the escrow
//         escrows(&mut deps.storage).remove(id.as_bytes());

//         let rcpt = deps.api.human_address(&escrow.recipient)?;

//         // send all tokens out
//         let messages = send_tokens(&deps.api, &env.contract.address, &rcpt, &escrow.balance)?;

//         let attributes = vec![attr("action", "approve"), attr("id", id), attr("to", rcpt)];
//         Ok(HandleResponse {
//             messages,
//             attributes,
//             data: None,
//         })
//     }
// }

// pub fn try_refund<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     info: MessageInfo,
//     id: String,
// ) -> Result<HandleResponse, ContractError> {
//     // this fails is no escrow there
//     let escrow = escrows_read(&deps.storage).load(id.as_bytes())?;

//     // the arbiter can send anytime OR anyone can send after expiration
//     if !escrow.is_expired(&env) && deps.api.canonical_address(&info.sender)? != escrow.arbiter {
//         Err(ContractError::Unauthorized {})
//     } else {
//         // we delete the escrow
//         escrows(&mut deps.storage).remove(id.as_bytes());

//         let rcpt = deps.api.human_address(&escrow.source)?;

//         // send all tokens out
//         let messages = send_tokens(&deps.api, &env.contract.address, &rcpt, &escrow.balance)?;

//         let attributes = vec![attr("action", "refund"), attr("id", id), attr("to", rcpt)];
//         Ok(HandleResponse {
//             messages,
//             attributes,
//             data: None,
//         })
//     }
// }

fn send_tokens<A: Api>(
    api: &A,
    from: &HumanAddr,
    to: &HumanAddr,
    balance: &GenericBalance,
) -> StdResult<Vec<CosmosMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<CosmosMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![BankMsg::Send {
            from_address: from.into(),
            to_address: to.into(),
            amount: native_balance.to_vec(),
        }
        .into()]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            let msg = Cw20HandleMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = WasmMsg::Execute {
                contract_addr: api.human_address(&c.address)?,
                msg: to_binary(&msg)?,
                send: vec![],
            };
            Ok(exec.into())
        })
        .collect();
    msgs.append(&mut cw20_msgs?);
    Ok(msgs)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::List {} => to_binary(&query_list(deps)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
    }
}

fn query_details<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: String,
) -> StdResult<DetailsResponse> {
    let clawback = clawbacks_read(&deps.storage).load(id.as_bytes())?;

    let cw20_whitelist = clawback.human_whitelist(&deps.api)?;

    // transform tokens
    let native_balance = clawback.balance.native;

    let cw20_balance: StdResult<Vec<_>> = clawback
        .balance
        .cw20
        .into_iter()
        .map(|token| {
            Ok(Cw20CoinHuman {
                address: deps.api.human_address(&token.address)?,
                amount: token.amount,
            })
        })
        .collect();

    let details = DetailsResponse {
        id,
        backup: deps.api.human_address(&clawback.backup)?,
        holder: deps.api.human_address(&clawback.holder)?,
        clawback_period: clawback.clawback_period,
        end_time: clawback.end_time,
        native_balance,
        cw20_balance: cw20_balance?,
        cw20_whitelist,
    };
    Ok(details)
}

fn query_list<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<ListResponse> {
    Ok(ListResponse {
        clawbacks: all_clawback_ids(&deps.storage)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::msg::HandleMsg::TopUp;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, CanonicalAddr, CosmosMsg, StdError, Uint128};
    use cw0::NativeBalance;

    use super::*;

    #[test]
    fn happy_path_native() {
        let mut deps = mock_dependencies(&[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let mock_clawback_period = 1;
        let mock_time = 1571920875;
        let mut init_env = mock_env();
        init_env.block.time = mock_time;

        let info = mock_info(&HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, init_env.clone(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create a clawback
        let create = CreateMsg {
            id: "foobar".to_string(),
            backup: HumanAddr::from("backup"),
            holder: HumanAddr::from("holder"),
            clawback_period: mock_clawback_period,
            cw20_whitelist: None,
        };
        let sender = HumanAddr::from("source");
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = HandleMsg::Create(create.clone());
        let res = handle(&mut deps, init_env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "create"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_details(&deps, "foobar".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "foobar".to_string(),
                backup: HumanAddr::from("backup"),
                holder: HumanAddr::from("holder"),
                clawback_period: mock_clawback_period,
                end_time: mock_time + mock_clawback_period,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        // withdraw it
        let id = create.id.clone();
        let info = mock_info(&create.holder, &[]);
        let mut new_env = mock_env();
        new_env.block.time = mock_time + mock_clawback_period;
        let res = handle(&mut deps, new_env.clone(), info, HandleMsg::Withdraw { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(attr("action", "withdraw"), res.attributes[0]);
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: create.holder.clone(),
                amount: balance,
            })
        );

        // second attempt fails (not found)
        let id = create.id.clone();
        let info = mock_info(&create.holder, &[]);
        let res = handle(&mut deps, new_env, info, HandleMsg::Withdraw { id });
        match res.unwrap_err() {
            ContractError::Std(StdError::NotFound { .. }) => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }

    #[test]
    fn happy_path_cw20() {
        let mut deps = mock_dependencies(&[]);
        let mock_time = 1571920875;
        let mut init_env = mock_env();
        init_env.block.time = mock_time;
        // init an empty contract
        let init_msg = InitMsg {};
        let info = mock_info(&HumanAddr::from("anyone"), &[]);

        let res = init(&mut deps, init_env.clone(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create a clawback
        let mock_clawback_period = 1;
        let create = CreateMsg {
            id: "foobar".to_string(),
            holder: HumanAddr::from("holder"),
            backup: HumanAddr::from("backup"),
            clawback_period: mock_clawback_period,
            cw20_whitelist: Some(vec![HumanAddr::from("other-token")]),
        };
        let receive = Cw20ReceiveMsg {
            sender: HumanAddr::from("source"),
            amount: Uint128(100),
            msg: Some(to_binary(&HandleMsg::Create(create.clone())).unwrap()),
        };
        let token_contract = HumanAddr::from("my-cw20-token");
        let info = mock_info(&token_contract, &[]);
        let msg = HandleMsg::Receive(receive.clone());
        let res = handle(&mut deps, init_env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "withdraw"), res.attributes[0]);
        // ensure the whitelist is what we expect
        let details = query_details(&deps, "foobar".to_string()).unwrap();

        assert_eq!(
            details,
            DetailsResponse {
                id: "foobar".to_string(),
                holder: HumanAddr::from("holder"),
                backup: HumanAddr::from("backup"),
                end_time: mock_time + mock_clawback_period,
                clawback_period: mock_clawback_period,
                native_balance: vec![],
                cw20_balance: vec![Cw20CoinHuman {
                    address: HumanAddr::from("my-cw20-token"),
                    amount: Uint128(100),
                }],
                cw20_whitelist: vec![
                    HumanAddr::from("other-token"),
                    HumanAddr::from("my-cw20-token")
                ],
            }
        );

        // withdraw it
        let id = create.id.clone();
        let info = mock_info(&create.holder, &[]);
        let mut new_env = mock_env();
        new_env.block.time = mock_time + mock_clawback_period;
        let res = handle(&mut deps, new_env.clone(), info, HandleMsg::Withdraw { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(attr("action", "withdraw"), res.attributes[0]);
        let send_msg = Cw20HandleMsg::Transfer {
            recipient: create.holder.clone(),
            amount: receive.amount,
        };
        assert_eq!(
            res.messages[0],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_contract,
                msg: to_binary(&send_msg).unwrap(),
                send: vec![],
            })
        );

        // second attempt fails (not found)
        let id = create.id.clone();
        let info = mock_info(&create.holder, &[]);
        let res = handle(&mut deps, new_env, info, HandleMsg::Withdraw { id });
        match res.unwrap_err() {
            ContractError::Std(StdError::NotFound { .. }) => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }

    #[test]
    fn remove_tokens_proper() {
        let mut tokens = GenericBalance::default();
        tokens.add_tokens(Balance::from(vec![coin(123, "atom"), coin(789, "eth")]));
        assert_eq!(
            tokens.remove_tokens(Balance::from(vec![coin(456, "atom"), coin(12, "btc")])),
            Err(())
        );
        assert_eq!(tokens.native, vec![coin(123, "atom"), coin(789, "eth")]);
        assert_eq!(
            tokens.remove_tokens(Balance::from(vec![coin(1, "atom"), coin(1, "btc")])),
            Err(())
        );
        assert_eq!(tokens.native, vec![coin(123, "atom"), coin(789, "eth")]);
        assert_eq!(
            tokens.remove_tokens(Balance::from(vec![coin(1, "atom"), coin(1, "eth")])),
            Ok(())
        );
        assert_eq!(tokens.native, vec![coin(122, "atom"), coin(788, "eth")]);
    }

    #[test]
    fn remove_cw_tokens_proper() {
        let mut tokens = GenericBalance::default();
        let bar_token = CanonicalAddr(b"bar_token".to_vec().into());
        let foo_token = CanonicalAddr(b"foo_token".to_vec().into());
        tokens.add_tokens(Balance::Cw20(Cw20Coin {
            address: foo_token.clone(),
            amount: Uint128(12345),
        }));
        assert_eq!(
            tokens.remove_tokens(Balance::Cw20(Cw20Coin {
                address: bar_token.clone(),
                amount: Uint128(777),
            })),
            Err(())
        );
        assert_eq!(
            tokens.cw20,
            vec![Cw20Coin {
                address: foo_token.clone(),
                amount: Uint128(12345),
            }]
        );
        assert_eq!(
            tokens.remove_tokens(Balance::Cw20(Cw20Coin {
                address: foo_token.clone(),
                amount: Uint128(23400),
            })),
            Err(())
        );
        assert_eq!(
            tokens.cw20,
            vec![Cw20Coin {
                address: foo_token.clone(),
                amount: Uint128(12345),
            }]
        );
        assert_eq!(
            tokens.remove_tokens(Balance::Cw20(Cw20Coin {
                address: foo_token.clone(),
                amount: Uint128(1),
            })),
            Ok(())
        );
        assert_eq!(
            tokens.cw20,
            vec![Cw20Coin {
                address: foo_token.clone(),
                amount: Uint128(12344),
            }]
        );
    }

    #[test]
    fn add_tokens_proper() {
        let mut tokens = GenericBalance::default();
        tokens.add_tokens(Balance::from(vec![coin(123, "atom"), coin(789, "eth")]));
        tokens.add_tokens(Balance::from(vec![coin(456, "atom"), coin(12, "btc")]));
        assert_eq!(
            tokens.native,
            vec![coin(579, "atom"), coin(789, "eth"), coin(12, "btc")]
        );
    }

    #[test]
    fn add_cw_tokens_proper() {
        let mut tokens = GenericBalance::default();
        let bar_token = CanonicalAddr(b"bar_token".to_vec().into());
        let foo_token = CanonicalAddr(b"foo_token".to_vec().into());
        tokens.add_tokens(Balance::Cw20(Cw20Coin {
            address: foo_token.clone(),
            amount: Uint128(12345),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20Coin {
            address: bar_token.clone(),
            amount: Uint128(777),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20Coin {
            address: foo_token.clone(),
            amount: Uint128(23400),
        }));
        assert_eq!(
            tokens.cw20,
            vec![
                Cw20Coin {
                    address: foo_token,
                    amount: Uint128(35745),
                },
                Cw20Coin {
                    address: bar_token,
                    amount: Uint128(777),
                }
            ]
        );
    }

    #[test]
    fn transfer_native() {
        let mut deps = mock_dependencies(&[]);

        // init an empty contract
        let init_msg = InitMsg {};
        let mock_clawback_period = 2;
        let mock_time = 1571920875;
        let mut init_env = mock_env();
        init_env.block.time = mock_time;

        let info = mock_info(&HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, init_env.clone(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());
        let balance = coins(100, "tokens");
        // create two clawbacks
        for idc in ["foo", "bar", "wrong-per", "wrong-back"].iter() {
            let create = CreateMsg {
                id: idc.to_string(),
                backup: if *idc == "wrong-back" {
                    HumanAddr::from("backup2")
                } else {
                    HumanAddr::from("backup")
                },
                holder: HumanAddr::from(*idc),
                clawback_period: if *idc == "wrong-per" {
                    mock_clawback_period - 1
                } else {
                    mock_clawback_period
                },
                cw20_whitelist: None,
            };
            let sender = HumanAddr::from("source");

            let info = mock_info(&sender, &balance);
            let msg = HandleMsg::Create(create.clone());

            let res = handle(&mut deps, init_env.clone(), info, msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(attr("action", "create"), res.attributes[0]);

            // ensure the details is what we expect
            let details = query_details(&deps, idc.to_string()).unwrap();
            assert_eq!(
                details,
                DetailsResponse {
                    id: idc.to_string(),
                    backup: if *idc == "wrong-back" {
                        HumanAddr::from("backup2")
                    } else {
                        HumanAddr::from("backup")
                    },
                    holder: HumanAddr::from(*idc),
                    clawback_period: if *idc == "wrong-per" {
                        mock_clawback_period - 1
                    } else {
                        mock_clawback_period
                    },
                    end_time: if *idc == "wrong-per" {
                        mock_time + mock_clawback_period - 1
                    } else {
                        mock_time + mock_clawback_period
                    },
                    native_balance: balance.clone(),
                    cw20_balance: vec![],
                    cw20_whitelist: vec![],
                }
            );
        }

        // transfer it
        let from_id = "foo".to_string();
        let to_id = "bar".to_string();
        let info = mock_info(HumanAddr::from(from_id.clone()), &[]);
        let mut new_env = mock_env();
        new_env.block.time = mock_time + 1;
        let res = handle(
            &mut deps,
            new_env.clone(),
            info,
            HandleMsg::ClawbackTransfer {
                from_id,
                to_id,
                amount: Balance::Native(NativeBalance(coins(1, "tokens"))),
            },
        )
        .unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "transfer"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_details(&deps, "foo".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "foo".to_string(),
                backup: HumanAddr::from("backup"),
                holder: HumanAddr::from("foo"),
                clawback_period: mock_clawback_period,
                end_time: mock_time + mock_clawback_period,
                native_balance: coins(99, "tokens"),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        let details = query_details(&deps, "bar".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "bar".to_string(),
                backup: HumanAddr::from("backup"),
                holder: HumanAddr::from("bar"),
                clawback_period: mock_clawback_period,
                end_time: mock_time + 1 + mock_clawback_period,
                native_balance: coins(101, "tokens"),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        // claw back
        let from_id = "bar".to_string();
        let to_id = "foo".to_string();
        let info = mock_info(HumanAddr::from("backup"), &[]);
        let res = handle(
            &mut deps,
            new_env.clone(),
            info,
            HandleMsg::ClawbackTransfer {
                from_id,
                to_id,
                amount: Balance::Native(NativeBalance(coins(1, "tokens"))),
            },
        )
        .unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "transfer"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_details(&deps, "foo".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "foo".to_string(),
                backup: HumanAddr::from("backup"),
                holder: HumanAddr::from("foo"),
                clawback_period: mock_clawback_period,
                end_time: mock_time + 1 + mock_clawback_period,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        let details = query_details(&deps, "bar".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "bar".to_string(),
                backup: HumanAddr::from("backup"),
                holder: HumanAddr::from("bar"),
                clawback_period: mock_clawback_period,
                end_time: mock_time + 1 + mock_clawback_period,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        // failures
        let from_id = "foo".to_string();
        let to_id = "wrong-per".to_string();
        let info = mock_info(HumanAddr::from(from_id.clone()), &[]);
        let res = handle(
            &mut deps,
            new_env.clone(),
            info,
            HandleMsg::ClawbackTransfer {
                from_id,
                to_id,
                amount: Balance::Native(NativeBalance(coins(1, "tokens"))),
            },
        );

        match res.unwrap_err() {
            ContractError::ContractMismatch {} => {}
            e => panic!("Expected ContractMismatch, got {}", e),
        }

        let from_id = "foo".to_string();
        let to_id = "wrong-back".to_string();
        let info = mock_info(HumanAddr::from(from_id.clone()), &[]);
        let res = handle(
            &mut deps,
            new_env.clone(),
            info,
            HandleMsg::ClawbackTransfer {
                from_id,
                to_id,
                amount: Balance::Native(NativeBalance(coins(1, "tokens"))),
            },
        );

        match res.unwrap_err() {
            ContractError::ContractMismatch {} => {}
            e => panic!("Expected ContractMismatch, got {}", e),
        }
    }

    #[test]
    fn transfer_cw20() {
        todo!();
    }

    #[test]
    fn refresh() {
        todo!();
    }

    #[test]
    fn burn() {
        todo!();
    }

    #[test]
    fn top_up_mixed_tokens() {
        let mut deps = mock_dependencies(&[]);
        let mock_time = 1571920875;
        let mut init_env = mock_env();
        init_env.block.time = mock_time;
        // init an empty contract
        let init_msg = InitMsg {};
        let info = mock_info(&HumanAddr::from("anyone"), &[]);
        let res = init(&mut deps, init_env.clone(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // only accept these tokens
        let whitelist = vec![HumanAddr::from("bar_token"), HumanAddr::from("foo_token")];

        // create a clawback with 2 native tokens
        let mock_clawback_period = 1;

        let create = CreateMsg {
            id: "foobar".to_string(),
            backup: HumanAddr::from("backup"),
            holder: HumanAddr::from("holder"),
            clawback_period: mock_clawback_period,
            cw20_whitelist: Some(whitelist),
        };
        let sender = HumanAddr::from("source");
        let balance = vec![coin(100, "fee"), coin(200, "stake")];
        let info = mock_info(&sender, &balance);
        let msg = HandleMsg::Create(create.clone());
        let res = handle(&mut deps, init_env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "create"), res.attributes[0]);

        // top it up with 2 more native tokens
        let extra_native = vec![coin(250, "random"), coin(300, "stake")];
        let info = mock_info(&sender, &extra_native);
        let top_up = HandleMsg::TopUp {
            id: create.id.clone(),
        };
        let res = handle(&mut deps, init_env.clone(), info, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "top_up"), res.attributes[0]);

        // top up with one foreign token
        let bar_token = HumanAddr::from("bar_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("random"),
            amount: Uint128(7890),
            msg: Some(to_binary(&base).unwrap()),
        });
        let info = mock_info(&bar_token, &[]);
        let res = handle(&mut deps, init_env.clone(), info, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "top_up"), res.attributes[0]);

        // top with a foreign token not on the whitelist
        // top up with one foreign token
        let baz_token = HumanAddr::from("baz_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("random"),
            amount: Uint128(7890),
            msg: Some(to_binary(&base).unwrap()),
        });
        let info = mock_info(&baz_token, &[]);
        let res = handle(&mut deps, init_env.clone(), info, top_up);
        match res.unwrap_err() {
            ContractError::NotInWhitelist {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // top up with second foreign token
        let foo_token = HumanAddr::from("foo_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("random"),
            amount: Uint128(888),
            msg: Some(to_binary(&base).unwrap()),
        });
        let info = mock_info(&foo_token, &[]);
        let res = handle(&mut deps, init_env.clone(), info, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(attr("action", "top_up"), res.attributes[0]);

        // withdraw it
        let mut new_env = mock_env();
        new_env.block.time = mock_time + mock_clawback_period;
        let id = create.id.clone();
        let info = mock_info(&create.holder, &[]);
        let res = handle(&mut deps, new_env.clone(), info, HandleMsg::Withdraw { id }).unwrap();
        assert_eq!(attr("action", "withdraw"), res.attributes[0]);
        assert_eq!(3, res.messages.len());

        // first message releases all native coins
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: create.holder.clone(),
                amount: vec![coin(100, "fee"), coin(500, "stake"), coin(250, "random")],
            })
        );

        // second one release bar cw20 token
        let send_msg = Cw20HandleMsg::Transfer {
            recipient: create.holder.clone(),
            amount: Uint128(7890),
        };
        assert_eq!(
            res.messages[1],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: bar_token,
                msg: to_binary(&send_msg).unwrap(),
                send: vec![],
            })
        );

        // third one release foo cw20 token
        let send_msg = Cw20HandleMsg::Transfer {
            recipient: create.holder.clone(),
            amount: Uint128(888),
        };
        assert_eq!(
            res.messages[2],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: foo_token,
                msg: to_binary(&send_msg).unwrap(),
                send: vec![],
            })
        );
    }
}
