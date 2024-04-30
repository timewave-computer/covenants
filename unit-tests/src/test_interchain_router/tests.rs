use super::suite::InterchainRouterBuilder;

#[test]
#[should_panic]
fn test_instantiate_validates_clock_address() {
    InterchainRouterBuilder::default()
        .with_clock_address("invalid_clock".to_string())
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_destination_receiver_addr() {
    let mut builder = InterchainRouterBuilder::default();
    builder
        .instantiate_msg
        .msg
        .destination_config
        .destination_receiver_addr = "invalid_receiver".to_string();
    builder.build();
}
