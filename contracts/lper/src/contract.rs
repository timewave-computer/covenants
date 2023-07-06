use cosmos_sdk_proto::cosmos::bank::v1beta1::SendAuthorization;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use astroport::{
    asset::{Asset, AssetInfo},
    pair::{Cw20HookMsg, ExecuteMsg::ProvideLiquidity, SimulationResponse},
};
use cw20::Cw20ReceiveMsg;

use crate::error::ContractError;
use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{ASSETS, AUTOSTAKE, HOLDER_ADDRESS, LP_POSITION, SLIPPAGE_TOLERANCE},
};

use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

use crate::state::{ContractState, CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-lp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO: validations
    CLOCK_ADDRESS.save(deps.storage, &Addr::unchecked(msg.clock_address))?;
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;
    LP_POSITION.save(deps.storage, &msg.lp_position)?;
    HOLDER_ADDRESS.save(deps.storage, &msg.holder_address)?;
    let assets: Vec<Asset> = msg.assets.into_iter().map(|asset: Asset| asset).collect();

    ASSETS.save(deps.storage, &assets)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // deps.api
    //     .debug(format!("WASMDEBUG: execute: received msg: {:?}", msg).as_str());
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::WithdrawLiquidity {} => try_withdraw(deps, env, info),
    }
}

fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let current_state = CONTRACT_STATE.load(deps.storage)?;

    match current_state {
        ContractState::Instantiated => try_enter_lp_position(deps, env, info),
        ContractState::LpPositionEntered => no_op(),
        ContractState::LpPositionExited => no_op(),
        ContractState::WithdrawComplete => no_op(),
    }
}

fn no_op() -> Result<Response, ContractError> {
    Ok(Response::default())
}

fn try_enter_lp_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let slippage_tolerance = SLIPPAGE_TOLERANCE.may_load(deps.storage)?;
    let auto_stake = AUTOSTAKE.may_load(deps.storage)?;
    let assets = ASSETS.load(deps.storage)?;
    let first_asset = &assets[0];
    let second_asset = &assets[1];

    // let sim_msg = astroport::pair::QueryMsg::Simulation {
    //     offer_asset: Asset {
    //         info: first_asset.to_owned().info,
    //         amount: Uint128::new(10),
    //     },
    //     ask_asset_info: None,
    // };

    // deps.api.debug(&format!("\nWASMDEBUG simulation msg: {:?}\n", sim_msg));
    // // figure out how much of second asset can we get with one of first asset
    // let simulation: SimulationResponse = deps.querier.query_wasm_smart(
    //     &pool_address.addr,
    //     &sim_msg
    // )?;
    // deps.api.debug(&format!("\nWASMDEBUG SIMULATION: {:?}\n", simulation));

    let balances: Vec<Coin> = deps
        .querier
        .query_all_balances(env.contract.clone().address)?
        .into_iter()
        .filter(|mut coin| {
            let mut valid_balance = false;
            for asset in assets.clone() {
                match asset.info {
                    AssetInfo::Token { contract_addr } => {
                        if coin.denom == contract_addr {
                            valid_balance = false
                        }
                    }
                    AssetInfo::NativeToken { denom } => {
                        if coin.denom == denom {
                            valid_balance = true
                        }
                    }
                }
            }
            valid_balance
        })
        .map(|mut c| {
            c.amount = Uint128::new(50000);
            c
        })
        .collect();

    deps.api
        .debug(&format!("WASMDEBUG: balances: {:?}", balances));

    // generate astroport Assets from balances
    let assets: Vec<Asset> = balances
        .clone()
        .into_iter()
        .map(|bal| Asset {
            info: AssetInfo::NativeToken { denom: bal.denom },
            amount: bal.amount,
        })
        .collect();

    let provide_liquidity_msg = ProvideLiquidity {
        assets,
        slippage_tolerance,
        auto_stake,
        receiver: Some(env.contract.address.to_string()),
    };
    deps.api.debug(&format!(
        "WASMDEBUG: sending provide liquidity: {:?}\n\n",
        provide_liquidity_msg
    ));

    Ok(
        Response::default().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pool_address.addr.to_string(),
            msg: to_binary(&provide_liquidity_msg)?,
            funds: balances.clone(),
        })),
    )
}

/// should be sent to the LP token contract associated with the pool
/// to withdraw liquidity from
fn try_withdraw(deps: DepsMut, _env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let pool_address = LP_POSITION.load(deps.storage)?;
    let assets = ASSETS.load(deps.storage)?;
    deps.api.debug(&format!("withdraw assets: {:?}", assets));

    let withdraw_liquidity_msg = Cw20HookMsg::WithdrawLiquidity { assets };

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: Addr::unchecked("contract5").to_string(),
        // sender: info.sender.to_string(),
        amount: Uint128::new(1),
        msg: to_binary(&withdraw_liquidity_msg)?,
    };

    let msg = WasmMsg::Execute {
        contract_addr: pool_address.addr,
        msg: to_binary(&astroport::pair::ExecuteMsg::Receive(cw20_receive_msg))?,
        funds: vec![],
    };

    // Ok(Response::default().add_submessage(
    //     SubMsg::new(CosmosMsg::Wasm(msg))
    // ))

    Ok(Response::default().add_message(CosmosMsg::Wasm(msg)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::LpPosition {} => Ok(to_binary(&LP_POSITION.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::Assets {} => Ok(to_binary(&ASSETS.may_load(deps.storage)?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            lp_position,
            holder_address,
            assets,
        } => {
            if let Some(clock_addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&clock_addr)?)?;
            }

            if let Some(lp_position) = lp_position {
                LP_POSITION.save(deps.storage, &lp_position)?;
            }

            if let Some(holder_address) = holder_address {
                HOLDER_ADDRESS.save(deps.storage, &holder_address)?;
            }

            if let Some(assets) = assets {
                ASSETS.save(deps.storage, &assets)?;
            }

            Ok(Response::default())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: reply");
    Ok(Response::new())
}
