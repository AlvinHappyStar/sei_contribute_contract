#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, CosmosMsg
};

use cw2::{get_contract_version, set_contract_version};
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg
};
use cw20::{Balance};
use crate::state::{
    Config, CONFIG, HISTORY
};

use crate::util;
// Version info, for migration info
const CONTRACT_NAME: &str = "contribute";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: info.sender.clone(),
        denom: cw20::Denom::Native(info.funds[0].denom.clone()),
        enabled: true,
        amount: 0u128.into(),
    };
    
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => util::execute_update_owner(deps.storage, deps.api, info.sender.clone(), owner),
        ExecuteMsg::UpdateEnabled { enabled } => util::execute_update_enabled(deps.storage, deps.api, info.sender.clone(), enabled),
        ExecuteMsg::Deposit { } => execute_deposit(deps, env, info),
        ExecuteMsg::Withdraw { } => execute_withdraw(deps, env, info),
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo
) -> Result<Response, ContractError> {

    util::check_enabled(deps.storage)?;

    let mut cfg = CONFIG.load(deps.storage)?;

    let balance = Balance::from(info.funds);

    let amount = util::get_amount_of_denom(balance, cfg.denom.clone())?;

    HISTORY.save(deps.storage, info.sender.clone(), &amount)?;

    cfg.amount += amount;
    CONFIG.save(deps.storage, &cfg)?;
    
    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "deposit"),
            attr("address", info.sender.clone()),
            attr("amount", amount),
        ]));
}



pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo
) -> Result<Response, ContractError> {

    util::check_owner(deps.storage, deps.api, info.sender.clone())?;

    let cfg = CONFIG.load(deps.storage)?;
    
    let contract_amount = util::get_token_amount_of_address(deps.querier, cfg.denom.clone(), env.contract.address.clone())?;

    let mut messages:Vec<CosmosMsg> = vec![];
    messages.push(util::transfer_token_message(deps.querier, cfg.denom.clone(), contract_amount, info.sender.clone())?);

    
    return Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("address", info.sender.clone()),
            attr("amount", contract_amount),
        ]));
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} 
            => to_binary(&query_config(deps, env)?),
        QueryMsg::HistoryMsg {address} => to_binary(&query_history(deps, address)?)
        
    }
}

pub fn query_config(deps: Deps, env: Env) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let treasury_amount = util::get_token_amount_of_address(deps.querier, cfg.denom.clone(), env.contract.address.clone()).unwrap();
    Ok(ConfigResponse {
        owner: cfg.owner,
        amount: treasury_amount,
        denom: cfg.denom,
        enabled: cfg.enabled
    })
}

fn query_history(
    deps: Deps,
    address: String
) -> StdResult<Uint128> {

    let amount = HISTORY.load(deps.storage, deps.api.addr_validate(&address)?)?;
    
    Ok(amount)
    
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

