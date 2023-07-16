use cosmwasm_std::{
    instantiate2_address, to_binary, Addr, CanonicalAddr, CodeInfoResponse, CosmosMsg, Deps,
    DepsMut, Env, WasmMsg,
};

use crate::{
    error::ContractError,
    msg::InstantiateMsg,
    state::{
        COVENANT_CLOCK_ADDR, COVENANT_DEPOSITOR_ADDR, COVENANT_HOLDER_ADDR, COVENANT_LP_ADDR,
        COVENANT_LS_ADDR, IBC_FEE, IBC_TIMEOUT,
    },
};

const CLOCK_SALT: &[u8] = b"clock";
const HOLDER_SALT: &[u8] = b"holder";
const LP_SALT: &[u8] = b"liquidity_pooler";
const LS_SALT: &[u8] = b"liquid_staking";
const DEPOSITOR_SALT: &[u8] = b"depositor";

struct CovenantAddresses {
    clock_addr: Addr,
    holder_addr: Addr,
    lp_addr: Addr,
    ls_addr: Addr,
}

fn get_contract_addresses(
    deps: DepsMut,
    env: &Env,
    msg: &InstantiateMsg,
) -> Result<CovenantAddresses, ContractError> {
    let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;

    let clock_addr = get_address(
        deps.as_ref(),
        msg.preset_clock_fields.clock_code,
        &creator,
        CLOCK_SALT,
    )?;
    let holder_addr: Addr = get_address(
        deps.as_ref(),
        msg.preset_holder_fields.holder_code,
        &creator,
        HOLDER_SALT,
    )?;
    let lp_addr: Addr = get_address(
        deps.as_ref(),
        msg.preset_lp_fields.lp_code,
        &creator,
        LP_SALT,
    )?;
    let ls_addr: Addr = get_address(
        deps.as_ref(),
        msg.preset_ls_fields.ls_code,
        &creator,
        LS_SALT,
    )?;
    let depositor_addr: Addr = get_address(
        deps.as_ref(),
        msg.preset_depositor_fields.depositor_code,
        &creator,
        DEPOSITOR_SALT,
    )?;

    // Save addresses for queries
    COVENANT_CLOCK_ADDR.save(deps.storage, &clock_addr)?;
    COVENANT_HOLDER_ADDR.save(deps.storage, &holder_addr)?;
    COVENANT_LP_ADDR.save(deps.storage, &lp_addr)?;
    COVENANT_LS_ADDR.save(deps.storage, &ls_addr)?;
    COVENANT_DEPOSITOR_ADDR.save(deps.storage, &depositor_addr)?;

    Ok(CovenantAddresses {
        clock_addr,
        holder_addr,
        lp_addr,
        ls_addr,
    })
}

fn get_address(
    deps: Deps,
    code_id: u64,
    creator: &CanonicalAddr,
    sale: &[u8],
) -> Result<Addr, ContractError> {
    let CodeInfoResponse {
        checksum: clock_checksum,
        ..
    } = deps.querier.query_wasm_code_info(code_id)?;

    Ok(deps
        .api
        .addr_humanize(&instantiate2_address(&clock_checksum, &creator, sale)?)?)
}

pub fn get_instantiate_messages(
    deps: DepsMut,
    env: Env,
    msg: InstantiateMsg,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let ibc_fee = IBC_FEE.load(deps.storage)?;
    let ibc_timeout = IBC_TIMEOUT.load(deps.storage)?;

    let addresses = get_contract_addresses(deps, &env, &msg)?;
    let admin = Some(env.contract.address.to_string());

    let clock_msg = WasmMsg::Instantiate2 {
        admin: admin.clone(),
        code_id: msg.preset_clock_fields.clock_code,
        label: msg.preset_clock_fields.label.clone(),
        msg: to_binary(&msg.preset_clock_fields.to_instantiate_msg())?,
        funds: vec![],
        salt: to_binary(&CLOCK_SALT)?,
    }
    .into();

    let holder_msg = WasmMsg::Instantiate2 {
        admin: admin.clone(),
        code_id: msg.preset_holder_fields.holder_code,
        label: msg.preset_holder_fields.label.clone(),
        msg: to_binary(
            &msg.preset_holder_fields
                .to_instantiate_msg(msg.pool_address.clone()),
        )?,
        funds: vec![],
        salt: to_binary(&HOLDER_SALT)?,
    }
    .into();

    let lp_msg = WasmMsg::Instantiate2 {
        admin: admin.clone(),
        code_id: msg.preset_lp_fields.lp_code,
        label: msg.preset_lp_fields.label.clone(),
        msg: to_binary(&msg.preset_lp_fields.to_instantiate_msg(
            addresses.clock_addr.to_string(),
            addresses.holder_addr.to_string(),
            msg.pool_address,
        ))?,
        funds: vec![],
        salt: to_binary(&LP_SALT)?,
    }
    .into();

    let ls_msg = WasmMsg::Instantiate2 {
        admin: admin.clone(),
        code_id: msg.preset_ls_fields.ls_code,
        label: msg.preset_ls_fields.label.clone(),
        msg: to_binary(&msg.preset_ls_fields.to_instantiate_msg(
            addresses.clock_addr.to_string(),
            addresses.lp_addr.to_string(),
            ibc_timeout,
            ibc_fee.clone(),
        ))?,
        funds: vec![],
        salt: to_binary(&LS_SALT)?,
    }
    .into();

    let depositor_msg = WasmMsg::Instantiate2 {
        admin: admin.clone(),
        code_id: msg.preset_depositor_fields.depositor_code,
        label: msg.preset_depositor_fields.label.clone(),
        msg: to_binary(&msg.preset_depositor_fields.to_instantiate_msg(
            "to be queried".to_string(), //TODO:
            addresses.clock_addr.to_string(),
            addresses.ls_addr.to_string(),
            addresses.lp_addr.to_string(),
            ibc_timeout,
            ibc_fee,
        ))?,
        funds: vec![],
        salt: to_binary(&LS_SALT)?,
    }
    .into();

    Ok(vec![clock_msg, holder_msg, lp_msg, ls_msg, depositor_msg])
}
