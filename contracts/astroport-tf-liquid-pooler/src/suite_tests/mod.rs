pub mod suite;
pub mod suite_builder;
pub mod tests;

use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo};
use cw_multi_test::{Contract, ContractWrapper};

pub fn liquid_pooler_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply)
    .with_migrate(crate::contract::migrate);

    Box::new(contract)
}

pub fn holder_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        valence_single_party_pol_holder::contract::execute,
        valence_single_party_pol_holder::contract::instantiate,
        valence_single_party_pol_holder::contract::query,
    )
    .with_migrate(valence_single_party_pol_holder::contract::migrate);

    Box::new(contract)
}

pub fn astro_whitelist_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw1_whitelist::contract::execute,
        cw1_whitelist::contract::instantiate,
        cw1_whitelist::contract::query,
    );

    Box::new(contract)
}

pub fn astro_token_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        astroport_xastro_token::contract::execute,
        astroport_xastro_token::contract::instantiate,
        astroport_xastro_token::contract::query);

    Box::new(contract)
}

pub fn astro_factory_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        astroport_factory::contract::execute,
        astroport_factory::contract::instantiate,
        astroport_factory::contract::query,
    ).with_reply(astroport_factory::contract::reply);

    Box::new(contract)
}

pub fn astro_pair_stable_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        astroport_pair_stable::contract::execute,
        astroport_pair_stable::contract::instantiate,
        astroport_pair_stable::contract::query,
    )
    .with_reply(astroport_pair_stable::contract::reply);

    Box::new(contract)
}

pub fn astro_pair_custom_concentrated_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        astroport_pair_concentrated::contract::execute,
        astroport_pair_concentrated::contract::instantiate,
        astroport_pair_stable::contract::query,
    );

    Box::new(contract)
}

pub fn astro_pair_xyk_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        astroport_pair::contract::execute,
        astroport_pair::contract::instantiate,
        astroport_pair::contract::query,
    )
    .with_reply(astroport_pair::contract::reply);

    Box::new(contract)
}

pub fn astro_coin_registry_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        astroport_native_coin_registry::contract::execute,
        astroport_native_coin_registry::contract::instantiate,
        astroport_native_coin_registry::contract::query,
    );

    Box::new(contract)
}
