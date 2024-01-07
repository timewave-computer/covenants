use std::{collections::BTreeSet, marker::PhantomData};

use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    Attribute, CosmosMsg, Empty, OwnedDeps, SubMsg, Uint128, Uint64,
};
use covenant_utils::{DestinationConfig, ReceiverConfig};
use neutron_sdk::{
    bindings::msg::{IbcFee, NeutronMsg},
    sudo::msg::RequestPacketTimeoutHeight,
    NeutronError,
};

use crate::{
    contract::{execute, instantiate},
    msg::MigrateMsg,
    suite_tests::suite::{DEFAULT_CHANNEL, DEFAULT_RECEIVER},
};

use super::suite::{SuiteBuilder, CLOCK_ADDR};

#[test]
fn test_instantiate_and_query_all() {
    let suite = SuiteBuilder::default().build();

    let clock = suite.query_clock_addr().to_string();
    let config = suite.query_destination_config();
    let denoms = suite.query_target_denoms();

    assert_eq!("contract0", clock);
    assert_eq!(
        ReceiverConfig::Ibc(DestinationConfig {
            destination_chain_channel_id: DEFAULT_CHANNEL.to_string(),
            destination_receiver_addr: DEFAULT_RECEIVER.to_string(),
            ibc_transfer_timeout: Uint64::new(10),
        }),
        config
    );
    assert_eq!(BTreeSet::new(), denoms);
}

#[test]
fn test_migrate_config() {
    let mut suite = SuiteBuilder::default().build();
    let target_denom_vec = vec!["new_denom_1".to_string(), "new_denom_2".to_string()];
    let target_denom_set: BTreeSet<String> = target_denom_vec.clone().into_iter().collect();
    let migrate_msg = MigrateMsg::UpdateConfig {
        clock_addr: Some("working_clock".to_string()),
        receiver_config: Some(covenant_utils::ReceiverConfig::Ibc(DestinationConfig {
            destination_chain_channel_id: "new_channel".to_string(),
            destination_receiver_addr: "new_receiver".to_string(),
            ibc_transfer_timeout: Uint64::new(100),
        })),
        target_denoms: Some(target_denom_vec),
    };

    suite.migrate(migrate_msg).unwrap();

    let clock = suite.query_clock_addr();
    let config = suite.query_destination_config();
    let target_denoms = suite.query_target_denoms();

    assert_eq!("working_clock", clock);
    assert_eq!(
        ReceiverConfig::Ibc(DestinationConfig {
            destination_chain_channel_id: "new_channel".to_string(),
            destination_receiver_addr: "new_receiver".to_string(),
            ibc_transfer_timeout: Uint64::new(100),
        }),
        config
    );
    assert_eq!(target_denom_set, target_denoms);
}

#[test]
#[should_panic(expected = "Caller is not the clock, only clock can tick contracts")]
fn test_unauthorized_tick() {
    let mut suite = SuiteBuilder::default().build();
    suite.tick("not_the_clock");
}

#[test]
fn test_tick() {
    let usdc_coin = coin(100, "usdc");
    let random_coin_1 = coin(100, "denom1");
    let random_coin_2 = coin(100, "denom2");
    let random_coin_3 = coin(100, "denom3");

    let coins = vec![usdc_coin, random_coin_1, random_coin_2, random_coin_3];
    let querier: MockQuerier<Empty> = MockQuerier::new(&[("cosmos2contract", &coins)]);

    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::new(&[]),
        custom_query_type: PhantomData,
    };
    // set the custom querier on our mock deps
    deps.querier = querier;

    let info = mock_info(CLOCK_ADDR, &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        SuiteBuilder::default()
            .with_denoms(vec!["usdc".to_string()])
            .instantiate,
    )
    .unwrap();

    let resp = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        crate::msg::ExecuteMsg::Tick {},
    )
    .unwrap();
    let mock_env = mock_env();
    let msg_exp = CosmosMsg::Custom(NeutronMsg::IbcTransfer {
        source_port: "transfer".to_string(),
        source_channel: "channel-1".to_string(),
        token: coin(100, "usdc"),
        sender: "cosmos2contract".to_string(),
        receiver: "receiver".to_string(),
        timeout_height: RequestPacketTimeoutHeight {
            revision_number: None,
            revision_height: None,
        },
        timeout_timestamp: mock_env
            .block
            .time
            .plus_seconds(Uint64::new(10).u64())
            .nanos(),
        memo: format!("ibc_distribution: denom1:{:?}", Uint128::new(100)),
        fee: IbcFee {
            // must be empty
            recv_fee: vec![],
            ack_fee: vec![cosmwasm_std::Coin {
                denom: "untrn".to_string(),
                amount: Uint128::new(100000),
            }],
            timeout_fee: vec![cosmwasm_std::Coin {
                denom: "untrn".to_string(),
                amount: Uint128::new(100000),
            }],
        },
    });
    let _expected_messages = vec![SubMsg {
        id: 0,
        msg: msg_exp,
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
    assert_eq!(expected_attributes, resp.attributes);

    // try to use the fallback method to distribute
    // explicitly defined denom
    let err = execute(
        deps.as_mut(),
        mock_env.clone(),
        info.clone(),
        crate::msg::ExecuteMsg::DistributeFallback {
            denoms: vec!["usdc".to_string()],
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        NeutronError::Std(cosmwasm_std::StdError::generic_err(
            "unauthorized denom distribution".to_string()
        ))
    );

    // now distribute a valid fallback denom
    let resp = execute(
        deps.as_mut(),
        mock_env,
        info,
        crate::msg::ExecuteMsg::DistributeFallback {
            denoms: vec!["denom1".to_string()],
        },
    )
    .unwrap();

    for msg in resp.messages {
        assert_eq!(
            msg,
            SubMsg {
                id: 0,
                msg: CosmosMsg::Custom(NeutronMsg::IbcTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: "channel-1".to_string(),
                    token: cosmwasm_std::Coin::new(100, "denom1".to_string()),
                    sender: "cosmos2contract".to_string(),
                    receiver: "receiver".to_string(),
                    timeout_height: RequestPacketTimeoutHeight {
                        revision_number: None,
                        revision_height: None
                    },
                    timeout_timestamp: 1571797429879305533,
                    memo: format!("ibc_distribution: {:?}:{:?}", "denom1", Uint128::new(100),)
                        .to_string(),
                    fee: IbcFee {
                        recv_fee: vec![],
                        ack_fee: vec![cosmwasm_std::coin(100000, "untrn".to_string())],
                        timeout_fee: vec![cosmwasm_std::coin(100000, "untrn".to_string())],
                    },
                },),
                gas_limit: None,
                reply_on: cosmwasm_std::ReplyOn::Never,
            }
        );
    }
}
