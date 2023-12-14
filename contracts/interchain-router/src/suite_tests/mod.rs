use covenant_clock::test_helpers::helpers::{
    mock_neutron_clock_execute, mock_neutron_clock_instantiate, mock_neutron_clock_query,
};
use cw_multi_test::{Contract, ContractWrapper};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

pub mod suite;
mod tests;

pub fn mock_clock_neutron_deps_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        mock_neutron_clock_execute,
        mock_neutron_clock_instantiate,
        mock_neutron_clock_query,
    );

    Box::new(contract)
}
