use cosmwasm_std::Addr;

use super::suite::{SuiteBuilder, DEFAULT_CLOCK_ADDRESS, DEFAULT_RECEIVER_AMOUNT, NATIVE_ATOM_DENOM, ST_ATOM_DENOM};


#[test]
fn test_instantiate_happy() {
    let suite = SuiteBuilder::default()
        .build();

    // suite.assert_clock_address(Addr::unchecked(DEFAULT_CLOCK_ADDRESS));
    // suite.assert_native_atom_receiver(WeightedReceiver {
    //     amount: DEFAULT_RECEIVER_AMOUNT,
    //     address: NATIVE_ATOM_DENOM.to_string(),
    // });
    // suite.assert_stride_atom_receiver(WeightedReceiver {
    //     amount: DEFAULT_RECEIVER_AMOUNT,
    //     address: ST_ATOM_DENOM.to_string(),
    // });
}