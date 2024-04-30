use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, Reply};
use cw_multi_test::{Contract, ContractWrapper};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

use super::contracts::{execute_into_neutron, get_empty_deps, get_empty_depsmut};

pub fn astro_token_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec =
        |deps: DepsMut<NeutronQuery>, env: Env, info: MessageInfo, msg: cw20::Cw20ExecuteMsg| {
            execute_into_neutron(astroport_token::contract::execute(
                get_empty_depsmut(deps),
                env,
                info,
                msg,
            ))
        };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::token::InstantiateMsg| {
        execute_into_neutron(astroport_token::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: cw20_base::msg::QueryMsg| {
        astroport_token::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>, env: Env, msg: astroport::token::MigrateMsg| {
        execute_into_neutron(astroport_token::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    Box::new(ContractWrapper::new(exec, init, query).with_migrate(migrate))
}

pub fn astro_whitelist_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: cw1_whitelist::msg::ExecuteMsg| {
        execute_into_neutron(astroport_whitelist::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: cw1_whitelist::msg::InstantiateMsg| {
        execute_into_neutron(astroport_whitelist::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: cw1_whitelist::msg::QueryMsg| {
        astroport_whitelist::contract::query(get_empty_deps(deps), env, msg)
    };

    Box::new(ContractWrapper::new(exec, init, query))
}

pub fn astro_factory_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::factory::ExecuteMsg| {
        execute_into_neutron(astroport_factory::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::factory::InstantiateMsg| {
        execute_into_neutron(astroport_factory::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: astroport::factory::QueryMsg| {
        astroport_factory::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>, env: Env, msg: astroport::factory::MigrateMsg| {
        execute_into_neutron(astroport_factory::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let reply = |deps: DepsMut<NeutronQuery>, env: Env, reply: Reply| {
        execute_into_neutron(astroport_factory::contract::reply(
            get_empty_depsmut(deps),
            env,
            reply,
        ))
    };

    Box::new(
        ContractWrapper::new(exec, init, query)
            .with_migrate(migrate)
            .with_reply(reply),
    )
}

pub fn astro_pair_stable_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::pair::ExecuteMsg| {
        execute_into_neutron(astroport_pair_stable::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::pair::InstantiateMsg| {
        execute_into_neutron(astroport_pair_stable::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: astroport::pair::QueryMsg| {
        astroport_pair_stable::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>, env: Env, msg: astroport::pair::MigrateMsg| {
        execute_into_neutron(astroport_pair_stable::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let reply = |deps: DepsMut<NeutronQuery>, env: Env, reply: Reply| {
        execute_into_neutron(astroport_pair_stable::contract::reply(
            get_empty_depsmut(deps),
            env,
            reply,
        ))
    };

    Box::new(
        ContractWrapper::new(exec, init, query)
            .with_migrate(migrate)
            .with_reply(reply),
    )
}

pub fn astro_pair_custom_concentrated_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::pair::ExecuteMsg| {
        execute_into_neutron(astroport_pair_concentrated::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::pair::InstantiateMsg| {
        execute_into_neutron(astroport_pair_concentrated::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query =
        |deps: Deps<NeutronQuery>, env: Env, msg: astroport::pair_concentrated::QueryMsg| {
            astroport_pair_concentrated::queries::query(get_empty_deps(deps), env, msg)
        };

    let migrate =
        |deps: DepsMut<NeutronQuery>, env: Env, msg: astroport::pair_concentrated::MigrateMsg| {
            execute_into_neutron(astroport_pair_concentrated::contract::migrate(
                get_empty_depsmut(deps),
                env,
                msg,
            ))
        };

    let reply = |deps: DepsMut<NeutronQuery>, env: Env, reply: Reply| {
        execute_into_neutron(astroport_pair_concentrated::contract::reply(
            get_empty_depsmut(deps),
            env,
            reply,
        ))
    };

    Box::new(
        ContractWrapper::new(exec, init, query)
            .with_migrate(migrate)
            .with_reply(reply),
    )
}

pub fn astro_pair_xyk_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::pair::ExecuteMsg| {
        execute_into_neutron(astroport_pair::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::pair::InstantiateMsg| {
        execute_into_neutron(astroport_pair::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: astroport::pair::QueryMsg| {
        astroport_pair::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>, env: Env, msg: astroport::pair::MigrateMsg| {
        execute_into_neutron(astroport_pair::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    let reply = |deps: DepsMut<NeutronQuery>, env: Env, reply: Reply| {
        execute_into_neutron(astroport_pair::contract::reply(
            get_empty_depsmut(deps),
            env,
            reply,
        ))
    };

    Box::new(
        ContractWrapper::new(exec, init, query)
            .with_migrate(migrate)
            .with_reply(reply),
    )
}

pub fn astro_coin_registry_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let exec = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::native_coin_registry::ExecuteMsg| {
        execute_into_neutron(astroport_native_coin_registry::contract::execute(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let init = |deps: DepsMut<NeutronQuery>,
                env: Env,
                info: MessageInfo,
                msg: astroport::native_coin_registry::InstantiateMsg| {
        execute_into_neutron(astroport_native_coin_registry::contract::instantiate(
            get_empty_depsmut(deps),
            env,
            info,
            msg,
        ))
    };

    let query = |deps: Deps<NeutronQuery>, env: Env, msg: cw20_base::msg::QueryMsg| {
        astroport_token::contract::query(get_empty_deps(deps), env, msg)
    };

    let migrate = |deps: DepsMut<NeutronQuery>,
                   env: Env,
                   msg: astroport::native_coin_registry::MigrateMsg| {
        execute_into_neutron(astroport_native_coin_registry::contract::migrate(
            get_empty_depsmut(deps),
            env,
            msg,
        ))
    };

    Box::new(ContractWrapper::new(exec, init, query).with_migrate(migrate))
}
