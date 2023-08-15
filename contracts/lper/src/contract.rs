#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use covenant_clock::helpers::verify_clock;
use cw2::set_contract_version;

use astroport::{
    asset::Asset,
    pair::{ExecuteMsg::ProvideLiquidity, PoolResponse},
    DecimalCheckedOps,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ProvidedLiquidityInfo, ContractState, LpConfig},
    state::{
        ASSETS,
        HOLDER_ADDRESS, PROVIDED_LIQUIDITY_INFO,
        LP_CONFIG,
    },
};

use neutron_sdk::NeutronResult;

use crate::state::{CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = "crates.io:covenant-lp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// type QueryDeps<'a> = Deps<'a, NeutronQuery>;
// type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;
const DOUBLE_SIDED_REPLY_ID: u64 = 321u64;
const SINGLE_SIDED_REPLY_ID: u64 = 322u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: lp instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    // validate the contract addresses
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let pool_addr = deps.api.addr_validate(&msg.pool_address)?;
    let holder_addr = deps.api.addr_validate(&msg.holder_address)?;

    // contract starts at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // store the relevant module addresses
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    HOLDER_ADDRESS.save(deps.storage, &holder_addr)?;

    ASSETS.save(deps.storage, &msg.assets)?;

    let lp_config = LpConfig {
        expected_native_token_amount: msg.expected_native_token_amount,
        expected_ls_token_amount: msg.expected_ls_token_amount,
        allowed_return_delta: msg.allowed_return_delta,
        pool_address: pool_addr,
        single_side_lp_limits: msg.single_side_lp_limits,
        autostake: msg.autostake,
        slippage_tolerance: msg.slippage_tolerance,
    };
    LP_CONFIG.save(deps.storage, &lp_config)?;

    // we begin with no liquidity provided
    PROVIDED_LIQUIDITY_INFO.save(
        deps.storage,
        &ProvidedLiquidityInfo {
            provided_amount_ls: Uint128::zero(),
            provided_amount_native: Uint128::zero(),
        },
    )?;

    Ok(Response::default()
        .add_attribute("method", "lp_instantiate")
        .add_attribute("clock_addr", clock_addr)
        .add_attribute("holder_addr", holder_addr)
        .add_attribute("ls_asset_denom", msg.assets.ls_asset_denom)
        .add_attribute("native_asset_denom", msg.assets.native_asset_denom)
        .add_attributes(lp_config.to_response_attributes())
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
    }
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_lp(deps, env),
    }
}

/// method which attempts to provision liquidity to the pool.
/// if both desired asset balances are non-zero, double sided liquidity
/// is provided. 
/// otherwise, single-sided liquidity provision is attempted.
fn try_lp(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let asset_data = ASSETS.load(deps.storage)?;

    // first we query our own balances and filter out any unexpected denoms
    let bal_coins = deps.querier.query_all_balances(env.contract.address.to_string())?;
    let (native_bal, ls_bal) = get_relevant_balances(
        bal_coins,
        asset_data.ls_asset_denom,
        asset_data.native_asset_denom,
    );

    // depending on available balances we attempt a different action:
    match (native_bal.amount.is_zero(), ls_bal.amount.is_zero()) {
        // one balance is non-zero, we attempt single-side
        (true, false) | (false, true) => {
            let single_sided_submsg = try_get_single_side_lp_submsg(
                deps.branch(), 
                native_bal,
                ls_bal,
            )?;
            if let Some(msg) = single_sided_submsg {
                return Ok(Response::default()
                    .add_submessage(msg)
                    .add_attribute("method", "single_side_lp"));
            }
        },
        // both balances are non-zero, we attempt double-side
        (false, false) => {
            let double_sided_submsg = try_get_double_side_lp_submsg(
                deps.branch(),
                native_bal,
                ls_bal,
            )?;
        
            if let Some(msg) = double_sided_submsg {
                return Ok(Response::default()
                    .add_submessage(msg)
                    .add_attribute("method", "double_side_lp"));
            }
        },
        // both balances zero, no liquidity can be provisioned
        _ => (),
    }

    // if no message could be constructed, we keep waiting for funds
    Ok(Response::default()
        .add_attribute("method", "try_lp")
        .add_attribute("status", "not enough funds")
    )
}

/// attempts to get a double sided ProvideLiquidity submessage.
/// amounts here do not matter. as long as we have non-zero balances of both
/// native and ls tokens, the maximum amount of liquidity is provided to maintain
/// the existing pool ratio.
fn try_get_double_side_lp_submsg(
    deps: DepsMut,
    native_bal: Coin,
    ls_bal: Coin,
) -> Result<Option<SubMsg>, ContractError> {
    let lp_config = LP_CONFIG.load(deps.storage)?;
    let asset_data = ASSETS.load(deps.storage)?;
    let holder_address = HOLDER_ADDRESS.load(deps.storage)?;

    // we now query the pool to know the balances
    let pool_response: PoolResponse = deps
        .querier
        .query_wasm_smart(&lp_config.pool_address, &astroport::pair::QueryMsg::Pool {})?;
    let (pool_native_bal, pool_ls_bal) = get_pool_asset_amounts(
        pool_response.assets,
        asset_data.ls_asset_denom.as_str(),
        asset_data.native_asset_denom.as_str(),
    )?;

    // we validate the pool to match our price expectations
    lp_config.validate_price_range(
        pool_native_bal,
        pool_ls_bal,
    )?;

    // we derive the ratio of native to ls.
    // using this ratio we know how many native tokens we should provide for every one ls token
    // by multiplying available ls token amount by the native_to_ls_ratio.
    let native_to_ls_ratio = Decimal::from_ratio(pool_native_bal, pool_ls_bal);

    // we thus find the required token amount to enter into the position using all available ls tokens:
    let required_native_amount = native_to_ls_ratio.checked_mul_uint128(ls_bal.amount)?;

    // depending on available balances we determine the highest amount
    // of liquidity we can provide:
    let (native_asset_double_sided, ls_asset_double_sided) =
        if native_bal.amount >= required_native_amount {
            // if we are able to satisfy the required amount, we do that:
            // provide all statom tokens along with required amount of native tokens
            (
                Asset {
                    info: asset_data.get_native_asset_info(),
                    amount: required_native_amount,
                },
                Asset {
                    info: asset_data.get_ls_asset_info(),
                    amount: ls_bal.amount,
                }
            )
        } else {
            // otherwise, our native token amount is insufficient to provide double
            // sided liquidity using all of our ls tokens.
            // this means that we should provide all of our available native tokens,
            // and as many ls tokens as needed to satisfy the existing ratio
            (
                Asset {
                    info: asset_data.get_native_asset_info(),
                    amount: native_bal.amount,
                },
                Asset {
                    info: asset_data.get_ls_asset_info(),
                    amount: Decimal::from_ratio(pool_ls_bal, pool_native_bal)
                        .checked_mul_uint128(native_bal.amount)?,
                },
            )
        };

    let (native_coin, ls_coin) = (
        native_asset_double_sided.to_coin()?,
        ls_asset_double_sided.to_coin()?,
    );
    
    // craft a ProvideLiquidity message with the determined assets
    let double_sided_liq_msg = ProvideLiquidity {
        assets: vec![
            native_asset_double_sided,
            ls_asset_double_sided,
        ],
        slippage_tolerance: lp_config.slippage_tolerance,
        auto_stake: lp_config.autostake,
        receiver: Some(holder_address.to_string()),
    };

    // update the provided amounts and leftover assets
    PROVIDED_LIQUIDITY_INFO.update(
        deps.storage,
        |mut info: ProvidedLiquidityInfo| -> StdResult<_> {
            info.provided_amount_ls = info
                .provided_amount_ls
                .checked_add(ls_coin.amount)?;
            info.provided_amount_native = info
                .provided_amount_native
                .checked_add(native_coin.amount)?;
            Ok(info)
        },
    )?;

    Ok(Some(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: lp_config.pool_address.to_string(),
            msg: to_binary(&double_sided_liq_msg)?,
            funds: vec![native_coin, ls_coin],
        }),
        DOUBLE_SIDED_REPLY_ID,
    )))
}

/// attempts to build a single sided `ProvideLiquidity` message.
/// pool ratio does not get validated here. as long as the single
/// side asset amount being provided is within our predefined
/// single-side liquidity limits, we provide it.
fn try_get_single_side_lp_submsg(
    deps: DepsMut,
    native_bal: Coin,
    ls_bal: Coin,
) -> Result<Option<SubMsg>, ContractError> {
    let asset_data = ASSETS.load(deps.storage)?;
    let holder_address = HOLDER_ADDRESS.load(deps.storage)?;
    let lp_config = LP_CONFIG.load(deps.storage)?;

    let assets = asset_data.to_asset_vec(native_bal.amount, ls_bal.amount);

    // given one non-zero asset, we build the ProvideLiquidity message
    let single_sided_liq_msg = ProvideLiquidity {
        assets,
        slippage_tolerance: lp_config.slippage_tolerance,
        auto_stake: lp_config.autostake,
        receiver: Some(holder_address.to_string()),
    };

    // now we try to submit the message for either LS or native single side liquidity
    if native_bal.amount.is_zero() && ls_bal.amount <= lp_config.single_side_lp_limits.ls_asset_limit {
        // update the provided liquidity info
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_ls = info.provided_amount_ls.checked_add(ls_bal.amount)?;
            Ok(info)
        })?;

        // if available ls token amount is within single side limits we build a single side msg
        let submsg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: lp_config.pool_address.to_string(),
                msg: to_binary(&single_sided_liq_msg)?,
                funds: vec![ls_bal],
            }),
            SINGLE_SIDED_REPLY_ID,
        );

        return Ok(Some(submsg));
    } else if ls_bal.amount.is_zero()
        && native_bal.amount <= lp_config.single_side_lp_limits.native_asset_limit {
        // update the provided liquidity info
        PROVIDED_LIQUIDITY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
            info.provided_amount_native =
                info.provided_amount_native.checked_add(native_bal.amount)?;
            Ok(info)
        })?;

        // if available native token amount is within single side limits we build a single side msg
        let submsg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: lp_config.pool_address.to_string(),
                msg: to_binary(&single_sided_liq_msg)?,
                funds: vec![native_bal],
            }),
            SINGLE_SIDED_REPLY_ID,
        );

        return Ok(Some(submsg));
    }

    // if neither ls or native token single side lp message was built, we just go back and wait
    Ok(None)
}

/// filters out a vector of `Coin`s to retrieve ones with ls/native denoms
fn get_relevant_balances(coins: Vec<Coin>, ls_denom: String, native_denom: String) -> (Coin, Coin) {
    let (mut native_bal, mut ls_bal) = (Coin::default(), Coin::default());

    for c in coins {
        if c.denom == ls_denom {
            // found ls balance
            ls_bal = c;
        } else if c.denom == native_denom {
            // found native token balance
            native_bal = c;
        }
    }
    (native_bal, ls_bal)
}

/// filters out irrelevant balances and returns ls and native amounts
fn get_pool_asset_amounts(
    assets: Vec<Asset>,
    ls_denom: &str,
    native_denom: &str,
) -> Result<(Uint128, Uint128), StdError> {
    let (mut native_bal, mut ls_bal) = (Uint128::zero(), Uint128::zero());

    for asset in assets {
        let coin = asset.to_coin()?;
        if coin.denom == ls_denom {
            // found ls balance
            ls_bal = coin.amount;
        } else if coin.denom == native_denom {
            // found native token balance
            native_bal = coin.amount;
        }
    }

    Ok((native_bal, ls_bal))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::Assets {} => Ok(to_binary(&ASSETS.may_load(deps.storage)?)?),
        QueryMsg::LpConfig {} => Ok(to_binary(&LP_CONFIG.may_load(deps.storage)?)?),
        // the deposit address for LP module is the contract itself
        QueryMsg::DepositAddress {} => Ok(to_binary(&Some(&env.contract.address.to_string()))?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> NeutronResult<Response> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            holder_address,
            assets,
            lp_config,
        } => {
            let mut response = Response::default().add_attribute("method", "update_config");

            if let Some(clock_addr) = clock_addr {
                CLOCK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&clock_addr)?)?;
                response = response.add_attribute("clock_addr", clock_addr);
            }

            if let Some(holder_address) = holder_address {
                HOLDER_ADDRESS.save(deps.storage, &deps.api.addr_validate(&holder_address)?)?;
                response = response.add_attribute("holder_address", holder_address);
            }

            if let Some(denoms) = assets {
                ASSETS.save(deps.storage, &denoms)?;
                response = response.add_attribute("ls_denom", denoms.ls_asset_denom.to_string());
                response = response.add_attribute("native_denom", denoms.native_asset_denom.to_string());
            }

            if let Some(config) = lp_config {
                deps.api.addr_validate(config.pool_address.as_str())?;
                LP_CONFIG.save(deps.storage, &config)?;
                response = response.add_attributes(config.to_response_attributes());
            }

            Ok(response)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            // This is a migrate message to update code id,
            // Data is optional base64 that we can parse to any data we would like in the future
            // let data: SomeStruct = from_binary(&data)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: reply");
    match msg.id {
        DOUBLE_SIDED_REPLY_ID => handle_double_sided_reply_id(deps, _env, msg),
        SINGLE_SIDED_REPLY_ID => handle_single_sided_reply_id(deps, _env, msg),
        _ => Err(ContractError::from(StdError::GenericErr {
            msg: "err".to_string(),
        })),
    }
}

fn handle_double_sided_reply_id(
    _deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    Ok(Response::default()
        .add_attribute("method", "handle_double_sided_reply_id")
        .add_attribute("reply_id", msg.id.to_string()))
}

fn handle_single_sided_reply_id(
    _deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    Ok(Response::default()
        .add_attribute("method", "handle_single_sided_reply_id")
        .add_attribute("reply_id", msg.id.to_string()))
}
