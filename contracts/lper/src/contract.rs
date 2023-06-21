#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{MessageInfo,  Response,
     StdResult, Addr, DepsMut, Env, Binary, Deps, to_binary, SubMsg, WasmMsg, CosmosMsg, Coin, Uint128, Reply, 
};
use cw2::set_contract_version;

use astroport::{pair::{ExecuteMsg::ProvideLiquidity, Cw20HookMsg}, asset::{Asset, AssetInfo}};
use cw20::Cw20ReceiveMsg;

use crate::{msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg}, state::{HOLDER_ADDRESS, LP_POSITION}};

use neutron_sdk::{
    bindings::{
        msg::{NeutronMsg},
        query::{NeutronQuery},
    },
    NeutronResult,
};

use crate::state::{
   CLOCK_ADDRESS, CONTRACT_STATE, ContractState,
};


const CONTRACT_NAME: &str = "crates.io:stride-lper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO: validations
    CLOCK_ADDRESS.save(deps.storage, &Addr::unchecked(msg.clock_address))?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    LP_POSITION.save(deps.storage, &msg.lp_position)?;
    HOLDER_ADDRESS.save(deps.storage, &msg.holder_address)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    deps.api
        .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::WithdrawRewards {} => try_withdraw(deps, env, info),
    }
}


fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    let current_state = CONTRACT_STATE.load(deps.storage)?;

    match current_state {
        ContractState::Instantiated => try_enter_lp_position(deps, env, info),
        ContractState::LpPositionEntered => no_op(),
        ContractState::LpPositionExited => no_op(),
        ContractState::WithdrawComplete => no_op(),
    }
}

fn no_op() -> NeutronResult<Response<NeutronMsg>> {
    Ok(Response::default())
}

fn try_enter_lp_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo, 
) -> NeutronResult<Response<NeutronMsg>> {
    let pool_address = LP_POSITION.load(deps.storage)?;

    // get balances of uatom and statom
    let balances: Vec<Coin> = deps.querier.query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|coin| coin.denom == "uatom" || coin.denom == "statom")
        .collect();
    
    // generate astroport Assets from balances
    let assets: Vec<Asset> = balances.clone().into_iter()
        .map(|bal| Asset {
            info: AssetInfo::NativeToken { 
                denom: bal.denom,
            },
            amount: bal.amount,
        })
        .collect();

    let provide_liquidity_msg = ProvideLiquidity { 
        assets,
        slippage_tolerance: None,
        auto_stake: Some(true),
        receiver: None,
    };
 
    Ok(Response::default().add_message(
        CosmosMsg::Wasm(WasmMsg::Execute { 
            contract_addr: pool_address.addr,
            msg: to_binary(&provide_liquidity_msg)?,
            funds: balances,
        })
    ))

}

fn try_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo, 
) -> NeutronResult<Response<NeutronMsg>> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    // todo
    let withdraw_liquidity_msg = Cw20HookMsg::WithdrawLiquidity {
        assets: vec![

        ],
    };

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: env.contract.address.to_string(),
        amount: Uint128::new(1),
        msg: to_binary(&withdraw_liquidity_msg)?,
    };

    let msg = WasmMsg::Execute { 
        contract_addr: pool_address.addr,
        msg: to_binary(&astroport::pair::ExecuteMsg::Receive(cw20_receive_msg))?,
        funds: vec![],
    };

    Ok(Response::default().add_message(
        CosmosMsg::Wasm(msg)
    ))
    
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, env: Env, msg: QueryMsg) -> NeutronResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(
            to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?
        ),
        QueryMsg::LpPosition {} => Ok(
            to_binary(&LP_POSITION.may_load(deps.storage)?)?
        )
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");
    Ok(Response::default())
}
