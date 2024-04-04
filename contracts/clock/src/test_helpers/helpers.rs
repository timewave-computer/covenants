use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    NeutronResult,
};

use crate::msg::{InstantiateMsg, QueryMsg};

type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;
type QueryDeps<'a> = Deps<'a, NeutronQuery>;

pub fn mock_neutron_clock_instantiate(
    _deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    Ok(Response::default())
}

pub fn mock_neutron_clock_execute(
    _deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    msg: crate::msg::ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        crate::msg::ExecuteMsg::Enqueue {} => Ok(Response::default()),
        crate::msg::ExecuteMsg::Dequeue {} => Ok(Response::default()),
        crate::msg::ExecuteMsg::Tick {} => Ok(Response::default()),
        crate::msg::ExecuteMsg::RecoverFunds { denoms: _ } => Ok(Response::default()),
    }
}

pub fn mock_neutron_clock_query(_deps: QueryDeps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsQueued { address: _ } => Ok(Binary::default()),
        QueryMsg::Queue {
            start_after: _,
            limit: _,
        } => Ok(Binary::default()),
        QueryMsg::TickMaxGas {} => Ok(Binary::default()),
        QueryMsg::Paused {} => Ok(Binary::default()),
        QueryMsg::Whitelist {} => Ok(Binary::default()),
    }
}
