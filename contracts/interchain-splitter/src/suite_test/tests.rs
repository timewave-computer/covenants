use cosmwasm_std::{Coin, Uint128};

use crate::{
    msg::{MigrateMsg, Receiver, SplitConfig, SplitType},
    suite_test::suite::{
        get_equal_split_config, get_fallback_split_config, ALT_DENOM, CLOCK_ADDR, DENOM_B,
    },
};

use super::suite::{SuiteBuilder, DENOM_A, PARTY_A_ADDR, PARTY_B_ADDR};

#[test]
fn test_instantiate_happy_and_query_all() {
    let suite = SuiteBuilder::default().build();

    let splits = suite.query_all_splits();
    let token_a_split = suite.query_denom_split(DENOM_A.to_string());
    let token_b_split = suite.query_denom_split(DENOM_B.to_string());
    let clock_addr = suite.query_clock_address();
    let fallback_split = suite.query_fallback_split();

    assert_eq!(get_equal_split_config(), token_a_split);
    assert_eq!(get_equal_split_config(), token_b_split);
    assert_eq!(CLOCK_ADDR.to_string(), clock_addr);
    assert_eq!(
        vec![
            (DENOM_A.to_string(), get_equal_split_config()),
            (DENOM_B.to_string(), get_equal_split_config()),
        ],
        splits,
    );
    assert_eq!(None, fallback_split);
}

#[test]
#[should_panic(expected = "misconfigured split")]
fn test_instantiate_split_misconfig() {
    SuiteBuilder::default()
        .with_custom_splits(vec![(
            DENOM_A.to_string(),
            SplitType::Custom(SplitConfig {
                receivers: vec![
                    Receiver {
                        addr: PARTY_A_ADDR.to_string(),
                        share: Uint128::new(50),
                    },
                    Receiver {
                        addr: PARTY_B_ADDR.to_string(),
                        share: Uint128::new(50),
                    },
                ],
            }),
        )])
        .build();
}

#[test]
fn test_distribute_equal_split() {
    let mut suite = SuiteBuilder::default().build();

    // fund the splitter with 100 of each denom
    suite.fund_coin(Coin::new(100, DENOM_A));
    suite.fund_coin(Coin::new(100, DENOM_B));

    suite.pass_blocks(10);

    // assert splitter is funded
    let splitter_denom_a_bal = suite.get_party_denom_balance(DENOM_A, suite.splitter.as_str());
    let splitter_denom_b_bal = suite.get_party_denom_balance(DENOM_B, suite.splitter.as_str());
    assert_eq!(Uint128::new(100), splitter_denom_a_bal);
    assert_eq!(Uint128::new(100), splitter_denom_b_bal);

    // tick initiates the distribution attempt
    suite.tick(CLOCK_ADDR).unwrap();
    suite.pass_blocks(10);

    let party_a_denom_a_bal = suite.get_party_denom_balance(DENOM_A, PARTY_A_ADDR);
    let party_a_denom_b_bal = suite.get_party_denom_balance(DENOM_B, PARTY_A_ADDR);
    let party_b_denom_a_bal = suite.get_party_denom_balance(DENOM_A, PARTY_B_ADDR);
    let party_b_denom_b_bal = suite.get_party_denom_balance(DENOM_B, PARTY_B_ADDR);
    let splitter_denom_a_bal = suite.get_party_denom_balance(DENOM_A, suite.splitter.as_str());
    let splitter_denom_b_bal = suite.get_party_denom_balance(DENOM_B, suite.splitter.as_str());

    assert_eq!(Uint128::new(50), party_a_denom_a_bal);
    assert_eq!(Uint128::new(50), party_a_denom_b_bal);
    assert_eq!(Uint128::new(50), party_b_denom_a_bal);
    assert_eq!(Uint128::new(50), party_b_denom_b_bal);
    assert_eq!(Uint128::zero(), splitter_denom_a_bal);
    assert_eq!(Uint128::zero(), splitter_denom_b_bal);
}

#[test]
fn test_distribute_token_swap() {
    let mut suite = SuiteBuilder::default()
        .with_custom_splits(vec![
            (
                DENOM_A.to_string(),
                SplitType::Custom(SplitConfig {
                    receivers: vec![Receiver {
                        addr: PARTY_B_ADDR.to_string(),
                        share: Uint128::new(100),
                    }],
                }),
            ),
            (
                DENOM_B.to_string(),
                SplitType::Custom(SplitConfig {
                    receivers: vec![Receiver {
                        addr: PARTY_A_ADDR.to_string(),
                        share: Uint128::new(100),
                    }],
                }),
            ),
        ])
        .build();

    // fund the splitter with 100 of each denom
    suite.fund_coin(Coin::new(100, DENOM_A));
    suite.fund_coin(Coin::new(100, DENOM_B));

    suite.pass_blocks(10);

    // assert splitter is funded
    let splitter_denom_a_bal = suite.get_party_denom_balance(DENOM_A, suite.splitter.as_str());
    let splitter_denom_b_bal = suite.get_party_denom_balance(DENOM_B, suite.splitter.as_str());
    assert_eq!(Uint128::new(100), splitter_denom_a_bal);
    assert_eq!(Uint128::new(100), splitter_denom_b_bal);

    // tick initiates the distribution attempt
    suite.tick(CLOCK_ADDR).unwrap();
    suite.pass_blocks(10);

    let party_a_denom_a_bal = suite.get_party_denom_balance(DENOM_A, PARTY_A_ADDR);
    let party_a_denom_b_bal = suite.get_party_denom_balance(DENOM_B, PARTY_A_ADDR);
    let party_b_denom_a_bal = suite.get_party_denom_balance(DENOM_A, PARTY_B_ADDR);
    let party_b_denom_b_bal = suite.get_party_denom_balance(DENOM_B, PARTY_B_ADDR);
    let splitter_denom_a_bal = suite.get_party_denom_balance(DENOM_A, suite.splitter.as_str());
    let splitter_denom_b_bal = suite.get_party_denom_balance(DENOM_B, suite.splitter.as_str());

    assert_eq!(Uint128::zero(), party_a_denom_a_bal);
    assert_eq!(Uint128::new(100), party_a_denom_b_bal);
    assert_eq!(Uint128::new(100), party_b_denom_a_bal);
    assert_eq!(Uint128::zero(), party_b_denom_b_bal);
    assert_eq!(Uint128::zero(), splitter_denom_a_bal);
    assert_eq!(Uint128::zero(), splitter_denom_b_bal);
}

#[test]
fn test_distribute_fallback() {
    let mut suite = SuiteBuilder::default()
        .with_fallback_split(get_fallback_split_config())
        .build();

    // fund the splitter with 100 of some random token not part of the config
    suite.fund_coin(Coin::new(100, ALT_DENOM.to_string()));

    suite.pass_blocks(10);

    // assert splitter is funded
    let splitter_alt_denom_bal = suite.get_party_denom_balance(ALT_DENOM, suite.splitter.as_str());
    assert_eq!(Uint128::new(100), splitter_alt_denom_bal);

    // tick initiates the distribution attempt
    suite.tick(CLOCK_ADDR).unwrap();
    suite.pass_blocks(10);

    let save_the_cats_foundation_bal = suite.get_party_denom_balance(ALT_DENOM, "save_the_cats");
    let splitter_alt_denom_bal = suite.get_party_denom_balance(ALT_DENOM, suite.splitter.as_str());

    assert_eq!(Uint128::zero(), splitter_alt_denom_bal);
    assert_eq!(Uint128::new(100), save_the_cats_foundation_bal);
}

#[test]
fn test_migrate_config() {
    let mut suite = SuiteBuilder::default().build();

    let new_clock = "new_clock".to_string();
    let new_fallback_split = SplitConfig {
        receivers: vec![Receiver {
            addr: "fallback_new".to_string(),
            share: Uint128::new(100),
        }],
    };
    let new_splits = vec![(
        "new_denom".to_string(),
        SplitType::Custom(SplitConfig {
            receivers: vec![Receiver {
                addr: "new_receiver".to_string(),
                share: Uint128::new(100),
            }],
        }),
    )];

    let migrate_msg = MigrateMsg::UpdateConfig {
        clock_addr: Some(new_clock.clone()),
        fallback_split: Some(new_fallback_split.clone()),
        splits: Some(new_splits),
    };

    suite.migrate(migrate_msg).unwrap();

    let splits = suite.query_all_splits();
    let clock_addr = suite.query_clock_address();
    let fallback_split = suite.query_fallback_split();

    assert_eq!(
        vec![(
            "new_denom".to_string(),
            SplitConfig {
                receivers: vec![Receiver {
                    addr: "new_receiver".to_string(),
                    share: Uint128::new(100)
                },],
            },
        )],
        splits
    );
    assert_eq!(Some(new_fallback_split), fallback_split);
    assert_eq!(new_clock, clock_addr);
}
