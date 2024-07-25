use std::{collections::BTreeMap, str::FromStr, thread, time::Duration};

use astroport::{
    asset::{Asset, AssetInfo},
    factory::{PairConfig, PairType},
    pair::StablePoolParams,
};
use cosmwasm_std::{Binary, Coin, Decimal, Uint128, Uint64};
use covenant_utils::{InterchainCovenantParty, PoolPriceConfig, SingleSideLpLimits};
use cw_utils::Expiration;
use localic_std::{
    errors::LocalError,
    modules::{
        bank::{get_balance, send},
        cosmwasm::{contract_execute, contract_instantiate, contract_query},
    },
};
use localic_utils::{
    types::ibc::get_multihop_ibc_denom, utils::test_context::TestContext, ADMIN_KEY, DEFAULT_KEY,
    GAIA_CHAIN_NAME, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME, STRIDE_CHAIN_ADMIN_ADDR,
    STRIDE_CHAIN_NAME,
};
use log::info;
use valence_astroport_liquid_pooler::msg::AstroportLiquidPoolerConfig;
// use valence_astroport_tf_liquid_pooler::msg::AstroportLiquidPoolerConfig;
use valence_covenant_single_party_pol::msg::{
    CovenantContractCodeIds, CovenantPartyConfig, LiquidPoolerConfig, LsInfo,
    RemoteChainSplitterConfig, Timeouts,
};

use crate::{
    helpers::{
        astroport::{get_lp_token_address, get_lp_token_balance, get_pool_address},
        common::{query_contract_state, tick},
        constants::{
            ACC1_ADDRESS_GAIA, ACC1_ADDRESS_NEUTRON, ACC2_ADDRESS_NEUTRON, ACC_1_KEY,
            ASTROPORT_PATH, EXECUTE_FLAGS, LOCAL_CODE_ID_CACHE_PATH, VALENCE_PATH,
        },
        covenant::Covenant,
    },
    send_non_native_balances,
};

const NATIVE_STATOM_DENOM: &str = "stuatom";

pub fn test_astroport_tokenfactory_liquid_pooler(
    test_ctx: &mut TestContext,
) -> Result<(), LocalError> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    uploader
        .send_with_local_cache(VALENCE_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    uploader
        .send_with_local_cache(ASTROPORT_PATH, NEUTRON_CHAIN_NAME, LOCAL_CODE_ID_CACHE_PATH)
        .unwrap();

    info!("Starting single party POL tests...");
    let astroport_native_coin_registry_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("astroport_native_coin_registry")
        .unwrap();

    let astroport_pair_stable_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("astroport_pair_stable")
        .unwrap();

    let astroport_token_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("astroport_token")
        .unwrap();

    let astroport_whitelist_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("astroport_whitelist")
        .unwrap();

    let astroport_factory_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("astroport_factory")
        .unwrap();

    let native_coin_registry_instantiate_msg = astroport::native_coin_registry::InstantiateMsg {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
    };

    let native_coin_registry_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        astroport_native_coin_registry_code_id,
        &serde_json::to_string(&native_coin_registry_instantiate_msg).unwrap(),
        "native-coin-registry",
        None,
        "",
    )?;

    let factory_instantiate_msg = astroport::factory::InstantiateMsg {
        pair_configs: vec![PairConfig {
            code_id: astroport_pair_stable_code_id,
            pair_type: PairType::Stable {},
            total_fee_bps: 0,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: true,
            permissioned: true,
        }],
        token_code_id: astroport_token_code_id,
        fee_address: None,
        generator_address: None,
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        whitelist_code_id: astroport_whitelist_code_id,
        coin_registry_address: native_coin_registry_contract.address.to_string(),
        tracker_config: None,
    };

    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        astroport_factory_code_id,
        &serde_json::to_string(&factory_instantiate_msg).unwrap(),
        "astroport-factory",
        None,
        "",
    )?;

    let atom_denom = test_ctx.get_native_denom().src(GAIA_CHAIN_NAME).get();
    let neutron_denom = test_ctx.get_native_denom().src(NEUTRON_CHAIN_NAME).get();
    let atom_on_stride = test_ctx.get_ibc_denom(&atom_denom, GAIA_CHAIN_NAME, STRIDE_CHAIN_NAME);
    // Add the coins to the registry
    let statom_on_neutron =
        test_ctx.get_ibc_denom(NATIVE_STATOM_DENOM, STRIDE_CHAIN_NAME, NEUTRON_CHAIN_NAME);

    let atom_on_neutron = test_ctx.get_ibc_denom(&atom_denom, GAIA_CHAIN_NAME, NEUTRON_CHAIN_NAME);
    let statom_on_gaia_via_neutron = get_multihop_ibc_denom(
        NATIVE_STATOM_DENOM,
        vec![
            &test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            &test_ctx
                .get_transfer_channels()
                .src(STRIDE_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
        ],
    );

    let add_to_registry_msg = astroport::native_coin_registry::ExecuteMsg::Add {
        native_coins: vec![(atom_on_neutron.clone(), 6), (statom_on_neutron.clone(), 6)],
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &native_coin_registry_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(&add_to_registry_msg).unwrap(),
        EXECUTE_FLAGS,
    )?;

    // Wait for the coins to be added
    thread::sleep(Duration::from_secs(5));

    // Create the stable pair
    let create_pair_msg = astroport::factory::ExecuteMsg::CreatePair {
        pair_type: PairType::Stable {},
        asset_infos: vec![
            AssetInfo::NativeToken {
                denom: atom_on_neutron.clone(),
            },
            AssetInfo::NativeToken {
                denom: statom_on_neutron.clone(),
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
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(&create_pair_msg).unwrap(),
        EXECUTE_FLAGS,
    )?;

    info!("Liquid Stake some ATOM");
    let amount_to_liquid_stake = 100_000_000_000;
    loop {
        test_ctx
            .build_tx_transfer()
            .with_chain_name(GAIA_CHAIN_NAME)
            .with_amount(amount_to_liquid_stake)
            .with_recipient(STRIDE_CHAIN_ADMIN_ADDR)
            .with_denom(&atom_denom)
            .send()
            .unwrap();

        info!("Waiting to receive IBC transfer...");
        thread::sleep(Duration::from_secs(5));
        let balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(STRIDE_CHAIN_NAME),
            STRIDE_CHAIN_ADMIN_ADDR,
        );
        if balance.iter().any(|c| c.denom == atom_on_stride) {
            break;
        }
    }

    // Liquid stake the ibc'd atoms for stuatom
    test_ctx
        .build_tx_liquid_stake()
        .with_key(ADMIN_KEY)
        .with_amount(amount_to_liquid_stake)
        .with_denom(&atom_denom)
        .send()
        .unwrap();

    info!("Send the StATOM and some ATOM to Neutron...");
    test_ctx
        .build_tx_transfer()
        .with_chain_name(STRIDE_CHAIN_NAME)
        .with_key(ADMIN_KEY)
        .with_amount(amount_to_liquid_stake)
        .with_recipient(NEUTRON_CHAIN_ADMIN_ADDR)
        .with_denom(NATIVE_STATOM_DENOM)
        .send()
        .unwrap();

    let uatom_contribution_amount: u128 = 5_000_000_000;
    loop {
        test_ctx
            .build_tx_transfer()
            .with_chain_name(GAIA_CHAIN_NAME)
            .with_amount(uatom_contribution_amount)
            .with_recipient(NEUTRON_CHAIN_ADMIN_ADDR)
            .with_denom(&atom_denom)
            .send()
            .unwrap();

        info!("Waiting to receive IBC transfers...");
        thread::sleep(Duration::from_secs(5));
        let balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            NEUTRON_CHAIN_ADMIN_ADDR,
        );
        if balance.iter().any(|c| c.denom == atom_on_neutron)
            && balance.iter().any(|c| c.denom == statom_on_neutron)
        {
            break;
        }
    }

    let pool_addr = get_pool_address(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron.clone(),
        },
        AssetInfo::NativeToken {
            denom: statom_on_neutron.clone(),
        },
    );

    let liquidity_contribution = uatom_contribution_amount / 2;
    let provide_liquidity_msg = astroport::pair::ExecuteMsg::ProvideLiquidity {
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: atom_on_neutron.clone(),
                },
                amount: Uint128::from(liquidity_contribution),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: statom_on_neutron.clone(),
                },
                amount: Uint128::from(liquidity_contribution),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        auto_stake: Some(false),
        receiver: Some(NEUTRON_CHAIN_ADMIN_ADDR.to_string()),
        min_lp_to_receive: None,
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &pool_addr,
        DEFAULT_KEY,
        &serde_json::to_string(&provide_liquidity_msg).unwrap(),
        &format!(
            "--amount {liquidity_contribution}{atom_on_neutron},{liquidity_contribution}{statom_on_neutron} {EXECUTE_FLAGS}"
        ),
    )?;

    thread::sleep(Duration::from_secs(5));

    let lp_token_address = get_lp_token_address(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron.clone(),
        },
        AssetInfo::NativeToken {
            denom: statom_on_neutron.clone(),
        },
    );

    let balance = get_lp_token_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &lp_token_address,
        &NEUTRON_CHAIN_ADMIN_ADDR,
    );
    info!("Neutron User LP Token balance: {}", balance);

    let code_id_ibc_forwarder = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_ibc_forwarder")
        .unwrap();

    let code_id_single_party_pol_holder = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_single_party_pol_holder")
        .unwrap();

    let code_id_remote_chain_splitter = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_remote_chain_splitter")
        .unwrap();

    let code_id_astroport_liquid_pooler = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_astroport_tf_liquid_pooler")
        .unwrap();

    let code_id_stride_liquid_staker = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_stride_liquid_staker")
        .unwrap();

    let code_id_interchain_router = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_interchain_router")
        .unwrap();

    let code_id_single_party_pol_covenant = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_covenant_single_party_pol")
        .unwrap();

    let code_id_clock = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_clock")
        .unwrap();

    // Instantiate the covenant
    let chain = localic_std::node::Chain::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );
    let current_height = chain.get_height();

    let instantiate_covenant_msg = valence_covenant_single_party_pol::msg::InstantiateMsg {
        label: "single_party_pol_stride_covenant".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(10000),
            ibc_transfer_timeout: Uint64::new(10000),
        },
        contract_codes: CovenantContractCodeIds {
            ibc_forwarder_code: code_id_ibc_forwarder,
            holder_code: code_id_single_party_pol_holder,
            remote_chain_splitter_code: code_id_remote_chain_splitter,
            liquid_pooler_code: code_id_astroport_liquid_pooler,
            liquid_staker_code: code_id_stride_liquid_staker,
            interchain_router_code: code_id_interchain_router,
            clock_code: code_id_clock,
        },
        clock_tick_max_gas: None,
        lockup_period: Expiration::AtHeight(current_height + 110),
        ls_info: LsInfo {
            ls_denom: NATIVE_STATOM_DENOM.to_string(),
            ls_denom_on_neutron: statom_on_neutron.to_string(),
            ls_chain_to_neutron_channel_id: test_ctx
                .get_transfer_channels()
                .src(STRIDE_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
            ls_neutron_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(STRIDE_CHAIN_NAME)
                .get(),
        },
        ls_forwarder_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_NEUTRON.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(10000),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN_NAME)
                .dest(STRIDE_CHAIN_NAME)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(STRIDE_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            remote_chain_denom: atom_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.to_string(),
                amount: Uint128::new(liquidity_contribution),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        lp_forwarder_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_NEUTRON.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(10000),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            remote_chain_denom: atom_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(liquidity_contribution),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::one(),
            acceptable_price_spread: Decimal::from_str("0.1").unwrap(),
        },
        remote_chain_splitter_config: RemoteChainSplitterConfig {
            channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            denom: atom_denom.clone(),
            amount: Uint128::from(uatom_contribution_amount),
            ls_share: Decimal::percent(50),
            native_share: Decimal::percent(50),
            fallback_address: None,
        },
        emergency_committee: None,
        covenant_party_config: InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            ibc_transfer_timeout: Uint64::new(300),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN_NAME)
                .dest(NEUTRON_CHAIN_NAME)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
            remote_chain_denom: atom_denom.clone(),
            addr: ACC1_ADDRESS_NEUTRON.to_string(),
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        },
        // we're importing the old liquid pooler config here because the new astroport release
        // is breaking the types
        liquid_pooler_config: LiquidPoolerConfig::Astroport(AstroportLiquidPoolerConfig {
            pool_pair_type: astroport_old::factory::PairType::Stable {},
            pool_address: pool_addr.to_string(),
            asset_a_denom: statom_on_neutron.clone(),
            asset_b_denom: atom_on_neutron.clone(),
            single_side_lp_limits: SingleSideLpLimits {
                asset_a_limit: Uint128::new(1000000),
                asset_b_limit: Uint128::new(1000000),
            },
        }),
        operation_mode: covenant_utils::op_mode::ContractOperationModeConfig::Permissioned(vec![]),
    };

    let covenant_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_single_party_pol_covenant,
        &serde_json::to_string(&instantiate_covenant_msg).unwrap(),
        "single-party-pol-stride-covenant",
        None,
        "",
    )?;
    info!("Covenant contract: {:?}", covenant_contract.address);

    let covenant = Covenant::SinglePartyPol {
        rb: test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        contract_address: &covenant_contract.address,
    };

    // Query the covenant addresses
    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let liquid_pooler_address = covenant.query_liquid_pooler_address();
    let liquid_staker_address = covenant.query_liquid_staker_address();
    let ls_forwarder_address = covenant.query_ibc_forwarder_address("ls".to_string());
    let liquid_pooler_forwarder_address = covenant.query_ibc_forwarder_address("lp".to_string());
    let remote_chain_splitter_address = covenant.query_splitter_address();
    let interchain_router_address = covenant.query_interchain_router_address("".to_string());

    info!("Fund covenant addresses with NTRN...");
    let addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        liquid_pooler_address.clone(),
        liquid_staker_address.clone(),
        ls_forwarder_address.clone(),
        liquid_pooler_forwarder_address.clone(),
        remote_chain_splitter_address.clone(),
        interchain_router_address.clone(),
    ];

    for address in &addresses {
        send(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            address,
            &[Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(5000000000),
            }],
            &Coin {
                denom: neutron_denom.clone(),
                amount: Uint128::new(5000),
            },
        )
        .unwrap();
    }

    info!("Tick until forwarders create ICA...");
    let party_deposit_address;
    let stride_ica_address;
    let ls_forwarder_ica_address;
    let liquid_pooler_forwarder_ica_address;
    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );
        let ls_forwarder_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &ls_forwarder_address,
        );
        info!("Liquid Staker forwarder state: {:?}", ls_forwarder_state);

        let liquid_pooler_forwarder_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_forwarder_address,
        );
        info!(
            "Liquid Pooler forwarder state: {:?}",
            liquid_pooler_forwarder_state
        );

        let splitter_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &remote_chain_splitter_address,
        );
        info!("Remote Chain Splitter state: {:?}", splitter_state);

        let liquid_staker_state = query_contract_state(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_staker_address,
        );
        info!("Liquid Staker state: {:?}", liquid_staker_state);

        if splitter_state == "ica_created"
            && liquid_staker_state == "ica_created"
            && ls_forwarder_state == "ica_created"
            && liquid_pooler_forwarder_state == "ica_created"
        {
            party_deposit_address = covenant.query_deposit_address("".to_string());

            let query_response = contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &liquid_staker_address,
                &serde_json::to_string(&valence_stride_liquid_staker::msg::QueryMsg::IcaAddress {})
                    .unwrap(),
            );
            stride_ica_address = query_response["data"].as_str().unwrap().to_string();

            let query_response = contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &ls_forwarder_address,
                &serde_json::to_string(&valence_ibc_forwarder::msg::QueryMsg::DepositAddress {})
                    .unwrap(),
            );
            ls_forwarder_ica_address = query_response["data"].as_str().unwrap().to_string();

            let query_response = contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &liquid_pooler_forwarder_address,
                &serde_json::to_string(
                    &valence_astroport_liquid_pooler::msg::QueryMsg::DepositAddress {},
                )
                .unwrap(),
            );
            liquid_pooler_forwarder_ica_address =
                query_response["data"].as_str().unwrap().to_string();

            info!("LS forwarder ICA address: {}", ls_forwarder_ica_address);
            info!(
                "Liquid Pooler forwarder ICA address: {}",
                liquid_pooler_forwarder_ica_address
            );
            break;
        }
    }

    info!("Fund the forwarder with sufficient funds...");
    send(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN_NAME),
        DEFAULT_KEY,
        &party_deposit_address,
        &[Coin {
            denom: atom_denom.clone(),
            amount: Uint128::new(uatom_contribution_amount),
        }],
        &Coin {
            denom: atom_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!("Tick until splitter splits the funds to ls and lp forwarders...");
    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );

        let ls_forwarder_ica_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(GAIA_CHAIN_NAME),
            &ls_forwarder_ica_address,
        );

        let liquid_pooler_forwarder_ica_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(GAIA_CHAIN_NAME),
            &liquid_pooler_forwarder_ica_address,
        );

        let party_deposit_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(GAIA_CHAIN_NAME),
            &party_deposit_address,
        );

        info!("LS forwarder ICA balance: {:?}", ls_forwarder_ica_balance);
        info!(
            "Liquid Pooler forwarder ICA balance: {:?}",
            liquid_pooler_forwarder_ica_balance
        );
        info!("Party deposit balance: {:?}", party_deposit_balance);

        if ls_forwarder_ica_balance
            .iter()
            .any(|c| c.denom == atom_denom.clone())
            && liquid_pooler_forwarder_ica_balance
                .iter()
                .any(|c| c.denom == atom_denom.clone())
        {
            info!("Liquid staker forwarder and liquid pooler forwarder received ATOM & StATOM");
            break;
        }
    }

    info!("Tick until liquid staker stakes...");
    let mut stride_ica_statom_balance;
    loop {
        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );

        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );

        let stride_ica_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(STRIDE_CHAIN_NAME),
            &stride_ica_address,
        );

        info!("Liquid Pooler balance: {:?}", liquid_pooler_balance);
        info!("Stride ICA balance: {:?}", stride_ica_balance);
        stride_ica_statom_balance = match stride_ica_balance
            .iter()
            .find(|c| c.denom == NATIVE_STATOM_DENOM)
        {
            Some(c) => c.amount,
            None => Uint128::zero(),
        };

        if !stride_ica_statom_balance.is_zero() {
            info!("Stride ICA received StATOM");
            break;
        }
    }

    info!("Permissionless forward...");
    loop {
        thread::sleep(Duration::from_secs(5));
        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_staker_address,
            DEFAULT_KEY,
            &serde_json::to_string(&valence_stride_liquid_staker::msg::ExecuteMsg::Transfer {
                amount: stride_ica_statom_balance,
            })
            .unwrap(),
            EXECUTE_FLAGS,
        )
        .unwrap();

        let stride_ica_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(STRIDE_CHAIN_NAME),
            &stride_ica_address,
        );

        info!("Stride ICA balance: {:?}", stride_ica_balance);

        let statom_balance = match stride_ica_balance
            .iter()
            .find(|c| c.denom == NATIVE_STATOM_DENOM)
        {
            Some(c) => c.amount,
            None => Uint128::zero(),
        };

        if statom_balance.is_zero() {
            let liquid_pooler_balance = get_balance(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &liquid_pooler_address,
            );
            info!("Liquid Pooler balance: {:?}", liquid_pooler_balance);
            break;
        }
    }

    info!("Tick until liquid pooler provides liquidity...");
    loop {
        // TODO: change to native balance query of the tokenfactory token
        let liquid_pooler_lp_balance = get_lp_token_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &lp_token_address,
            &liquid_pooler_address,
        );
        info!(
            "Liquid pooler LP token balance: {}",
            liquid_pooler_lp_balance
        );

        let holder_lp_balance = get_lp_token_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &lp_token_address,
            &holder_address,
        );
        info!("Holder LP token balance: {}", holder_lp_balance);

        let neutron_user_lp_balance = get_lp_token_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &lp_token_address,
            ACC2_ADDRESS_NEUTRON,
        );
        info!("Neutron User LP token balance: {}", neutron_user_lp_balance);

        if liquid_pooler_lp_balance != "0" {
            break;
        }

        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );
    }

    info!("User redeems LP tokens for underlying liquidity...");
    let covenant_party_balance = get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN_NAME),
        ACC1_ADDRESS_GAIA,
    );
    info!("Covenant party balance: {:?}", covenant_party_balance);

    thread::sleep(Duration::from_secs(10));
    loop {
        match contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
            ACC_1_KEY,
            &serde_json::to_string(&valence_single_party_pol_holder::msg::ExecuteMsg::Claim {})
                .unwrap(),
            EXECUTE_FLAGS,
        ) {
            Ok(_) => break,
            Err(_) => {
                info!("Waiting for lock up period to be over...");
                thread::sleep(Duration::from_secs(10));
                continue;
            }
        }
    }

    loop {
        let hub_user_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(GAIA_CHAIN_NAME),
            &ACC1_ADDRESS_GAIA,
        );

        let liquid_pooler_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &liquid_pooler_address,
        );

        let holder_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &holder_address,
        );

        let interchain_router_balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &interchain_router_address,
        );

        info!("Hub user balance: {:?}", hub_user_balance);
        info!("Liquid Pooler balance: {:?}", liquid_pooler_balance);
        info!("Holder balance: {:?}", holder_balance);
        info!("Interchain router balance: {:?}", interchain_router_balance);

        if hub_user_balance.iter().any(|c| c.denom == atom_denom)
            && hub_user_balance
                .iter()
                .any(|c| c.denom == statom_on_gaia_via_neutron)
        {
            info!("Covenant party received the funds!");
            break;
        }

        tick(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &clock_address,
        );
    }

    // Send the balances back so we have a fresh start for the next test
    send_non_native_balances(
        test_ctx,
        GAIA_CHAIN_NAME,
        ACC_1_KEY,
        ACC1_ADDRESS_GAIA,
        NEUTRON_CHAIN_ADMIN_ADDR,
        &atom_denom,
    );

    info!("Finished single party POL stride tests!");

    Ok(())
}
