use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

mod suite;
mod tests;

// Advantage to using a macro for this is that the error trace links
// to the exact line that the error occured, instead of inside of a
// function where the assertion would otherwise happen.
macro_rules! is_error {
    ($x:expr, $e:expr) => {
        assert!(format!("{:#}", $x.unwrap_err()).contains($e))
    };
}
pub(crate) use is_error;

pub fn clock_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply)
    .with_migrate(crate::contract::migrate);
    Box::new(contract)
}

pub fn clock_tester_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        covenant_clock_tester::contract::execute,
        covenant_clock_tester::contract::instantiate,
        covenant_clock_tester::contract::query,
    );
    Box::new(contract)
}
