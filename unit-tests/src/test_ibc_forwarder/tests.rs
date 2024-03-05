use super::suite::IbcForwarderBuilder;

#[test]
fn test_covenant() {
    let suite = IbcForwarderBuilder::default()
        .with_remote_chain_connection_id("some other connection".to_string())
        .build();

    println!("suite rc info: {:?}", suite.remote_chain_info);
}
