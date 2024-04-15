pub mod suite;
pub mod tests;

use cw_multi_test::{Contract, ContractWrapper};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};
use valence_clock::test_helpers::helpers::{
    mock_neutron_clock_execute, mock_neutron_clock_instantiate, mock_neutron_clock_query,
};

pub fn mock_clock_neutron_deps_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    let contract = ContractWrapper::new(
        mock_neutron_clock_execute,
        mock_neutron_clock_instantiate,
        mock_neutron_clock_query,
    );

    Box::new(contract)
}
