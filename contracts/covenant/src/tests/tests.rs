use super::suite::SuiteBuilder;


#[test]
fn test_happy() {
    let suite = SuiteBuilder::default()
        .build();

    let clock_addr = suite.query_clock_address();
    println!("clock addr: {:?}", clock_addr);
}