pub mod suite;
pub mod suite_builder;
pub mod tests;

use cosmwasm_std::Empty;
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
