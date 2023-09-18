use std::marker::PhantomData;

use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    Attribute, Coin, CosmosMsg, Empty, IbcMsg, IbcTimeout, OwnedDeps, SubMsg, Uint64,
};
use covenant_utils::DestinationConfig;

use crate::{
    contract::{execute, instantiate},
    msg::MigrateMsg,
    suite_tests::suite::{DEFAULT_CHANNEL, DEFAULT_RECEIVER},
};

use super::suite::{SuiteBuilder, CLOCK_ADDR};

#[test]
fn test_instantiate_and_query_all() {
    let suite = SuiteBuilder::default().build();

    let clock = suite.query_clock_addr();
    let config = suite.query_destination_config();

    assert_eq!("clock", clock);
    assert_eq!(
        DestinationConfig {
            destination_chain_channel_id: DEFAULT_CHANNEL.to_string(),
            destination_receiver_addr: DEFAULT_RECEIVER.to_string(),
            ibc_transfer_timeout: Uint64::new(10),
        },
        config
    );
}

#[test]
fn test_migrate_config() {
    let mut suite = SuiteBuilder::default().build();

    let migrate_msg = MigrateMsg::UpdateConfig {
        clock_addr: Some("working_clock".to_string()),
        destination_config: Some(DestinationConfig {
            destination_chain_channel_id: "new_channel".to_string(),
            destination_receiver_addr: "new_receiver".to_string(),
            ibc_transfer_timeout: Uint64::new(100),
        }),
    };

    suite.migrate(migrate_msg).unwrap();

    let clock = suite.query_clock_addr();
    let config = suite.query_destination_config();

    assert_eq!("working_clock", clock);
    assert_eq!(
        DestinationConfig {
            destination_chain_channel_id: "new_channel".to_string(),
            destination_receiver_addr: "new_receiver".to_string(),
            ibc_transfer_timeout: Uint64::new(100),
        },
        config
    );
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_unauthorized_tick() {
    let mut suite = SuiteBuilder::default().build();
    suite.tick("not_the_clock");
}

#[test]
fn test_tick() {
    let querier: MockQuerier<Empty> = MockQuerier::new(&[("cosmos2contract", &coins(100, "usdc"))]);

    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::new(&[]),
        custom_query_type: PhantomData,
    };
    // set the custom querier on our mock deps
    deps.querier = querier;

    let info = mock_info(CLOCK_ADDR, &[]);
    let init_msg = SuiteBuilder::default().instantiate;

    instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        SuiteBuilder::default().instantiate,
    )
    .unwrap();

    let resp = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CLOCK_ADDR, &[]),
        crate::msg::ExecuteMsg::Tick {},
    )
    .unwrap();
    let mock_env = mock_env();
    let expected_messages = vec![SubMsg {
        id: 0,
        msg: CosmosMsg::Ibc(IbcMsg::Transfer {
            amount: Coin::new(100, "usdc"),
            channel_id: DEFAULT_CHANNEL.to_string(),
            to_address: DEFAULT_RECEIVER.to_string(),
            timeout: IbcTimeout::with_timestamp(
                mock_env
                    .block
                    .time
                    .plus_seconds(init_msg.ibc_transfer_timeout.u64()),
            ),
        }),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Never,
    }];
    let expected_attributes = vec![
        Attribute {
            key: "method".to_string(),
            value: "try_route_balances".to_string(),
        },
        Attribute {
            key: "usdc".to_string(),
            value: "100".to_string(),
        },
    ];

    // assert the expected response attributes and messages
    assert_eq!(expected_messages, resp.messages);
    assert_eq!(expected_attributes, resp.attributes);
}
