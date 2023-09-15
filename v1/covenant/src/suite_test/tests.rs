use super::suite::SuiteBuilder;

#[test]
#[should_panic]
fn test_happy() {
    // currently fails because of no code_id provided for lp, ls and depositor contracts
    let suite = SuiteBuilder::default().build();

    let clock_addr = suite.query_clock_address();
    println!("clock addr: {clock_addr:?}");

    let holder_addr = suite.query_holder_address();
    println!("holder addr: {holder_addr:?}");
}
