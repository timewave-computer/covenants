#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    DepsMut, Env, MessageInfo, Reply, Response, Uint128,
};

use covenant_astroport_liquid_pooler::msg::{PresetAstroLiquidPoolerFields, AssetData, SingleSideLpLimits};
use covenant_clock::msg::PresetClockFields;
use covenant_remote_chain_splitter::msg::PresetRemoteChainSplitterFields;
use covenant_stride_liquid_staker::msg::PresetLsFields;
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::InstantiateMsg,
    state::{
        PRESET_CLOCK_FIELDS,
        PRESET_LP_FIELDS, PRESET_LS_FIELDS, PRESET_REMOTE_CHAIN_SPLITTER_FIELDS,
    },
};

const CONTRACT_NAME: &str = "crates.io:covenant-single-party-pol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let preset_ls_fields = PresetLsFields {
        ls_code: msg.contract_codes.liquid_staker_code,
        label: format!("{}-stride-liquid-staker", msg.label),
        ls_denom: msg.ls_asset_denom,
        stride_neutron_ibc_transfer_channel_id: msg.stride_neutron_ibc_transfer_channel_id,
        neutron_stride_ibc_connection_id: msg.neutron_stride_ibc_connection_id,
        autopilot_format: msg.autopilot_format,
    };

    let preset_remote_splitter_fields = PresetRemoteChainSplitterFields {
        remote_chain_connection_id: msg.remote_chain_connection_id,
        remote_chain_channel_id: msg.remote_chain_channel_id,
        denom: msg.native_asset_denom,
        amount: msg.amount,
        splits: vec![], // TODO
        ibc_fee: msg.preset_ibc_fee.to_ibc_fee(),
        ica_timeout: msg.timeouts.ica_timeout,
        ibc_transfer_timeout: msg.timeouts.ibc_transfer_timeout,
        code_id: msg.contract_codes.remote_chain_splitter_code,
        label: format!("{}-remote-chain-splitter", msg.label),
    };

    let preset_lp_fields = PresetAstroLiquidPoolerFields {
        slippage_tolerance: None,
        assets: AssetData {
            asset_a_denom: msg.neutron_native_asset_denom,
            asset_b_denom: msg.neutron_ls_asset_denom,
        },
        single_side_lp_limits: SingleSideLpLimits {
            asset_a_limit: Uint128::new(10000),
            asset_b_limit: Uint128::new(100000),
        },
        label: format!("{}-astro-liquid-pooler", msg.label),
        code_id: msg.contract_codes.liquid_pooler_code,
        expected_pool_ratio: msg.expected_pool_ratio,
        acceptable_pool_ratio_delta: msg.acceptable_pool_ratio_delta,
        pair_type: msg.pool_pair_type,
    };

    let preset_clock_fields = PresetClockFields {
        tick_max_gas: msg.clock_tick_max_gas,
        whitelist: vec![],
        code_id: msg.contract_codes.clock_code,
        label: format!("{}-clock", msg.label),
    };


    // store all the preset fields for each contract instantiation
    PRESET_CLOCK_FIELDS.save(deps.storage, &preset_clock_fields)?;
    PRESET_LP_FIELDS.save(deps.storage, &preset_lp_fields)?;
    PRESET_LS_FIELDS.save(deps.storage, &preset_ls_fields)?;
    PRESET_REMOTE_CHAIN_SPLITTER_FIELDS.save(deps.storage, &preset_remote_splitter_fields)?;

    Ok(Response::default()
        .add_attribute("method", "covenant-instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        _ => Err(ContractError::UnknownReplyId {}),
    }
}
