use covenant_utils::op_mode::ContractOperationModeConfig;

use super::suite::InterchainRouterBuilder;

#[test]
fn test_instantiate_with_valid_op_mode() {
    let _suite = InterchainRouterBuilder::default().build();
}

#[test]
fn test_instantiate_in_permissionless_mode() {
    let _suite = InterchainRouterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissionless)
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_privileged_accounts() {
    InterchainRouterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![
            "some contract".to_string(),
        ]))
        .build();
}

#[test]
#[should_panic]
fn test_instantiate_validates_empty_privileged_accounts() {
    InterchainRouterBuilder::default()
        .with_op_mode(ContractOperationModeConfig::Permissioned(vec![]))
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
