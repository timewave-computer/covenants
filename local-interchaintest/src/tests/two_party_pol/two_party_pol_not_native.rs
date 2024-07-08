use std::{collections::BTreeMap, str::FromStr, thread, time::Duration};

use astroport::{
    asset::{Asset, AssetInfo},
    factory::{InstantiateMsg as FactoryInstantiateMsg, PairConfig, PairType},
    native_coin_registry::{
        ExecuteMsg as NativeCoinRegistryExecuteMsg,
        InstantiateMsg as NativeCoinRegistryInstantiateMsg,
    },
    pair::StablePoolParams,
};
use cosmwasm_std::{coin, Binary, Coin, Decimal, Uint128, Uint64};
use covenant_utils::{
    op_mode::ContractOperationModeConfig, split::SplitConfig, InterchainCovenantParty,
    PoolPriceConfig, SingleSideLpLimits,
};
use cw_utils::Expiration;
use localic_std::{
    errors::LocalError,
    modules::{
        bank::{get_balance, send},
        cosmwasm::{contract_execute, contract_instantiate},
    },
    node::Chain,
};
use log::info;
use valence_astroport_liquid_pooler::msg::AstroportLiquidPoolerConfig;
use valence_covenant_two_party_pol::msg::{CovenantContractCodeIds, CovenantPartyConfig, Timeouts};
use valence_two_party_pol_holder::msg::{CovenantType, RagequitConfig, RagequitTerms};

use crate::{
    helpers::{
        astroport::{get_lp_token_address, get_lp_token_balance, get_pool_address},
        common::{query_contract_state, tick},
        covenant::Covenant,
    },
    utils::{
        constants::{
            ACC1_ADDRESS_GAIA, ACC1_ADDRESS_NEUTRON, ACC2_ADDRESS_NEUTRON, ACC2_ADDRESS_OSMO,
            ACC_0_KEY, ACC_1_KEY, ACC_2_KEY, ASTROPORT_PATH, EXECUTE_FLAGS, GAIA_CHAIN,
            NEUTRON_CHAIN, OSMOSIS_CHAIN, VALENCE_PATH,
        },
        ibc::{get_multihop_ibc_denom, ibc_send},
        setup::deploy_contracts_on_chain,
        test_context::TestContext,
    },
};

pub fn test_two_party_pol(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    deploy_contracts_on_chain(test_ctx, VALENCE_PATH, NEUTRON_CHAIN);
    deploy_contracts_on_chain(test_ctx, ASTROPORT_PATH, NEUTRON_CHAIN);

    info!("Starting two party POL tests...");
    let astroport_native_coin_registry_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_native_coin_registry")
        .unwrap();

    let astroport_pair_stable_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_pair_stable")
        .unwrap();

    let astroport_token_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_token")
        .unwrap();

    let astroport_whitelist_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_whitelist")
        .unwrap();

    let astroport_factory_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_factory")
        .unwrap();

    let neutron_request_builder = test_ctx
        .get_request_builder()
        .get_request_builder(NEUTRON_CHAIN);
    let gaia_request_builder = test_ctx
        .get_request_builder()
        .get_request_builder(GAIA_CHAIN);
    let osmosis_request_builder = test_ctx
        .get_request_builder()
        .get_request_builder(OSMOSIS_CHAIN);

    let neutron_admin_acc = test_ctx.get_admin_addr().src(NEUTRON_CHAIN).get();

    let native_coin_registry_instantiate_msg = NativeCoinRegistryInstantiateMsg {
        owner: neutron_admin_acc.clone(),
    };
    let native_coin_registry_contract = contract_instantiate(
        neutron_request_builder,
        ACC_0_KEY,
        astroport_native_coin_registry_code_id,
        &serde_json::to_string(&native_coin_registry_instantiate_msg).unwrap(),
        "native-coin-registry",
        None,
        "",
    )?;
    info!(
        "Native coin registry contract: {:?}",
        native_coin_registry_contract.address
    );

    let atom_denom = test_ctx.get_native_denom().src(GAIA_CHAIN).get();
    let neutron_denom = test_ctx.get_native_denom().src(NEUTRON_CHAIN).get();
    let osmo_denom = test_ctx.get_native_denom().src(OSMOSIS_CHAIN).get();
    let atom_on_neutron = test_ctx
        .get_ibc_denoms()
        .src(GAIA_CHAIN)
        .dest(NEUTRON_CHAIN)
        .get();
    let osmo_on_neutron = test_ctx
        .get_ibc_denoms()
        .src(OSMOSIS_CHAIN)
        .dest(NEUTRON_CHAIN)
        .get();
    let atom_on_osmo_via_neutron = get_multihop_ibc_denom(
        &atom_denom,
        vec![
            &test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            &test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
        ],
    );
    let osmo_on_gaia_via_neutron = get_multihop_ibc_denom(
        &osmo_denom,
        vec![
            &test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            &test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
        ],
    );

    let add_to_registry_msg = NativeCoinRegistryExecuteMsg::Add {
        native_coins: vec![(atom_on_neutron.clone(), 6), (osmo_on_neutron.clone(), 6)],
    };
    contract_execute(
        neutron_request_builder,
        &native_coin_registry_contract.address,
        ACC_0_KEY,
        &serde_json::to_string(&add_to_registry_msg).unwrap(),
        EXECUTE_FLAGS,
    )?;
    thread::sleep(Duration::from_secs(3));

    let factory_instantiate_msg = FactoryInstantiateMsg {
        pair_configs: vec![PairConfig {
            code_id: astroport_pair_stable_code_id,
            pair_type: PairType::Stable {},
            total_fee_bps: 0,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: true,
        }],
        token_code_id: astroport_token_code_id,
        fee_address: None,
        generator_address: None,
        owner: neutron_admin_acc.clone(),
        whitelist_code_id: astroport_whitelist_code_id,
        coin_registry_address: native_coin_registry_contract.address.to_string(),
    };
    let factory_contract = contract_instantiate(
        neutron_request_builder,
        ACC_0_KEY,
        astroport_factory_code_id,
        &serde_json::to_string(&factory_instantiate_msg).unwrap(),
        "astroport-factory",
        None,
        "",
    )?;
    info!("Factory contract: {:?}", factory_contract.address);

    let create_pair_msg = astroport::factory::ExecuteMsg::CreatePair {
        pair_type: PairType::Stable {},
        asset_infos: vec![
            AssetInfo::NativeToken {
                denom: atom_on_neutron.clone(),
            },
            AssetInfo::NativeToken {
                denom: osmo_on_neutron.clone(),
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
        neutron_request_builder,
        &factory_contract.address,
        ACC_0_KEY,
        &serde_json::to_string(&create_pair_msg).unwrap(),
        EXECUTE_FLAGS,
    )?;

    // Send some ATOM and OSMO to NTRN
    let uatom_contribution_amount = 500_000_000;
    loop {
        ibc_send(
            gaia_request_builder,
            ACC_0_KEY,
            &neutron_admin_acc,
            coin(uatom_contribution_amount, atom_denom.clone()),
            coin(100000, atom_denom.clone()),
            &test_ctx
                .get_transfer_channels()
                .src(GAIA_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
            None,
        )?;
        info!("Waiting to receive ATOM IBC transfer...");
        thread::sleep(Duration::from_secs(5));
        let balance = get_balance(neutron_request_builder, &neutron_admin_acc);
        if balance.iter().any(|c| {
            c.denom == atom_on_neutron && c.amount >= Uint128::new(uatom_contribution_amount)
        }) {
            break;
        }
    }

    let uosmo_contribution_amount = 5_000_000_000;
    loop {
        ibc_send(
            osmosis_request_builder,
            ACC_0_KEY,
            &neutron_admin_acc,
            coin(uosmo_contribution_amount, osmo_denom.clone()),
            coin(100000, osmo_denom.clone()),
            &test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
            None,
        )?;
        info!("Waiting to receive OSMO IBC transfer...");
        thread::sleep(Duration::from_secs(5));
        let balance = get_balance(neutron_request_builder, &neutron_admin_acc);
        if balance.iter().any(|c| {
            c.denom == osmo_on_neutron && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            break;
        }
    }

    // Provide the ATOM/OSMO liquidity to the pair
    let pool_addr = get_pool_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron.clone(),
        },
        AssetInfo::NativeToken {
            denom: osmo_on_neutron.clone(),
        },
    );

    let provide_liquidity_msg = astroport::pair::ExecuteMsg::ProvideLiquidity {
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: atom_on_neutron.clone(),
                },
                amount: Uint128::from(uatom_contribution_amount),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: osmo_on_neutron.clone(),
                },
                amount: Uint128::from(uosmo_contribution_amount),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        auto_stake: Some(false),
        receiver: Some(neutron_admin_acc.clone()),
    };

    contract_execute(
        neutron_request_builder,
        &pool_addr,
        ACC_0_KEY,
        &serde_json::to_string(&provide_liquidity_msg).unwrap(),
        &format!("--amount {uatom_contribution_amount}{atom_on_neutron},{uosmo_contribution_amount}{osmo_on_neutron} {EXECUTE_FLAGS}"),
    ).unwrap();
    thread::sleep(Duration::from_secs(3));

    let valence_ibc_forwarder_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_ibc_forwarder")
        .unwrap();

    let valence_two_party_pol_holder_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_two_party_pol_holder")
        .unwrap();

    let valence_clock_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_clock")
        .unwrap();

    let valence_interchain_router_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_interchain_router")
        .unwrap();

    let valence_native_router_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_native_router")
        .unwrap();

    let valence_liquid_pooler_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_astroport_liquid_pooler")
        .unwrap();

    let valence_covenant_two_party_pol_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("valence_covenant_two_party_pol")
        .unwrap();

    let chain = Chain::new(neutron_request_builder);
    let current_block_height = chain.get_height();

    // Instantiate the covenants
    let target = "Two party POL happy path";
    info!(target: target,"Starting Two party POL happy path test...");

    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "two-party-pol-covenant-happy".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(100),          // seconds
            ibc_transfer_timeout: Uint64::new(100), // seconds
        },
        contract_codes: CovenantContractCodeIds {
            ibc_forwarder_code: valence_ibc_forwarder_code_id,
            holder_code: valence_two_party_pol_holder_code_id,
            clock_code: valence_clock_code_id,
            interchain_router_code: valence_interchain_router_code_id,
            native_router_code: valence_native_router_code_id,
            liquid_pooler_code: valence_liquid_pooler_code_id,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 200),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
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
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_OSMO.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            remote_chain_denom: osmo_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: osmo_on_neutron.clone(),
            contribution: Coin {
                denom: osmo_denom.clone(),
                amount: Uint128::new(uosmo_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        covenant_type: CovenantType::Share,
        ragequit_config: Some(RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.1").unwrap(),
            state: None,
        })),
        deposit_deadline: Expiration::AtHeight(current_block_height + 180),
        party_a_share: Decimal::percent(50),
        party_b_share: Decimal::percent(50),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.1").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.09").unwrap(),
        },
        splits: BTreeMap::from([
            (
                atom_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
            (
                osmo_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
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
                asset_a_denom: atom_on_neutron.clone(),
                asset_b_denom: osmo_on_neutron.clone(),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(100000),
                    asset_b_limit: Uint128::new(1000000),
                },
            },
        ),
        fallback_address: None,
        operation_mode: ContractOperationModeConfig::Permissioned(vec![]),
    };

    let covenant_contract = contract_instantiate(
        neutron_request_builder,
        ACC_0_KEY,
        valence_covenant_two_party_pol_code_id,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "two-party-pol-covenant-happy-path",
        None,
        "",
    )?;
    info!(target: target,"Covenant contract: {:?}", covenant_contract.address);
    let covenant = Covenant::TwoPartyPol {
        rb: neutron_request_builder,
        contract_address: &covenant_contract.address,
    };

    // Query the covenant addresses
    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let liquid_pooler_address = covenant.query_liquid_pooler_address();
    let party_a_router_address = covenant.query_interchain_router_address("party_a".to_string());
    let party_b_router_address = covenant.query_interchain_router_address("party_b".to_string());
    let party_a_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_a".to_string());
    let party_b_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_b".to_string());

    info!(target: target,"Fund covenant addresses with NTRN...");
    let mut addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        liquid_pooler_address.clone(),
        party_a_router_address.clone(),
        party_b_router_address.clone(),
    ];
    if !party_a_ibc_forwarder_address.is_empty() {
        addresses.push(party_a_ibc_forwarder_address.clone());
    }
    if !party_b_ibc_forwarder_address.is_empty() {
        addresses.push(party_b_ibc_forwarder_address.clone());
    }
    for address in &addresses {
        send(
            neutron_request_builder,
            ACC_0_KEY,
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

    info!(target: target,"Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        let forwarder_b_state =
            query_contract_state(neutron_request_builder, &party_b_ibc_forwarder_address);
        info!(target: target,"Forwarder A state: {:?}", forwarder_a_state);
        info!(target: target,"Forwarder B state: {:?}", forwarder_b_state);
        if forwarder_a_state == "ica_created" && forwarder_b_state == "ica_created" {
            party_a_deposit_address = covenant.query_deposit_address("party_a".to_string());
            party_b_deposit_address = covenant.query_deposit_address("party_b".to_string());
            break;
        }
    }
    info!(target: target,"Party A deposit address: {}", party_a_deposit_address);
    info!(target: target,"Party B deposit address: {}", party_b_deposit_address);

    info!(target: target,"Fund the forwarders with sufficient funds...");
    send(
        gaia_request_builder,
        ACC_0_KEY,
        &party_a_deposit_address,
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
    send(
        osmosis_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(uosmo_contribution_amount),
        }],
        &Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!(target: target,"Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            info!(target: target,"Holder received ATOM & OSMO");
            break;
        } else if holder_state == "active" {
            info!(target: target,"Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!(target: target,"Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron.clone(),
        },
        AssetInfo::NativeToken {
            denom: osmo_on_neutron.clone(),
        },
    );

    loop {
        let balance = get_lp_token_balance(
            neutron_request_builder,
            &lp_token_address,
            &liquid_pooler_address,
        );

        if balance == "0" {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        } else {
            break;
        }
    }

    info!(target: target,"Tick until holder expires...");
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        info!(target: target,"Holder state: {:?}", holder_state);
        if holder_state == "expired" {
            break;
        }
    }

    info!(target: target,"Party A claims and router receives the funds");
    let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
    info!(target: target,"Router A balances: {:?}", router_a_balances);

    thread::sleep(Duration::from_secs(10));
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_1_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
    info!(target: target,"Router A balances: {:?}", router_a_balances);

    info!(target: target,"Tick until party A claim is distributed");
    info!(target: target,"Hub receiver address: {}", ACC1_ADDRESS_GAIA);
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!(target: target,"Hub receiver balances: {:?}", hub_receiver_balances);
        if hub_receiver_balances
            .iter()
            .any(|c| c.denom == atom_denom.clone())
            && hub_receiver_balances
                .iter()
                .any(|c| c.denom == osmo_on_gaia_via_neutron.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!(target: target,"Party B claims and router receives the funds");
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!(target: target,"Router B balances: {:?}", router_b_balances);
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_2_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!(target: target,"Router B balances: {:?}", router_b_balances);

    info!(target: target,"Tick until both parties receive their funds");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!(target: target,"Hub receiver balances: {:?}", hub_receiver_balances);
        let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
        info!(target: target,"Osmosis receiver balances: {:?}", osmo_receiver_balances);
        if osmo_receiver_balances
            .iter()
            .any(|c| c.denom == osmo_denom.clone())
            && osmo_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_osmo_via_neutron.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    // Send the balances back so we have a fresh start for the next test
    let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
    for coin in hub_receiver_balances {
        if coin.denom != atom_denom.clone() {
            ibc_send(
                gaia_request_builder,
                ACC_1_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: atom_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(GAIA_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }
    let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
    for coin in osmo_receiver_balances {
        if coin.denom != osmo_denom.clone() {
            ibc_send(
                osmosis_request_builder,
                ACC_2_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: osmo_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(OSMOSIS_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    let current_block_height = chain.get_height();

    let target = "Two party share based POL ragequit path";
    info!(target: target,"Starting Two party share based POL ragequit test...");

    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "two-party-pol-covenant-ragequit".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(100),          // seconds
            ibc_transfer_timeout: Uint64::new(100), // seconds
        },
        contract_codes: CovenantContractCodeIds {
            ibc_forwarder_code: valence_ibc_forwarder_code_id,
            holder_code: valence_two_party_pol_holder_code_id,
            clock_code: valence_clock_code_id,
            interchain_router_code: valence_interchain_router_code_id,
            native_router_code: valence_native_router_code_id,
            liquid_pooler_code: valence_liquid_pooler_code_id,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 300),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
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
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_OSMO.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            remote_chain_denom: osmo_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: osmo_on_neutron.clone(),
            contribution: Coin {
                denom: osmo_denom.clone(),
                amount: Uint128::new(uosmo_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        covenant_type: CovenantType::Share,
        ragequit_config: Some(RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.1").unwrap(),
            state: None,
        })),
        deposit_deadline: Expiration::AtHeight(current_block_height + 200),
        party_a_share: Decimal::percent(50),
        party_b_share: Decimal::percent(50),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.1").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.09").unwrap(),
        },
        splits: BTreeMap::from([
            (
                atom_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
                    ]),
                },
            ),
            (
                osmo_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(50)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(50)),
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
                asset_a_denom: atom_on_neutron.clone(),
                asset_b_denom: osmo_on_neutron.clone(),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(100000),
                    asset_b_limit: Uint128::new(100000),
                },
            },
        ),
        fallback_address: None,
        operation_mode: ContractOperationModeConfig::Permissioned(vec![]),
    };

    let covenant_contract = contract_instantiate(
        neutron_request_builder,
        ACC_0_KEY,
        valence_covenant_two_party_pol_code_id,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "two-party-pol-covenant-ragequit",
        None,
        "",
    )?;
    info!(target: target,"Covenant contract: {:?}", covenant_contract.address);
    let covenant = Covenant::TwoPartyPol {
        rb: neutron_request_builder,
        contract_address: &covenant_contract.address,
    };

    // Query the covenant addresses
    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let liquid_pooler_address = covenant.query_liquid_pooler_address();
    let party_a_router_address = covenant.query_interchain_router_address("party_a".to_string());
    let party_b_router_address = covenant.query_interchain_router_address("party_b".to_string());
    let party_a_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_a".to_string());
    let party_b_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_b".to_string());

    info!(target: target,"Fund covenant addresses with NTRN...");
    let mut addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        liquid_pooler_address.clone(),
        party_a_router_address.clone(),
        party_b_router_address.clone(),
    ];
    if !party_a_ibc_forwarder_address.is_empty() {
        addresses.push(party_a_ibc_forwarder_address.clone());
    }
    if !party_b_ibc_forwarder_address.is_empty() {
        addresses.push(party_b_ibc_forwarder_address.clone());
    }
    for address in &addresses {
        send(
            neutron_request_builder,
            ACC_0_KEY,
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

    info!(target: target,"Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        let forwarder_b_state =
            query_contract_state(neutron_request_builder, &party_b_ibc_forwarder_address);
        info!(target: target,"Forwarder A state: {:?}", forwarder_a_state);
        info!(target: target,"Forwarder B state: {:?}", forwarder_b_state);
        if forwarder_a_state == "ica_created" && forwarder_b_state == "ica_created" {
            party_a_deposit_address = covenant.query_deposit_address("party_a".to_string());
            party_b_deposit_address = covenant.query_deposit_address("party_b".to_string());
            break;
        }
    }
    info!(target: target,"Party A deposit address: {}", party_a_deposit_address);
    info!(target: target,"Party B deposit address: {}", party_b_deposit_address);

    info!(target: target,"Fund the forwarders with sufficient funds...");
    send(
        gaia_request_builder,
        ACC_0_KEY,
        &party_a_deposit_address,
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
    send(
        osmosis_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(uosmo_contribution_amount),
        }],
        &Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!(target: target,"Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            info!(target: target,"Holder received ATOM & OSMO");
            break;
        } else if holder_state == "active" {
            info!(target: target,"Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!(target: target,"Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron.clone(),
        },
        AssetInfo::NativeToken {
            denom: osmo_on_neutron.clone(),
        },
    );

    loop {
        let balance = get_lp_token_balance(
            neutron_request_builder,
            &lp_token_address,
            &liquid_pooler_address,
        );

        if balance == "0" {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        } else {
            break;
        }
    }

    info!(target: target,"Party A ragequits...");
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_1_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Ragequit {})
            .unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();

    info!(target: target,"Party B claims and router receives the funds");
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!(target: target,"Router B balances: {:?}", router_b_balances);
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_2_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!(target: target,"Router B balances: {:?}", router_b_balances);

    info!(target: target,"Tick until both parties receive their funds");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!(target: target,"Hub receiver balances: {:?}", hub_receiver_balances);
        let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
        info!(target: target,"Osmosis receiver balances: {:?}", osmo_receiver_balances);
        if hub_receiver_balances
            .iter()
            .any(|c| c.denom == osmo_on_gaia_via_neutron.clone())
            && osmo_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_osmo_via_neutron.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    // Send the balances back so we have a fresh start for the next test
    let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
    for coin in hub_receiver_balances {
        if coin.denom != atom_denom.clone() {
            ibc_send(
                gaia_request_builder,
                ACC_1_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: atom_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(GAIA_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }
    let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
    for coin in osmo_receiver_balances {
        if coin.denom != osmo_denom.clone() {
            ibc_send(
                osmosis_request_builder,
                ACC_2_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: osmo_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(OSMOSIS_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    let current_block_height = chain.get_height();

    let target = "Two party side based POL ragequit path";
    info!(target: target,"Starting Two party side based POL ragequit test...");

    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "two-party-pol-covenant-side-ragequit".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(100),          // seconds
            ibc_transfer_timeout: Uint64::new(100), // seconds
        },
        contract_codes: CovenantContractCodeIds {
            ibc_forwarder_code: valence_ibc_forwarder_code_id,
            holder_code: valence_two_party_pol_holder_code_id,
            clock_code: valence_clock_code_id,
            interchain_router_code: valence_interchain_router_code_id,
            native_router_code: valence_native_router_code_id,
            liquid_pooler_code: valence_liquid_pooler_code_id,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 300),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
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
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_OSMO.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            remote_chain_denom: osmo_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: osmo_on_neutron.clone(),
            contribution: Coin {
                denom: osmo_denom.clone(),
                amount: Uint128::new(uosmo_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        covenant_type: CovenantType::Side,
        ragequit_config: Some(RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.1").unwrap(),
            state: None,
        })),
        deposit_deadline: Expiration::AtHeight(current_block_height + 200),
        party_a_share: Decimal::percent(50),
        party_b_share: Decimal::percent(50),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.1").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.09").unwrap(),
        },
        splits: BTreeMap::from([
            (
                atom_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(100)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(0)),
                    ]),
                },
            ),
            (
                osmo_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(0)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(100)),
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
                asset_a_denom: atom_on_neutron.clone(),
                asset_b_denom: osmo_on_neutron.clone(),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(1000000),
                    asset_b_limit: Uint128::new(1000000),
                },
            },
        ),
        fallback_address: None,
        operation_mode: ContractOperationModeConfig::Permissioned(vec![]),
    };

    let covenant_contract = contract_instantiate(
        neutron_request_builder,
        ACC_0_KEY,
        valence_covenant_two_party_pol_code_id,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "two-party-pol-covenant-side-ragequit",
        None,
        "",
    )?;
    info!(target: target,"Covenant contract: {:?}", covenant_contract.address);
    let covenant = Covenant::TwoPartyPol {
        rb: neutron_request_builder,
        contract_address: &covenant_contract.address,
    };

    // Query the covenant addresses
    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let liquid_pooler_address = covenant.query_liquid_pooler_address();
    let party_a_router_address = covenant.query_interchain_router_address("party_a".to_string());
    let party_b_router_address = covenant.query_interchain_router_address("party_b".to_string());
    let party_a_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_a".to_string());
    let party_b_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_b".to_string());

    info!(target: target,"Fund covenant addresses with NTRN...");
    let mut addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        liquid_pooler_address.clone(),
        party_a_router_address.clone(),
        party_b_router_address.clone(),
    ];
    if !party_a_ibc_forwarder_address.is_empty() {
        addresses.push(party_a_ibc_forwarder_address.clone());
    }
    if !party_b_ibc_forwarder_address.is_empty() {
        addresses.push(party_b_ibc_forwarder_address.clone());
    }
    for address in &addresses {
        send(
            neutron_request_builder,
            ACC_0_KEY,
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

    info!(target: target,"Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        let forwarder_b_state =
            query_contract_state(neutron_request_builder, &party_b_ibc_forwarder_address);
        info!(target: target,"Forwarder A state: {:?}", forwarder_a_state);
        info!(target: target,"Forwarder B state: {:?}", forwarder_b_state);
        if forwarder_a_state == "ica_created" && forwarder_b_state == "ica_created" {
            party_a_deposit_address = covenant.query_deposit_address("party_a".to_string());
            party_b_deposit_address = covenant.query_deposit_address("party_b".to_string());
            break;
        }
    }
    info!(target: target,"Party A deposit address: {}", party_a_deposit_address);
    info!(target: target,"Party B deposit address: {}", party_b_deposit_address);

    info!(target: target,"Fund the forwarders with sufficient funds...");
    send(
        gaia_request_builder,
        ACC_0_KEY,
        &party_a_deposit_address,
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
    send(
        osmosis_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(uosmo_contribution_amount),
        }],
        &Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!(target: target,"Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            info!(target: target,"Holder received ATOM & OSMO");
            break;
        } else if holder_state == "active" {
            info!(target: target,"Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!(target: target,"Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron.clone(),
        },
        AssetInfo::NativeToken {
            denom: osmo_on_neutron.clone(),
        },
    );

    loop {
        let balance = get_lp_token_balance(
            neutron_request_builder,
            &lp_token_address,
            &liquid_pooler_address,
        );

        if balance == "0" {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        } else {
            break;
        }
    }

    let previous_balance = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
    info!(target: target,"Party A ragequits...");
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_1_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Ragequit {})
            .unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();

    info!(target: target,"Tick until both parties receive their funds");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!(target: target,"Hub receiver balances: {:?}", hub_receiver_balances);
        let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
        info!(target: target,"Osmosis receiver balances: {:?}", osmo_receiver_balances);
        if previous_balance != hub_receiver_balances
            && osmo_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_osmo_via_neutron.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    // Send the balances back so we have a fresh start for the next test
    let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
    for coin in hub_receiver_balances {
        if coin.denom != atom_denom.clone() {
            ibc_send(
                gaia_request_builder,
                ACC_1_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: atom_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(GAIA_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }
    let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
    for coin in osmo_receiver_balances {
        if coin.denom != osmo_denom.clone() {
            ibc_send(
                osmosis_request_builder,
                ACC_2_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: osmo_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(OSMOSIS_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    let current_block_height = chain.get_height();

    let target = "Two party POL side based happy path";
    info!(target: target,"Starting Two party POL side based happy path test...");

    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "two-party-pol-covenant-side-happy".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(100),          // seconds
            ibc_transfer_timeout: Uint64::new(100), // seconds
        },
        contract_codes: CovenantContractCodeIds {
            ibc_forwarder_code: valence_ibc_forwarder_code_id,
            holder_code: valence_two_party_pol_holder_code_id,
            clock_code: valence_clock_code_id,
            interchain_router_code: valence_interchain_router_code_id,
            native_router_code: valence_native_router_code_id,
            liquid_pooler_code: valence_liquid_pooler_code_id,
        },
        clock_tick_max_gas: None,
        lockup_config: Expiration::AtHeight(current_block_height + 230),
        party_a_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC1_ADDRESS_GAIA.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
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
            native_denom: atom_on_neutron.clone(),
            contribution: Coin {
                denom: atom_denom.clone(),
                amount: Uint128::new(uatom_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        party_b_config: CovenantPartyConfig::Interchain(InterchainCovenantParty {
            party_receiver_addr: ACC2_ADDRESS_OSMO.to_string(),
            party_chain_connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            ibc_transfer_timeout: Uint64::new(100),
            party_to_host_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(OSMOSIS_CHAIN)
                .dest(NEUTRON_CHAIN)
                .get(),
            host_to_party_chain_channel_id: test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN)
                .dest(OSMOSIS_CHAIN)
                .get(),
            remote_chain_denom: osmo_denom.clone(),
            addr: ACC2_ADDRESS_NEUTRON.to_string(),
            native_denom: osmo_on_neutron.clone(),
            contribution: Coin {
                denom: osmo_denom.clone(),
                amount: Uint128::new(uosmo_contribution_amount),
            },
            denom_to_pfm_map: BTreeMap::new(),
            fallback_address: None,
        }),
        covenant_type: CovenantType::Side,
        ragequit_config: Some(RagequitConfig::Enabled(RagequitTerms {
            penalty: Decimal::from_str("0.1").unwrap(),
            state: None,
        })),
        deposit_deadline: Expiration::AtHeight(current_block_height + 210),
        party_a_share: Decimal::percent(50),
        party_b_share: Decimal::percent(50),
        pool_price_config: PoolPriceConfig {
            expected_spot_price: Decimal::from_str("0.1").unwrap(),
            acceptable_price_spread: Decimal::from_str("0.09").unwrap(),
        },
        splits: BTreeMap::from([
            (
                atom_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(100)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(0)),
                    ]),
                },
            ),
            (
                osmo_on_neutron.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(0)),
                        (ACC2_ADDRESS_OSMO.to_string(), Decimal::percent(100)),
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
                asset_a_denom: atom_on_neutron.clone(),
                asset_b_denom: osmo_on_neutron.clone(),
                single_side_lp_limits: SingleSideLpLimits {
                    asset_a_limit: Uint128::new(1000000),
                    asset_b_limit: Uint128::new(10000000),
                },
            },
        ),
        fallback_address: None,
        operation_mode: ContractOperationModeConfig::Permissioned(vec![]),
    };

    let covenant_contract = contract_instantiate(
        neutron_request_builder,
        ACC_0_KEY,
        valence_covenant_two_party_pol_code_id,
        &serde_json::to_string(&covenant_instantiate_msg).unwrap(),
        "two-party-pol-covenant-side-happy",
        None,
        "",
    )?;
    info!(target: target,"Covenant contract: {:?}", covenant_contract.address);
    let covenant = Covenant::TwoPartyPol {
        rb: neutron_request_builder,
        contract_address: &covenant_contract.address,
    };

    // Query the covenant addresses
    let clock_address = covenant.query_clock_address();
    let holder_address = covenant.query_holder_address();
    let liquid_pooler_address = covenant.query_liquid_pooler_address();
    let party_a_router_address = covenant.query_interchain_router_address("party_a".to_string());
    let party_b_router_address = covenant.query_interchain_router_address("party_b".to_string());
    let party_a_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_a".to_string());
    let party_b_ibc_forwarder_address = covenant.query_ibc_forwarder_address("party_b".to_string());

    info!(target: target,"Fund covenant addresses with NTRN...");
    let mut addresses = vec![
        clock_address.clone(),
        holder_address.clone(),
        liquid_pooler_address.clone(),
        party_a_router_address.clone(),
        party_b_router_address.clone(),
    ];
    if !party_a_ibc_forwarder_address.is_empty() {
        addresses.push(party_a_ibc_forwarder_address.clone());
    }
    if !party_b_ibc_forwarder_address.is_empty() {
        addresses.push(party_b_ibc_forwarder_address.clone());
    }
    for address in &addresses {
        send(
            neutron_request_builder,
            ACC_0_KEY,
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

    info!(target: target,"Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        let forwarder_b_state =
            query_contract_state(neutron_request_builder, &party_b_ibc_forwarder_address);
        info!(target: target,"Forwarder A state: {:?}", forwarder_a_state);
        info!(target: target,"Forwarder B state: {:?}", forwarder_b_state);
        if forwarder_a_state == "ica_created" && forwarder_b_state == "ica_created" {
            party_a_deposit_address = covenant.query_deposit_address("party_a".to_string());
            party_b_deposit_address = covenant.query_deposit_address("party_b".to_string());
            break;
        }
    }
    info!(target: target,"Party A deposit address: {}", party_a_deposit_address);
    info!(target: target,"Party B deposit address: {}", party_b_deposit_address);

    info!(target: target,"Fund the forwarders with sufficient funds...");
    send(
        gaia_request_builder,
        ACC_0_KEY,
        &party_a_deposit_address,
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
    send(
        osmosis_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(uosmo_contribution_amount),
        }],
        &Coin {
            denom: osmo_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!(target: target,"Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == osmo_on_neutron.clone()
                && c.amount >= Uint128::new(uosmo_contribution_amount)
        }) {
            info!(target: target,"Holder received ATOM & OSMO");
            break;
        } else if holder_state == "active" {
            info!(target: target,"Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!(target: target,"Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron.clone(),
        },
        AssetInfo::NativeToken {
            denom: osmo_on_neutron.clone(),
        },
    );

    loop {
        let balance = get_lp_token_balance(
            neutron_request_builder,
            &lp_token_address,
            &liquid_pooler_address,
        );

        if balance == "0" {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        } else {
            break;
        }
    }

    info!(target: target,"Tick until holder expires...");
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        info!(target: target,"Holder state: {:?}", holder_state);
        if holder_state == "expired" {
            break;
        }
    }

    let previous_balance_gaia = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
    let previous_balance_osmosis = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
    info!(target: target,"Party A claims");

    thread::sleep(Duration::from_secs(10));
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_1_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    info!(target: target,"Tick until both parties receive their funds");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!(target: target,"Hub receiver balances: {:?}", hub_receiver_balances);
        let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
        info!(target: target,"Osmosis receiver balances: {:?}", osmo_receiver_balances);
        if previous_balance_gaia != hub_receiver_balances
            && previous_balance_osmosis != osmo_receiver_balances
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    // Send the balances back so we have a fresh start for the next test
    let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
    for coin in hub_receiver_balances {
        if coin.denom != atom_denom.clone() {
            ibc_send(
                gaia_request_builder,
                ACC_1_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: atom_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(GAIA_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }
    let osmo_receiver_balances = get_balance(osmosis_request_builder, ACC2_ADDRESS_OSMO);
    for coin in osmo_receiver_balances {
        if coin.denom != osmo_denom.clone() {
            ibc_send(
                osmosis_request_builder,
                ACC_2_KEY,
                &neutron_admin_acc,
                coin,
                Coin {
                    denom: osmo_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(OSMOSIS_CHAIN)
                    .dest(NEUTRON_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    info!("Finished two party POL tests!");

    Ok(())
}
