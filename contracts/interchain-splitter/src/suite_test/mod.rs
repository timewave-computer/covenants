use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

mod suite;
mod tests;


pub fn splitter_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate);
    Box::new(contract)
}
