use std::fmt::Display;

use cosmwasm_std::{
    CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdError, SubMsg,
};
use cw_multi_test::{Contract, ContractWrapper};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

/// Turn a neutron response into an empty response
/// This is fine because the contract return an empty response, but our testing enviroment expects a neutron response
/// the contract that uses this function will never emit a neutron response anyways
pub fn execute_into_neutron<E: Display>(
    into: Result<Response, E>,
) -> Result<Response<NeutronMsg>, E> {
    into.map(|r| {
        let mut res: Response<NeutronMsg> = Response::<NeutronMsg>::default();
        res.data = r.data;
        res.messages = r
            .messages
            .into_iter()
            .map(|m| {
                let msg: CosmosMsg<NeutronMsg> = match m.msg {
                    CosmosMsg::Bank(b) => CosmosMsg::<NeutronMsg>::Bank(b),
                    CosmosMsg::Staking(s) => CosmosMsg::<NeutronMsg>::Staking(s),
                    CosmosMsg::Distribution(d) => CosmosMsg::<NeutronMsg>::Distribution(d),
                    CosmosMsg::Stargate { type_url, value } => {
                        CosmosMsg::<NeutronMsg>::Stargate { type_url, value }
                    }
                    CosmosMsg::Ibc(ibc) => CosmosMsg::<NeutronMsg>::Ibc(ibc),
                    CosmosMsg::Wasm(w) => CosmosMsg::<NeutronMsg>::Wasm(w),
                    CosmosMsg::Gov(g) => CosmosMsg::<NeutronMsg>::Gov(g),
                    _ => CosmosMsg::<NeutronMsg>::Custom(NeutronMsg::RemoveSchedule {
                        name: "".to_string(),
                    }),
                };

                SubMsg::<NeutronMsg> {
                    id: m.id,
                    msg,
                    gas_limit: m.gas_limit,
                    reply_on: m.reply_on,
                }
            })
            .collect();
        res.attributes = r.attributes;
        res
    })
    // .map_err(|e| NeutronError::Std(StdError::GenericErr { msg: e.to_string() }))
}

/// Turn neutron DepsMut into empty DepsMut
pub fn get_empty_depsmut(deps: DepsMut<NeutronQuery>) -> DepsMut<'_, Empty> {
    DepsMut {
        storage: deps.storage,
        api: deps.api,
        querier: deps.querier.into_empty(),
    }
}

/// Turn neutron Deps into empty Deps
pub(crate) fn get_empty_deps(deps: Deps<NeutronQuery>) -> Deps<'_, Empty> {
    Deps {
        storage: deps.storage,
        api: deps.api,
        querier: deps.querier.into_empty(),
    }
}

pub fn clock_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_clock::msg::ExecuteMsg| {
        execute_into_neutron(valence_clock::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_clock::msg::InstantiateMsg| {
        execute_into_neutron(valence_clock::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: valence_clock::msg::QueryMsg| {
        valence_clock::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>, env: Env, msg: valence_clock::msg::MigrateMsg| {
        execute_into_neutron(valence_clock::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let reply = |deps: DepsMut<NeutronQuery>, env: Env, reply: Reply| {
        execute_into_neutron(valence_clock::contract::reply(
            get_empty_depsmut(deps),
            env,
            reply,
        ))
    };

    let contract = ContractWrapper::new(exec, init, query)
        .with_migrate(migrate)
        .with_reply(reply);
    Box::new(contract)
}

pub fn ibc_forwarder_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        valence_ibc_forwarder::contract::execute,
        valence_ibc_forwarder::contract::instantiate,
        valence_ibc_forwarder::contract::query,
    )
    .with_reply(valence_ibc_forwarder::contract::reply)
    .with_sudo(valence_ibc_forwarder::contract::sudo)
    .with_migrate(valence_ibc_forwarder::contract::migrate);
    Box::new(contract)
}

pub fn interchain_router_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        valence_interchain_router::contract::execute,
        valence_interchain_router::contract::instantiate,
        valence_interchain_router::contract::query,
    )
    .with_migrate(valence_interchain_router::contract::migrate);
    Box::new(contract)
}

pub fn remote_splitter_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        valence_remote_chain_splitter::contract::execute,
        valence_remote_chain_splitter::contract::instantiate,
        valence_remote_chain_splitter::contract::query,
    )
    .with_reply(valence_remote_chain_splitter::contract::reply)
    .with_sudo(valence_remote_chain_splitter::contract::sudo)
    .with_migrate(valence_remote_chain_splitter::contract::migrate);
    Box::new(contract)
}

pub fn osmo_lp_outpost_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_outpost_osmo_liquid_pooler::msg::ExecuteMsg| {
        execute_into_neutron(valence_outpost_osmo_liquid_pooler::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_outpost_osmo_liquid_pooler::msg::InstantiateMsg| {
        execute_into_neutron(valence_outpost_osmo_liquid_pooler::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let reply = |deps: DepsMut<NeutronQuery>, env: Env, reply: Reply| {
        execute_into_neutron(valence_outpost_osmo_liquid_pooler::contract::reply(
            get_empty_depsmut(deps),
            env,
            reply,
        ))
    };

    let query = |deps: Deps<NeutronQuery>,
                 env: Env,
                 msg: valence_outpost_osmo_liquid_pooler::msg::QueryMsg| {
        valence_outpost_osmo_liquid_pooler::contract::query(get_empty_deps(deps), env, msg)
    };

    Box::new(ContractWrapper::new(exec, init, query).with_reply(reply))
}

pub fn native_router_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_native_router::msg::ExecuteMsg| {
        execute_into_neutron(valence_native_router::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_native_router::msg::InstantiateMsg| {
        execute_into_neutron(valence_native_router::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: valence_native_router::msg::QueryMsg| {
        valence_native_router::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate =
        |deps: DepsMut<NeutronQuery>, env: Env, msg: valence_native_router::msg::MigrateMsg| {
            execute_into_neutron(valence_native_router::contract::migrate(
                get_empty_depsmut(deps),
                env,
                msg,
            ))
        };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn native_splitter_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_native_splitter::msg::ExecuteMsg| {
        execute_into_neutron(valence_native_splitter::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_native_splitter::msg::InstantiateMsg| {
        execute_into_neutron(valence_native_splitter::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query =
        |deps: Deps<NeutronQuery>, env: Env, msg: valence_native_splitter::msg::QueryMsg| {
            valence_native_splitter::contract::query(get_empty_deps(deps), env, msg)
        };

    let migrate =
        |deps: DepsMut<NeutronQuery>, env: Env, msg: valence_native_splitter::msg::MigrateMsg| {
            execute_into_neutron(valence_native_splitter::contract::migrate(
                get_empty_depsmut(deps),
                env,
                msg,
            ))
        };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn single_party_covenant_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |_deps: DepsMut<NeutronQuery>,
                _env: Env,
                _info: MessageInfo,
                _msg: Empty|
     -> Result<Response<NeutronMsg>, StdError> {
        Err(StdError::generic_err("Execute msg is not implemented"))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_covenant_single_party_pol::msg::InstantiateMsg| {
        execute_into_neutron(valence_covenant_single_party_pol::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>,
                 env: Env,
                 msg: valence_covenant_single_party_pol::msg::QueryMsg| {
        valence_covenant_single_party_pol::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>,
                   env: Env,
                   msg: valence_covenant_single_party_pol::msg::MigrateMsg| {
        execute_into_neutron(valence_covenant_single_party_pol::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn single_party_holder_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_single_party_pol_holder::msg::ExecuteMsg| {
        execute_into_neutron(valence_single_party_pol_holder::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_single_party_pol_holder::msg::InstantiateMsg| {
        execute_into_neutron(valence_single_party_pol_holder::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>,
                 env: Env,
                 msg: valence_single_party_pol_holder::msg::QueryMsg| {
        valence_single_party_pol_holder::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>,
                   env: Env,
                   msg: valence_single_party_pol_holder::msg::MigrateMsg| {
        execute_into_neutron(valence_single_party_pol_holder::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn stride_lser_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        valence_stride_liquid_staker::contract::execute,
        valence_stride_liquid_staker::contract::instantiate,
        valence_stride_liquid_staker::contract::query,
    )
    .with_reply(valence_stride_liquid_staker::contract::reply)
    .with_sudo(valence_stride_liquid_staker::contract::sudo)
    .with_migrate(valence_stride_liquid_staker::contract::migrate);
    Box::new(contract)
}

pub fn swap_covenant_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |_deps: DepsMut<NeutronQuery>,
                _env: Env,
                _info: MessageInfo,
                _msg: Empty|
     -> Result<Response<NeutronMsg>, StdError> {
        Err(StdError::generic_err("Execute msg is not implemented"))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_covenant_swap::msg::InstantiateMsg| {
        execute_into_neutron(valence_covenant_swap::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: valence_covenant_swap::msg::QueryMsg| {
        valence_covenant_swap::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate =
        |deps: DepsMut<NeutronQuery>, env: Env, msg: valence_covenant_swap::msg::MigrateMsg| {
            execute_into_neutron(valence_covenant_swap::contract::migrate(
                get_empty_depsmut(deps),
                env,
                msg,
            ))
        };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn swap_holder_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_swap_holder::msg::ExecuteMsg| {
        execute_into_neutron(valence_swap_holder::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_swap_holder::msg::InstantiateMsg| {
        execute_into_neutron(valence_swap_holder::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: valence_swap_holder::msg::QueryMsg| {
        valence_swap_holder::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate =
        |deps: DepsMut<NeutronQuery>, env: Env, msg: valence_swap_holder::msg::MigrateMsg| {
            execute_into_neutron(valence_swap_holder::contract::migrate(
                get_empty_depsmut(deps),
                env,
                msg,
            ))
        };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn two_party_covenant_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |_deps: DepsMut<NeutronQuery>,
                _env: Env,
                _info: MessageInfo,
                _msg: Empty|
     -> Result<Response<NeutronMsg>, StdError> {
        Err(StdError::generic_err("Execute msg is not implemented"))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_covenant_two_party_pol::msg::InstantiateMsg| {
        execute_into_neutron(valence_covenant_two_party_pol::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query =
        |deps: Deps<NeutronQuery>, env: Env, msg: valence_covenant_two_party_pol::msg::QueryMsg| {
            valence_covenant_two_party_pol::contract::query(get_empty_deps(deps), env, msg)
        };

    let migrate = |deps: DepsMut<NeutronQuery>,
                   env: Env,
                   msg: valence_covenant_two_party_pol::msg::MigrateMsg| {
        execute_into_neutron(valence_covenant_two_party_pol::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn two_party_holder_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_two_party_pol_holder::msg::ExecuteMsg| {
        execute_into_neutron(valence_two_party_pol_holder::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_two_party_pol_holder::msg::InstantiateMsg| {
        execute_into_neutron(valence_two_party_pol_holder::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query =
        |deps: Deps<NeutronQuery>, env: Env, msg: valence_two_party_pol_holder::msg::QueryMsg| {
            valence_two_party_pol_holder::contract::query(get_empty_deps(deps), env, msg)
        };

    let migrate = |deps: DepsMut<NeutronQuery>,
                   env: Env,
                   msg: valence_two_party_pol_holder::msg::MigrateMsg| {
        execute_into_neutron(valence_two_party_pol_holder::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let contract = ContractWrapper::new(exec, init, query).with_migrate(migrate);
    Box::new(contract)
}

pub fn astroport_pooler_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_astroport_liquid_pooler::msg::ExecuteMsg| {
        execute_into_neutron(valence_astroport_liquid_pooler::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: valence_astroport_liquid_pooler::msg::InstantiateMsg| {
        execute_into_neutron(valence_astroport_liquid_pooler::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>,
                 env: Env,
                 msg: valence_astroport_liquid_pooler::msg::QueryMsg| {
        valence_astroport_liquid_pooler::contract::query(get_empty_deps(deps), env, msg)
    };

    let reply = |deps: DepsMut<NeutronQuery>, env: Env, reply: Reply| {
        execute_into_neutron(valence_astroport_liquid_pooler::contract::reply(
            get_empty_depsmut(deps),
            env,
            reply,
        ))
    };

    let migrate = |deps: DepsMut<NeutronQuery>,
                   env: Env,
                   msg: valence_astroport_liquid_pooler::msg::MigrateMsg| {
        execute_into_neutron(valence_astroport_liquid_pooler::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let contract = ContractWrapper::new(exec, init, query)
        .with_reply(reply)
        .with_migrate(migrate);
    Box::new(contract)
}
