use cosmwasm_std::{Uint128, Coin};

use crate::{suite_test::suite::{get_equal_split_config, DENOM_B, CLOCK_ADDR, get_public_goods_split_config, ALT_DENOM}, msg::{SplitConfig, SplitType, NativeReceiver, ReceiverType}};

use super::suite::{SuiteBuilder, DENOM_A, PARTY_A_ADDR, PARTY_B_ADDR};



#[test]
fn test_instantiate_happy_and_query_all() {
    let suite = SuiteBuilder::default().build();

    let splits = suite.query_all_splits();
    let token_a_split = suite.query_denom_split(DENOM_A.to_string());
    let token_b_split = suite.query_denom_split(DENOM_B.to_string());
    let clock_addr = suite.query_clock_address();

    assert_eq!(get_equal_split_config(), token_a_split);
    assert_eq!(get_equal_split_config(), token_b_split);
    assert_eq!(CLOCK_ADDR.to_string(), clock_addr);
    assert_eq!(
        vec![
            ("".to_string(), get_public_goods_split_config()),
            (DENOM_A.to_string(), get_equal_split_config()),
            (DENOM_B.to_string(), get_equal_split_config()),
        ],
        splits,
    );
}

#[test]
#[should_panic(expected = "misconfigured split")]
fn test_instantiate_split_misconfig() {
    SuiteBuilder::default()
        .with_custom_splits(vec![(
            DENOM_A.to_string(),
            SplitType::Custom(SplitConfig { 
                receivers: vec![
                    (
                        ReceiverType::Native(NativeReceiver { address: PARTY_A_ADDR.to_string() }),
                        Uint128::new(50),
                    ),
                    (
                        ReceiverType::Native(NativeReceiver { address: PARTY_B_ADDR.to_string() }),
                        Uint128::new(60),
                    ),
                ]
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
                    receivers: vec![
                        (
                            ReceiverType::Native(NativeReceiver { address: PARTY_B_ADDR.to_string() }),
                            Uint128::new(100),
                        ),
                    ]
                })
            ),
            (
                DENOM_B.to_string(),
                SplitType::Custom(SplitConfig { 
                    receivers: vec![
                        (
                            ReceiverType::Native(NativeReceiver { address: PARTY_A_ADDR.to_string() }),
                            Uint128::new(100),
                        ),
                    ]
                })
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
    let mut suite = SuiteBuilder::default().build();

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