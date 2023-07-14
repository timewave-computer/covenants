use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

mod suite;
mod tests;

// Advantage to using a macro for this is that the error trace links
// to the exact line that the error occured, instead of inside of a
// function where the assertion would otherwise happen.
macro_rules! is_error {
    ($x:expr, $e:expr) => {
        assert!(format!("{:#}", $x).contains($e))
    };
}
pub(crate) use is_error;

pub fn holder_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}
