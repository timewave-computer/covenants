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
