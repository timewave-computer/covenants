use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, ibc::applications::transfer::v1::MsgTransfer};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{Env, MessageInfo, Response, Deps, DepsMut, StdError, Binary, Addr};
use covenant_clock::helpers::verify_clock;
use cw2::set_contract_version;
use neutron_sdk::{NeutronResult, bindings::{msg::NeutronMsg, query::NeutronQuery, types::ProtobufAny}, interchain_txs::helpers::get_port_id, NeutronError,};
use prost::Message;

use crate::{msg::{InstantiateMsg, ExecuteMsg, ContractState, RemoteChainInfo}, state::{CONTRACT_STATE, CLOCK_ADDRESS, INTERCHAIN_ACCOUNTS, IBC_FEE, ICA_TIMEOUT, IBC_TRANSFER_TIMEOUT, REMOTE_CHAIN_INFO, NEXT_CONTRACT}, error::ContractError};


const CONTRACT_NAME: &str = "crates.io:covenant-ibc-forwarder";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INTERCHAIN_ACCOUNT_ID: &str = "ica";

type QueryDeps<'a> = Deps<'a, NeutronQuery>;
type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONTRACT_STATE.save(deps.storage, &ContractState::Instantiated)?;

    // ibc fees and timeouts
    IBC_FEE.save(deps.storage, &msg.ibc_fee)?;
    ICA_TIMEOUT.save(deps.storage, &msg.ica_timeout)?;
    IBC_TRANSFER_TIMEOUT.save(deps.storage, &msg.ibc_transfer_timeout)?;
    let next_contract = deps.api.addr_validate(&msg.next_contract)?;
    NEXT_CONTRACT.save(deps.storage, &next_contract)?;

    REMOTE_CHAIN_INFO.save(deps.storage, &RemoteChainInfo {
        connection_id: msg.remote_chain_connection_id,
        channel_id: msg.remote_chain_channel_id,
        denom: msg.denom,
        amount: msg.amount,
    })?;
    
    Ok(Response::default()
        .add_attribute("method", "ibc_forwarder_instantiate")

    )
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
    }
}


/// attempts to advance the state machine. validates the caller to be the clock.
fn try_tick(deps: ExecuteDeps, env: Env, info: MessageInfo) -> NeutronResult<Response<NeutronMsg>> {
    // Verify caller is the clock
    verify_clock(&info.sender, &CLOCK_ADDRESS.load(deps.storage)?)?;

    let current_state = CONTRACT_STATE.load(deps.storage)?;
    match current_state {
        ContractState::Instantiated => try_register_ica(deps, env),
        ContractState::ICACreated => try_forward_funds(env, deps),
        ContractState::Complete => todo!(),
    }
}

/// tries to register an ICA on the remote chain
fn try_register_ica(deps: ExecuteDeps, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    
    let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;
    
    let register_msg = NeutronMsg::register_interchain_account(
        remote_chain_info.connection_id,
        INTERCHAIN_ACCOUNT_ID.to_string()
    );

    let key = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    
    // we are saving empty data here because we handle response of registering ICA in sudo_open_ack method
    INTERCHAIN_ACCOUNTS.save(deps.storage, key, &None)?;

    Ok(Response::new()
        .add_attribute("method", "try_register_ica")
        .add_message(register_msg)
    )
}

fn try_forward_funds(env: Env, mut deps: ExecuteDeps) -> NeutronResult<Response<NeutronMsg>> {


    // first we verify whether the next contract is ready for receiving the funds
    let next_contract = NEXT_CONTRACT.load(deps.storage)?;
    let deposit_address_query: Option<Addr> = deps.querier.query_wasm_smart(
        next_contract,
        &crate::msg::QueryMsg::DepositAddress {},
    )?;

    // if query returns None, then we error and wait
    let deposit_address = if let Some(addr) = deposit_address_query {
        addr
    } else {
        return Err(NeutronError::Std(
            StdError::not_found("Next contract is not ready for receiving the funds yet")
        ))
    };

    let port_id = get_port_id(env.contract.address.as_str(), INTERCHAIN_ACCOUNT_ID);
    let interchain_account = INTERCHAIN_ACCOUNTS.load(
        deps.storage,
        port_id.clone()
    )?;

    match interchain_account {
        Some((address, controller_conn_id)) => {
            let ibc_transfer_timeout = IBC_TRANSFER_TIMEOUT.load(deps.storage)?;
            let ica_timeout = ICA_TIMEOUT.load(deps.storage)?;
            let fee = IBC_FEE.load(deps.storage)?;
            let remote_chain_info = REMOTE_CHAIN_INFO.load(deps.storage)?;

            let coin = remote_chain_info.proto_coin();

            let transfer_msg = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: remote_chain_info.channel_id,
                token: Some(coin),
                sender: address,
                receiver: deposit_address.to_string(),
                timeout_height: None,
                timeout_timestamp: env.block.time
                    .plus_seconds(ica_timeout.u64())
                    .plus_seconds(ibc_transfer_timeout.u64())
                    .nanos(),
            };

            let protobuf_msg = to_proto_msg_transfer(transfer_msg)?;

            // tx to our ICA that wraps the transfer message defined above
            let submit_msg = NeutronMsg::submit_tx(
                controller_conn_id,
                INTERCHAIN_ACCOUNT_ID.to_string(),
                vec![protobuf_msg],
                "".to_string(),
                ica_timeout.u64(),
                fee,
            );

            // sudo callback msg
            
            Ok(Response::default())
        },
        None => Ok(Response::default()
            .add_attribute("method", "try_forward_funds")
            .add_attribute("error", "no_ica_found")
        ),
    }
}

/// helper that serializes a MsgTransfer to protobuf
fn to_proto_msg_transfer(msg: impl Message) -> NeutronResult<ProtobufAny> {
    // Serialize the Transfer message
    let mut buf = Vec::new();
    buf.reserve(msg.encoded_len());
    if let Err(e) = msg.encode(&mut buf) {
        return Err(StdError::generic_err(format!("Encode error: {e}")).into());
    }

    Ok(ProtobufAny {
        type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
        value: Binary::from(buf),
    })
}