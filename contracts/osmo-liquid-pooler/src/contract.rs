use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, to_json_binary, to_json_string, Addr, Attribute, Binary, Coin, CosmosMsg,
    Decimal, Env, Fraction, IbcTimeout, MessageInfo, Response, StdError, StdResult, Uint128,
    WasmMsg,
};
use covenant_utils::{
    migrate_helper::get_recover_msg, polytone::get_polytone_execute_msg_binary,
    withdraw_lp_helper::WithdrawLPMsgs, ForwardMetadata, PacketMetadata,
};
use cw2::{get_contract_version, set_contract_version};
use neutron_sdk::{
    bindings::{
        msg::{IbcFee, NeutronMsg},
        query::NeutronQuery,
    },
    query::min_ibc_fee::MinIbcFeeResponse,
    sudo::msg::RequestPacketTimeoutHeight,
    NeutronError, NeutronResult,
};
use polytone::callbacks::CallbackRequest;
use semver::Version;
use valence_clock::helpers::{enqueue_msg, verify_clock};
use valence_outpost_osmo_liquid_pooler::msg::OutpostWithdrawLiquidityConfig;

use crate::{
    error::ContractError,
    msg::{
        ContractState, ExecuteMsg, IbcConfig, InstantiateMsg, LiquidityProvisionConfig, MigrateMsg,
        PartyChainInfo, QueryMsg,
    },
    polytone_handlers::{
        get_ibc_pfm_withdraw_coin_message, get_ibc_withdraw_coin_message,
        get_note_execute_neutron_msg, get_proxy_query_balances_message, try_handle_callback,
    },
    state::{
        HOLDER_ADDRESS, IBC_CONFIG, LIQUIDITY_PROVISIONING_CONFIG, NOTE_ADDRESS,
        POLYTONE_CALLBACKS, PROXY_ADDRESS,
    },
};

use crate::state::{CLOCK_ADDRESS, CONTRACT_STATE};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const PROVIDE_LIQUIDITY_CALLBACK_ID: u8 = 1;
pub(crate) const PROXY_BALANCES_QUERY_CALLBACK_ID: u8 = 2;
pub(crate) const CREATE_PROXY_CALLBACK_ID: u8 = 3;
pub(crate) const WITHDRAW_LIQUIDITY_CALLBACK_ID: u8 = 4;

type ExecuteDeps<'a> = cosmwasm_std::DepsMut<'a, NeutronQuery>;
type QueryDeps<'a> = cosmwasm_std::Deps<'a, NeutronQuery>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate the contract addresses
    let clock_addr = deps.api.addr_validate(&msg.clock_address)?;
    let holder_addr = deps.api.addr_validate(&msg.holder_address)?;
    let note_addr = deps.api.addr_validate(&msg.note_address)?;

    // contract starts at Instantiated state
    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // store the relevant contract addresses
    CLOCK_ADDRESS.save(deps.storage, &clock_addr)?;
    HOLDER_ADDRESS.save(deps.storage, &holder_addr)?;
    NOTE_ADDRESS.save(deps.storage, &note_addr)?;

    // initialize polytone state sync related items
    let latest_balances: HashMap<String, Coin> = HashMap::new();
    let lp_config = LiquidityProvisionConfig {
        latest_balances,
        party_1_denom_info: msg.party_1_denom_info,
        party_2_denom_info: msg.party_2_denom_info,
        pool_id: msg.pool_id,
        outpost: msg.osmo_outpost,
        lp_token_denom: msg.lp_token_denom,
        slippage_tolerance: msg.slippage_tolerance,
        pool_price_config: msg.pool_price_config,
        funding_duration: msg.funding_duration,
        single_side_lp_limits: msg.single_side_lp_limits,
    };
    LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

    let ibc_config = IbcConfig {
        party_1_chain_info: msg.party_1_chain_info,
        party_2_chain_info: msg.party_2_chain_info,
        osmo_to_neutron_channel_id: msg.osmo_to_neutron_channel_id,
        osmo_ibc_timeout: msg.osmo_ibc_timeout,
    };
    IBC_CONFIG.save(deps.storage, &ibc_config)?;

    Ok(Response::default()
        .add_message(enqueue_msg(clock_addr.as_str())?)
        .add_attribute("method", "osmosis_lp_instantiate")
        .add_attribute("contract_state", "instantiated")
        .add_attributes(lp_config.to_response_attributes())
        .add_attributes(ibc_config.to_response_attributes())
        .add_attribute("note_address", note_addr)
        .add_attribute("holder_address", holder_addr)
        .add_attribute("clock_address", clock_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: ExecuteDeps,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        ExecuteMsg::Tick {} => try_tick(deps, env, info),
        ExecuteMsg::Callback(callback_msg) => try_handle_callback(env, deps, info, callback_msg),
        ExecuteMsg::Withdraw { percentage } => try_initiate_withdrawal(deps, info, percentage),
        ExecuteMsg::RecoverFunds { denoms } => {
            let holder_addr = HOLDER_ADDRESS.load(deps.storage)?;

            // query the holder for emergency commitee address
            let commitee_raw_query = deps
                .querier
                .query_wasm_raw(holder_addr.to_string(), b"e_c_a".as_slice())?;
            let emergency_commitee: Addr = if let Some(resp) = commitee_raw_query {
                let resp: Addr = from_json(resp)?;
                resp
            } else {
                return Err(ContractError::Std(StdError::generic_err(
                    "emergency committee address not found",
                ))
                .to_neutron_std());
            };

            // validate emergency committee as caller
            ensure!(
                info.sender == emergency_commitee,
                ContractError::Std(StdError::generic_err(
                    "only emergency committee can recover funds"
                ))
                .to_neutron_std()
            );

            // collect available denom coins into a bank send
            let recover_msg = get_recover_msg(
                deps.into_empty(),
                env,
                denoms,
                emergency_commitee.to_string(),
            )?;
            Ok(Response::new().add_message(recover_msg))
        }
    }
}

/// we initiate the withdrawal phase by setting the contract state to `PendingWithdrawal`
fn try_initiate_withdrawal(
    deps: ExecuteDeps,
    info: MessageInfo,
    percentage: Option<Decimal>,
) -> NeutronResult<Response<NeutronMsg>> {
    ensure!(
        info.sender == HOLDER_ADDRESS.load(deps.storage)?,
        ContractError::NotHolder {}.to_neutron_std()
    );

    let withdraw_share = percentage.unwrap_or(Decimal::one());

    ensure!(
        withdraw_share <= Decimal::one() && withdraw_share > Decimal::zero(),
        ContractError::Std(StdError::generic_err(format!(
            "withdraw percentage must be in range (0, 1], got {:?}",
            withdraw_share
        ),))
        .to_neutron_std()
    );

    // we advance the contract state to `PendingWithdrawal` and force latest balances sync
    CONTRACT_STATE.save(
        deps.storage,
        &ContractState::PendingWithdrawal {
            share: withdraw_share,
        },
    )?;
    LIQUIDITY_PROVISIONING_CONFIG.update(deps.storage, |mut lp_config| -> StdResult<_> {
        lp_config.reset_latest_proxy_balances();
        Ok(lp_config)
    })?;

    Ok(Response::default()
        .add_attribute("method", "try_initiate_withdrawal")
        .add_attribute("contract_state", "pending_withdrawal")
        .add_attribute("pending_withdrawal", withdraw_share.to_string()))
}

/// this method will attempt to set the contract state to `Distributing`,
/// with any non-lp token denoms available on our remote proxy.
/// doing so will cause upcoming ticks to try to send the withdrawn coins
/// to the holder, completing the withdrawal flow and reverting this contract
/// to active state.
fn withdraw_party_denoms(
    deps: ExecuteDeps,
    p1_proxy_bal: &Coin,
    p2_proxy_bal: &Coin,
) -> NeutronResult<Response<NeutronMsg>> {
    // if either denom balance is non-zero, collect them
    let mut withdraw_coins: Vec<Coin> = Vec::with_capacity(2);
    if p1_proxy_bal.amount > Uint128::zero() {
        withdraw_coins.push(p1_proxy_bal.clone());
    }
    if p2_proxy_bal.amount > Uint128::zero() {
        withdraw_coins.push(p2_proxy_bal.clone());
    }

    if withdraw_coins.is_empty() {
        // both both balances are zero, we revert the state to active
        // and ping a WithdrawFailed message to the holder.
        let withdraw_failed_msg = WasmMsg::Execute {
            contract_addr: HOLDER_ADDRESS.load(deps.storage)?.to_string(),
            msg: to_json_binary(&WithdrawLPMsgs::WithdrawFailed {})?,
            funds: vec![],
        };
        CONTRACT_STATE.save(deps.storage, &ContractState::Active)?;
        Ok(Response::default()
            .add_attribute("method", "withdraw_party_denoms")
            .add_attribute("contract_state", "active")
            .add_attribute("p1_balance", p1_proxy_bal.to_string())
            .add_attribute("p2_balance", p2_proxy_bal.to_string())
            .add_message(withdraw_failed_msg))
    } else {
        // otherwise we set the contract state to distributing.
        // this will cause incoming ticks to assert whether this contract had received the funds.
        // if the funds are not received, the tick will attempt to withdraw them again.
        // if all expected coins are received, the contract will submit `Distribute` message to the holder.
        CONTRACT_STATE.save(
            deps.storage,
            &ContractState::Distributing {
                coins: withdraw_coins.clone(),
            },
        )?;

        Ok(Response::default()
            .add_attribute("method", "withdraw_party_denoms")
            .add_attribute("contract_state", "distributing")
            .add_attribute("p1_balance", p1_proxy_bal.to_string())
            .add_attribute("p2_balance", p2_proxy_bal.to_string())
            .add_attribute("coins", to_json_string(&withdraw_coins)?))
    }
}

pub fn try_withdraw(
    deps: ExecuteDeps,
    env: Env,
    withdraw_share: Decimal,
    (party_1_bal, party_2_bal, lp_bal): (&Coin, &Coin, &Coin),
    lp_config: LiquidityProvisionConfig,
) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;

    // if there are 0 available lp token balances, we attempt to
    // withdraw the party denoms directly.
    if lp_bal.amount.is_zero() {
        return withdraw_party_denoms(deps, party_1_bal, party_2_bal);
    }

    let lp_redeem_amount = lp_bal
        .amount
        .checked_multiply_ratio(withdraw_share.numerator(), withdraw_share.denominator())
        .map_err(|e| ContractError::CheckedMultiplyError(e).to_neutron_std())?;

    let exit_pool_message: CosmosMsg = WasmMsg::Execute {
        contract_addr: lp_config.outpost.to_string(),
        msg: to_json_binary(
            &valence_outpost_osmo_liquid_pooler::msg::ExecuteMsg::WithdrawLiquidity {
                config: OutpostWithdrawLiquidityConfig {
                    pool_id: lp_config.pool_id,
                },
            },
        )?,
        funds: vec![Coin {
            denom: lp_config.lp_token_denom.to_string(),
            amount: lp_redeem_amount,
        }],
    }
    .into();

    POLYTONE_CALLBACKS.save(
        deps.storage,
        format!(
            "neutron_try_withdraw_liquidity : {:?}",
            env.block.height.to_string()
        ),
        &to_json_string(&exit_pool_message)?,
    )?;

    let exit_pool_note_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: get_polytone_execute_msg_binary(
            vec![exit_pool_message],
            Some(CallbackRequest {
                receiver: env.contract.address.to_string(),
                msg: to_json_binary(&WITHDRAW_LIQUIDITY_CALLBACK_ID)?,
            }),
            ibc_config.osmo_ibc_timeout,
        )?,
        funds: vec![],
    });

    Ok(Response::default().add_messages(vec![exit_pool_note_msg]))
}

/// attempts to advance the state machine. performs `info.sender` validation.
fn try_tick(deps: ExecuteDeps, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    match CONTRACT_STATE.load(deps.storage)? {
        // create a proxy account
        ContractState::Instantiated => try_create_proxy(deps, env),
        // fund the proxy account
        ContractState::ProxyCreated => try_deliver_funds(deps, env),
        // attempt to provide liquidity
        ContractState::ProxyFunded { funding_expiration } => {
            // the funding expiration is due, we advance the state to
            // Active. it will enable withdrawals and start pulling
            // any non-LP tokens from proxy back to this contract.
            if funding_expiration.is_expired(&env.block) {
                CONTRACT_STATE.save(deps.storage, &ContractState::Active)?;
                Ok(Response::default()
                    .add_attribute("method", "tick")
                    .add_attribute("contract_state", "active"))
            } else {
                // otherwise we attempt to provide liquidity
                try_provide_liquidity(deps, env)
            }
        }
        ContractState::Active => try_sync_proxy_balances(deps, env),
        ContractState::PendingWithdrawal { share } => {
            let lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
            match lp_config.get_proxy_balances() {
                Some((party_1_bal, party_2_bal, lp_bal)) => try_withdraw(
                    deps,
                    env,
                    share,
                    (party_1_bal, party_2_bal, lp_bal),
                    lp_config.clone(),
                ),
                None => try_sync_proxy_balances(deps, env),
            }
        }
        ContractState::Distributing { coins } => try_distribute(deps, env, coins),
    }
}

fn try_distribute(
    deps: ExecuteDeps,
    env: Env,
    coins: Vec<Coin>,
) -> NeutronResult<Response<NeutronMsg>> {
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    // query our own relevant token denoms
    let denom_1_balance = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.party_1_denom_info.local_denom.to_string(),
    )?;
    let denom_2_balance = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.party_2_denom_info.local_denom.to_string(),
    )?;

    // we map the withdrawn coins to match the party denoms
    let withdrawn_coin_1 = match coins
        .iter()
        .find(|c| c.denom == lp_config.party_1_denom_info.osmosis_coin.denom)
    {
        Some(c) => c.clone(),
        None => Coin {
            denom: lp_config.party_1_denom_info.osmosis_coin.denom.to_string(),
            amount: Uint128::zero(),
        },
    };

    let withdrawn_coin_2 = match coins
        .iter()
        .find(|c| c.denom == lp_config.party_2_denom_info.osmosis_coin.denom)
    {
        Some(c) => c.clone(),
        None => Coin {
            denom: lp_config.party_2_denom_info.osmosis_coin.denom.to_string(),
            amount: Uint128::zero(),
        },
    };

    // verify if denom 1 was successfully withdrawn
    let denom_1_withdrawn = denom_1_balance.amount >= withdrawn_coin_1.amount;
    let denom_2_withdrawn = denom_2_balance.amount >= withdrawn_coin_2.amount;

    // send tokens to holder and revert state to active
    if denom_1_withdrawn && denom_2_withdrawn {
        let holder_addr = HOLDER_ADDRESS.load(deps.storage)?;
        CONTRACT_STATE.save(deps.storage, &ContractState::Active)?;

        // submit a Distribute message to the holder,
        // which carries the withdrawn coins along.
        // this is basically a callback confirming that
        // the withdrawal flow had been handled successfully.
        let holder_distribute_callback_msg = WasmMsg::Execute {
            contract_addr: holder_addr.to_string(),
            msg: to_json_binary(&WithdrawLPMsgs::Distribute {})?,
            funds: vec![denom_1_balance, denom_2_balance],
        };

        // reset the proxy balances to trigger a query
        lp_config.reset_latest_proxy_balances();
        LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

        return Ok(Response::default()
            .add_attribute("method", "holder_distribute_callback")
            .add_message(holder_distribute_callback_msg));
    }

    let mut ibc_withdraw_msgs: Vec<CosmosMsg> = vec![];
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    let mut attributes: Vec<Attribute> = vec![
        Attribute::new("withdrawn_coin_1", to_json_string(&withdrawn_coin_1)?),
        Attribute::new("withdrawn_coin_2", to_json_string(&withdrawn_coin_2)?),
        Attribute::new("denom_1_balance", to_json_string(&denom_1_balance)?),
        Attribute::new("denom_2_balance", to_json_string(&denom_2_balance)?),
    ];

    if !denom_1_withdrawn && !withdrawn_coin_1.amount.is_zero() {
        // withdraw denom 1
        match &ibc_config.party_1_chain_info.inwards_pfm {
            Some(forward_metadata) => {
                let msg = get_ibc_pfm_withdraw_coin_message(
                    forward_metadata.channel.to_string(),
                    proxy_address.to_string(),
                    forward_metadata.receiver.to_string(),
                    withdrawn_coin_1,
                    env.block
                        .time
                        .plus_seconds(2 * ibc_config.osmo_ibc_timeout.u64())
                        .nanos(),
                    to_json_string(&PacketMetadata {
                        forward: Some(ForwardMetadata {
                            receiver: env.contract.address.to_string(),
                            port: forward_metadata.port.to_string(),
                            channel: ibc_config.party_1_chain_info.party_chain_to_neutron_channel,
                        }),
                    })?,
                );
                attributes.push(Attribute::new(
                    "denom_2_ibc_distribution",
                    to_json_string(&msg)?,
                ));
                ibc_withdraw_msgs.push(msg);
            }
            None => {
                let msg = get_ibc_withdraw_coin_message(
                    ibc_config.osmo_to_neutron_channel_id.to_string(),
                    env.contract.address.to_string(),
                    withdrawn_coin_1,
                    IbcTimeout::with_timestamp(
                        env.block
                            .time
                            .plus_seconds(2 * ibc_config.osmo_ibc_timeout.u64()),
                    ),
                );
                attributes.push(Attribute::new(
                    "denom_2_ibc_distribution",
                    to_json_string(&msg)?,
                ));
                ibc_withdraw_msgs.push(msg);
            }
        }
    }
    if !denom_2_withdrawn && !withdrawn_coin_2.amount.is_zero() {
        // withdraw denom 2
        match &ibc_config.party_2_chain_info.inwards_pfm {
            Some(forward_metadata) => {
                let msg = get_ibc_pfm_withdraw_coin_message(
                    forward_metadata.channel.to_string(),
                    proxy_address.to_string(),
                    forward_metadata.receiver.to_string(),
                    withdrawn_coin_2,
                    env.block
                        .time
                        .plus_seconds(2 * ibc_config.osmo_ibc_timeout.u64())
                        .nanos(),
                    to_json_string(&PacketMetadata {
                        forward: Some(ForwardMetadata {
                            receiver: env.contract.address.to_string(),
                            port: forward_metadata.port.to_string(),
                            channel: ibc_config.party_2_chain_info.party_chain_to_neutron_channel,
                        }),
                    })?,
                );
                attributes.push(Attribute::new(
                    "denom_2_ibc_distribution",
                    to_json_string(&msg)?,
                ));
                ibc_withdraw_msgs.push(msg);
            }
            None => {
                let msg = get_ibc_withdraw_coin_message(
                    ibc_config.osmo_to_neutron_channel_id.to_string(),
                    env.contract.address.to_string(),
                    withdrawn_coin_2,
                    IbcTimeout::with_timestamp(
                        env.block
                            .time
                            .plus_seconds(2 * ibc_config.osmo_ibc_timeout.u64()),
                    ),
                );
                attributes.push(Attribute::new(
                    "denom_2_ibc_distribution",
                    to_json_string(&msg)?,
                ));
                ibc_withdraw_msgs.push(msg);
            }
        }
    }

    if !ibc_withdraw_msgs.is_empty() {
        let note_msg = get_note_execute_neutron_msg(
            ibc_withdraw_msgs,
            ibc_config.osmo_ibc_timeout,
            note_address,
            None,
        )?;
        Ok(Response::default()
            .add_attributes(attributes)
            .add_message(note_msg))
    } else {
        Ok(Response::default().add_attributes(attributes))
    }
}

fn try_sync_proxy_balances(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    // get the message to query proxy for its balances
    let note_query_balances_msg = get_proxy_query_balances_message(
        env,
        proxy_address,
        note_address.to_string(),
        lp_config.clone(),
        ibc_config,
        PROXY_BALANCES_QUERY_CALLBACK_ID,
    )?;
    // reset the proxy balances as they will be updated
    lp_config.reset_latest_proxy_balances();
    LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

    Ok(Response::default().add_message(note_query_balances_msg))
}

/// fires an empty message to the note contract. this in turn triggers
/// the voice contract to create a proxy for this contract.
/// state is advanced from `instantiated` to `proxy_created` on the
/// polytone callback, where we query the note for remote address.
/// if address is found, we store it in PROXY_ADDRESS and advance the
/// state to `proxy_created`.
/// see polytone_handlers `process_execute_callback` match statement
/// handling the CREATE_PROXY_CALLBACK_ID for details.
fn try_create_proxy(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;

    let polytone_execute_msg_binary = get_polytone_execute_msg_binary(
        vec![],
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&CREATE_PROXY_CALLBACK_ID)?,
        }),
        ibc_config.osmo_ibc_timeout,
    )?;

    let note_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: note_address.to_string(),
        msg: polytone_execute_msg_binary,
        funds: vec![],
    });

    Ok(Response::default()
        .add_message(note_msg)
        .add_attribute("method", "try_create_proxy"))
}

fn try_deliver_funds(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;

    // check if both balances have a recent query
    match (
        lp_config.get_party_1_proxy_balance(),
        lp_config.get_party_2_proxy_balance(),
    ) {
        (Some(proxy_party_1_coin), Some(proxy_party_2_coin)) => {
            // if proxy holds both party contributions, we advance the state machine
            if lp_config.proxy_received_party_contributions(proxy_party_1_coin, proxy_party_2_coin)
            {
                // otherwise we advance the state machine and store an
                // expiration time for the funding period
                let funding_expiration = lp_config.funding_duration.after(&env.block);
                CONTRACT_STATE.save(
                    deps.storage,
                    &ContractState::ProxyFunded { funding_expiration },
                )?;
                Ok(Response::default()
                    .add_attribute("method", "try_tick")
                    .add_attribute("contract_state", "proxy_funded"))
            } else {
                // otherwise we attempt to deliver the funds
                try_fund_proxy(deps, env)
            }
        }
        // if either balance is unknown, we requery
        _ => {
            // reset the balances and submit the query
            lp_config.reset_latest_proxy_balances();
            LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;
            query_proxy_balances(deps, env)
        }
    }
}

fn try_fund_proxy(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;

    // we get our target denom balances which we should transfer to the proxy
    let coin_1_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.party_1_denom_info.local_denom.to_string(),
    )?;

    let coin_2_bal = deps.querier.query_balance(
        env.contract.address.to_string(),
        lp_config.party_2_denom_info.local_denom.to_string(),
    )?;

    // if either available balance is not sufficient,
    // we reset the latest proxy balance to `None`.
    // this will trigger a query on following tick.
    if lp_config.party_1_denom_info.osmosis_coin.amount > coin_1_bal.amount
        || lp_config.party_2_denom_info.osmosis_coin.amount > coin_2_bal.amount
    {
        // remove party denom entries from the balances map.
        // this will trigger a proxy balance query on the following tick.
        lp_config.reset_latest_proxy_balances();
        LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

        return Ok(Response::default()
            .add_attribute("method", "try_fund_proxy")
            .add_attribute("result", "insufficient_balances"));
    }

    let mut transfer_messages = vec![];
    let min_ibc_fee: MinIbcFeeResponse = deps.querier.query(&NeutronQuery::MinIbcFee {}.into())?;
    if coin_1_bal.amount > Uint128::zero() {
        transfer_messages.push(get_ibc_transfer_message(
            ibc_config.party_1_chain_info,
            env.clone(),
            coin_1_bal,
            proxy_address.to_string(),
            &min_ibc_fee.min_fee,
        )?);
    }
    if coin_2_bal.amount > Uint128::zero() {
        transfer_messages.push(get_ibc_transfer_message(
            ibc_config.party_2_chain_info,
            env,
            coin_2_bal,
            proxy_address,
            &min_ibc_fee.min_fee,
        )?);
    }

    Ok(Response::default()
        .add_messages(transfer_messages)
        .add_attribute("method", "try_fund_proxy"))
}

fn try_provide_liquidity(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let mut lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;

    // we generate a provide_liquidity message for the outpost
    // and wrap it in a note message
    let outpost_msg = lp_config.get_osmo_outpost_provide_liquidity_message()?;
    let note_outpost_liquidity_msg = get_note_execute_neutron_msg(
        vec![outpost_msg],
        ibc_config.osmo_ibc_timeout,
        note_address.clone(),
        Some(CallbackRequest {
            receiver: env.contract.address.to_string(),
            msg: to_json_binary(&PROVIDE_LIQUIDITY_CALLBACK_ID)?,
        }),
    )?;

    // following the liquidity provision message we perform a proxy balances query.
    // this gets executed after the lp attempt, so on callback we can know if
    // our lp attempt succeeded.
    let note_query_balances_msg = get_proxy_query_balances_message(
        env,
        proxy_address,
        note_address.to_string(),
        lp_config.clone(),
        ibc_config,
        PROXY_BALANCES_QUERY_CALLBACK_ID,
    )?;

    // reset the prices as they have expired
    lp_config.reset_latest_proxy_balances();
    LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &lp_config)?;

    Ok(Response::default()
        .add_message(note_outpost_liquidity_msg)
        .add_message(note_query_balances_msg)
        .add_attribute("method", "try_lp"))
}

fn query_proxy_balances(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let note_address = NOTE_ADDRESS.load(deps.storage)?;
    let ibc_config = IBC_CONFIG.load(deps.storage)?;
    let proxy_address = PROXY_ADDRESS.load(deps.storage)?;
    let lp_config = LIQUIDITY_PROVISIONING_CONFIG.load(deps.storage)?;

    let note_balance_query_msg = get_proxy_query_balances_message(
        env,
        proxy_address,
        note_address.to_string(),
        lp_config,
        ibc_config,
        PROXY_BALANCES_QUERY_CALLBACK_ID,
    )?;
    Ok(Response::default()
        .add_message(note_balance_query_msg)
        .add_attribute("method", "try_query_proxy_balances"))
}

fn get_ibc_transfer_message(
    party_chain_info: PartyChainInfo,
    env: Env,
    coin: Coin,
    proxy_address: String,
    ibc_fee: &IbcFee,
) -> StdResult<NeutronMsg> {
    // depending on whether pfm is configured,
    // we return a ibc transfer message
    match party_chain_info.outwards_pfm {
        // pfm necesary, we configure the memo
        Some(forward_metadata) => Ok(NeutronMsg::IbcTransfer {
            source_port: "transfer".to_string(),
            source_channel: party_chain_info.neutron_to_party_chain_channel,
            token: coin,
            sender: env.contract.address.to_string(),
            receiver: forward_metadata.receiver,
            timeout_height: RequestPacketTimeoutHeight {
                revision_number: None,
                revision_height: None,
            },
            timeout_timestamp: env
                .block
                .time
                .plus_seconds(party_chain_info.ibc_timeout.u64())
                .nanos(),
            memo: to_json_string(&PacketMetadata {
                forward: Some(ForwardMetadata {
                    receiver: proxy_address.to_string(),
                    port: forward_metadata.port,
                    channel: forward_metadata.channel,
                }),
            })?,
            fee: ibc_fee.clone(),
        }),
        // no pfm necessary, we do a regular transfer
        None => Ok(NeutronMsg::IbcTransfer {
            source_port: "transfer".to_string(),
            source_channel: party_chain_info.neutron_to_party_chain_channel,
            token: coin,
            sender: env.contract.address.to_string(),
            receiver: proxy_address.to_string(),
            timeout_height: RequestPacketTimeoutHeight {
                revision_number: None,
                revision_height: None,
            },
            timeout_timestamp: env
                .block
                .time
                .plus_seconds(party_chain_info.ibc_timeout.u64())
                .nanos(),
            memo: "".to_string(),
            fee: ibc_fee.clone(),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: QueryDeps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ClockAddress {} => Ok(to_json_binary(&CLOCK_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::ContractState {} => Ok(to_json_binary(&CONTRACT_STATE.may_load(deps.storage)?)?),
        QueryMsg::HolderAddress {} => Ok(to_json_binary(&HOLDER_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::DepositAddress {} => Ok(to_json_binary(&env.contract.address)?),
        QueryMsg::ProxyAddress {} => Ok(to_json_binary(&PROXY_ADDRESS.may_load(deps.storage)?)?),
        QueryMsg::IbcConfig {} => Ok(to_json_binary(&IBC_CONFIG.may_load(deps.storage)?)?),
        QueryMsg::LiquidityProvisionConfig {} => Ok(to_json_binary(
            &LIQUIDITY_PROVISIONING_CONFIG.may_load(deps.storage)?,
        )?),
        QueryMsg::Callbacks {} => {
            let mut vals = vec![];
            POLYTONE_CALLBACKS
                .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
                .for_each(|c| {
                    if let Ok((k, v)) = c {
                        vals.push(format!("{:?} : {:?}", k, v))
                    }
                });

            Ok(to_json_binary(&vals)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: ExecuteDeps, _env: Env, msg: MigrateMsg) -> NeutronResult<Response> {
    match msg {
        MigrateMsg::UpdateConfig {
            clock_addr,
            holder_address,
            note_address,
            ibc_config,
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

            if let Some(config) = *ibc_config {
                IBC_CONFIG.save(deps.storage, &config)?;
                response = response.add_attributes(config.to_response_attributes());
            }

            if let Some(address) = note_address {
                let note = deps.api.addr_validate(&address)?;
                NOTE_ADDRESS.save(deps.storage, &note)?;
                response = response.add_attribute("note_address", note);
            }

            if let Some(config) = *lp_config {
                LIQUIDITY_PROVISIONING_CONFIG.save(deps.storage, &config)?;
                response = response.add_attributes(config.to_response_attributes());
            }

            Ok(response)
        }
        MigrateMsg::UpdateCodeId { data: _ } => {
            let version: Version = match CONTRACT_VERSION.parse() {
                Ok(v) => v,
                Err(e) => return Err(NeutronError::Std(StdError::generic_err(e.to_string()))),
            };

            let storage_version: Version = match get_contract_version(deps.storage)?.version.parse()
            {
                Ok(v) => v,
                Err(e) => return Err(NeutronError::Std(StdError::generic_err(e.to_string()))),
            };
            if storage_version < version {
                set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            }
            Ok(Response::new())
        }
    }
}
