use super::suite::SuiteBuilder;

#[test]
fn test_instantiate_happy() {
    let _suite = SuiteBuilder::default().build();

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
