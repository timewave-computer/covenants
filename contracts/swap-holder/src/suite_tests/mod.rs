use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};
use neutron_sdk::bindings::{
    msg::{IbcFee, NeutronMsg},
    query::NeutronQuery,
};


mod suite;
mod tests;

pub fn swap_holder_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}
