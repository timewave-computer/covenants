use std::{collections::BTreeMap, str::FromStr, thread, time::Duration};

use cosmwasm_std::{coin, Binary, Coin, Decimal, Uint128, Uint64};
use covenant_utils::{
    op_mode::ContractOperationModeConfig, split::SplitConfig, InterchainCovenantParty,
    NativeCovenantParty, PoolPriceConfig, SingleSideLpLimits,
};
use cw_utils::Expiration;
use localic_std::{
    errors::LocalError,
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query},
    node::Chain,
};
use valence_astroport_liquid_pooler::msg::AstroportLiquidPoolerConfig;
use valence_covenant_two_party_pol::msg::{CovenantContractCodeIds, CovenantPartyConfig, Timeouts};
use valence_two_party_pol_holder::msg::{CovenantType, RagequitConfig, RagequitTerms};

use crate::utils::{
    constants::{
        ACC1_ADDRESS_GAIA, ACC1_ADDRESS_NEUTRON, ACC2_ADDRESS_NEUTRON, ACC_0_KEY, ASTROPORT_PATH,
        EXECUTE_FLAGS, GAIA_CHAIN, NEUTRON_CHAIN, VALENCE_PATH,
    },
    ibc::ibc_send,
    setup::deploy_contracts_on_chain,
    test_context::TestContext,
};

use astroport::{
    asset::Asset, native_coin_registry::InstantiateMsg as NativeCoinRegistryInstantiateMsg,
};
use astroport::{
    asset::AssetInfo, factory::InstantiateMsg as FactoryInstantiateMsg, pair::StablePoolParams,
};
use astroport::{
    factory::{PairConfig, PairType},
    native_coin_registry::ExecuteMsg as NativeCoinRegistryExecuteMsg,
};

pub fn test_two_party_pol_native(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    //deploy_contracts_on_chain(test_ctx, VALENCE_PATH, NEUTRON_CHAIN);
    //deploy_contracts_on_chain(test_ctx, ASTROPORT_PATH, NEUTRON_CHAIN);

    /*let astroport_native_coin_registry_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_native_coin_registry")
        .unwrap()
        .clone();

    let astroport_pair_stable_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_pair_stable")
        .unwrap()
        .clone();

    let astroport_token_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_token")
        .unwrap()
        .clone();

    let astroport_whitelist_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_whitelist")
        .unwrap()
        .clone();

    let astroport_factory_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_factory")
        .unwrap()
        .clone();*/

    // Instantiate the native coin registry contractf
    let native_coin_registry_instantiate_msg = NativeCoinRegistryInstantiateMsg {
        owner: test_ctx.get_admin_addr().src(NEUTRON_CHAIN).get(),
    };
    let native_coin_registry_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        ACC_0_KEY,
        /*astroport_native_coin_registry_code_id,*/
        26,
        &serde_json::to_string(&native_coin_registry_instantiate_msg).unwrap(),
        "native-coin-registry",
        None,
        "",
    )?;
    println!(
        "Native coin registry contract: {:?}",
        native_coin_registry_contract.address
    );

    // Add ATOM and NTRN to coin registry
    let atom_on_neutron_denom = test_ctx
        .get_ibc_denoms()
        .src(GAIA_CHAIN)
        .dest(NEUTRON_CHAIN)
        .get();
    let atom_denom = test_ctx.get_native_denom().src(GAIA_CHAIN).get();
    let neutron_denom = test_ctx.get_native_denom().src(NEUTRON_CHAIN).get();

    let add_to_registry_msg = NativeCoinRegistryExecuteMsg::Add {
        native_coins: vec![
            (atom_on_neutron_denom.clone(), 6),
            (neutron_denom.clone(), 6),
        ],
    };
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        &native_coin_registry_contract.address,
        ACC_0_KEY,
        &serde_json::to_string(&add_to_registry_msg).unwrap(),
        EXECUTE_FLAGS,
    )?;
    thread::sleep(Duration::from_secs(3));

    // Instantiate the factory contract
    let factory_instantiate_msg = FactoryInstantiateMsg {
        pair_configs: vec![PairConfig {
            /*code_id: astroport_pair_stable_code_id,*/
            code_id: 29,
            pair_type: PairType::Stable {},
            total_fee_bps: 0,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: true,
        }],
        /*token_code_id: astroport_token_code_id,*/
        token_code_id: 28,
        fee_address: None,
        generator_address: None,
        owner: test_ctx.get_admin_addr().src(NEUTRON_CHAIN).get(),
        /*whitelist_code_id: astroport_whitelist_code_id,*/
        whitelist_code_id: 20,
        coin_registry_address: native_coin_registry_contract.address.to_string(),
    };
    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        ACC_0_KEY,
        /*astroport_factory_code_id,*/
        23,
        &serde_json::to_string(&factory_instantiate_msg).unwrap(),
        "astroport-factory",
        None,
        "",
    )?;
    println!("Factory contract: {:?}", factory_contract.address);

    // Create the stable pair ATOM/NTRN
    let create_pair_msg = astroport::factory::ExecuteMsg::CreatePair {
        pair_type: PairType::Stable {},
        asset_infos: vec![
            AssetInfo::NativeToken {
                denom: atom_on_neutron_denom.clone(),
            },
            AssetInfo::NativeToken {
                denom: neutron_denom.clone(),
            },
        ],
        init_params: Some(Binary::from(
            serde_json::to_vec(&StablePoolParams {
                amp: 3,
                owner: None,
            })
            .unwrap(),
        )),
    };
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        &factory_contract.address,
        ACC_0_KEY,
        &serde_json::to_string(&create_pair_msg).unwrap(),
        EXECUTE_FLAGS,
    )?;

    // Send some ATOM to NTRN
    let amount_to_send = 5_000_000_000;
    ibc_send(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN),
        ACC_0_KEY,
        &test_ctx.get_admin_addr().src(NEUTRON_CHAIN).get(),
        coin(amount_to_send, atom_denom.clone()),
        coin(100000, atom_denom.clone()),
        &test_ctx
            .get_transfer_channels()
            .src(GAIA_CHAIN)
            .dest(NEUTRON_CHAIN)
            .get(),
        None,
    )?;
    thread::sleep(Duration::from_secs(5));

    // Provide the ATOM/NTRN liquidity to the pair
    let pair_info = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        &factory_contract.address,
        &serde_json::to_string(&astroport::factory::QueryMsg::Pair {
            asset_infos: vec![
                AssetInfo::NativeToken {
                    denom: atom_on_neutron_denom.clone(),
                },
                AssetInfo::NativeToken {
                    denom: neutron_denom.clone(),
                },
            ],
        })
        .unwrap(),
    );
    let pool_addr = pair_info["data"]["contract_addr"].as_str().unwrap();

    let uatom_contribution_amount: u128 = 5_000_000_000;
    let untrn_contribution_amount: u128 = 50_000_000_000;
    let provide_liquidity_msg = astroport::pair::ExecuteMsg::ProvideLiquidity {
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: atom_on_neutron_denom.clone(),
                },
                amount: Uint128::from(uatom_contribution_amount),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: neutron_denom.clone(),
                },
                amount: Uint128::from(untrn_contribution_amount),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        auto_stake: Some(false),
        receiver: Some(test_ctx.get_admin_addr().src(NEUTRON_CHAIN).get()),
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        pool_addr,
        ACC_0_KEY,
        &serde_json::to_string(&provide_liquidity_msg).unwrap(),
        &format!("--amount {uatom_contribution_amount}{atom_on_neutron_denom},{untrn_contribution_amount}{neutron_denom} {EXECUTE_FLAGS}"),
    ).unwrap();
    thread::sleep(Duration::from_secs(3));

    // Instantiate the covenant
    let chain = Chain::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
    );
    let current_block_height = chain.get_height();

    /*let valence_ibc_forwarder_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_ibc_forwarder")
        .unwrap()
        .clone();

    let valence_two_party_pol_holder_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_two_party_pol_holder")
        .unwrap()
        .clone();

    let valence_clock_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_clock")
        .unwrap()
        .clone();

    let valence_interchain_router_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_interchain_router")
        .unwrap()
        .clone();

    let valence_native_router_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_native_router")
        .unwrap()
        .clone();

    let valence_liquid_pooler_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_astroport_liquid_pooler")
        .unwrap()
        .clone();

    let valence_covenant_two_party_pol_code_id = test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_covenant_two_party_pol")
        .unwrap()
        .clone();*/

    // Two party POL happy path
    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "two-party-pol-covenant-happy".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(10000),          // seconds
            ibc_transfer_timeout: Uint64::new(10000), // seconds
        },
        contract_codes: CovenantContractCodeIds {
            //ibc_forwarder_code: valence_ibc_forwarder_code_id,
            ibc_forwarder_code: 4,
            //holder_code: valence_two_party_pol_holder_code_id,
            holder_code: 11,
            //clock_code: valence_clock_code_id,
            clock_code: 17,
            //interchain_router_code: valence_interchain_router_code_id,
            interchain_router_code: 3,
            //native_router_code: valence_native_router_code_id,
            native_router_code: 14,
            //liquid_pooler_code: valence_liquid_pooler_code_id,
            liquid_pooler_code: 7,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 130),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(10000),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            remote_chain_denom: atom_denom.clone(),
            addr: ACC1_ADDRESS_NEUTRON.to_string(),
            native_denom: atom_on_neutron_denom.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Native(NativeCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: neutron_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            contribution: Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(untrn_contribution_amount),
            },
        }),
        covenant_type: CovenantType::Share,
        ragequit_config: Some(RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.1").unwrap(),
            state: None,
        })),
        deposit_deadline: Expiration::AtHeight(current_block_height + 110),
        party_a_share: Decimal::percent(50),
        party_b_share: Decimal::percent(50),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.1").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.09").unwrap(),
        },
        splits: BTreeMap::from([
            (
                atom_on_neutron_denom.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
            (
                neutron_denom.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
        ]),
        fallback_split: None,
        emergency_committee: None,
        liquid_pooler_config: valence_covenant_two_party_pol::msg::LiquidPoolerConfig::Astroport(
            AstroportLiquidPoolerConfig {
                pool_pair_type: PairType::Stable {},
                pool_address: pool_addr.to_string(),
                asset_a_denom: atom_on_neutron_denom.clone(),
                asset_b_denom: neutron_denom.clone(),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(100000),
                    asset_b_limit: Uint128::new(100000),
                },
            },
        ),
        fallback_address: None,
        operation_mode: ContractOperationModeConfig::Permissionless,
    };

    let contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        ACC_0_KEY,
        //valence_covenant_two_party_pol_code_id,
        8,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "two-party-pol-covenant",
        None,
        "",
    )?;

    Ok(())
}
