use std::{collections::BTreeMap, str::FromStr, thread, time::Duration};

use cosmwasm_std::{coin, Binary, Coin, Decimal, Uint128, Uint64};
use covenant_utils::{
    op_mode::ContractOperationModeConfig, split::SplitConfig, InterchainCovenantParty,
    NativeCovenantParty, PoolPriceConfig, SingleSideLpLimits,
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
        two_party_pol::{
            query_clock_address, query_contract_state, query_deposit_address, query_holder_address,
            query_ibc_forwarder_address, query_interchain_router_address,
            query_liquid_pooler_address, tick,
        },
    },
    utils::{
        constants::{
            ACC1_ADDRESS_GAIA, ACC1_ADDRESS_NEUTRON, ACC2_ADDRESS_NEUTRON, ACC_0_KEY, ACC_1_KEY,
            ACC_2_KEY, ASTROPORT_PATH, EXECUTE_FLAGS, GAIA_CHAIN, NEUTRON_CHAIN, VALENCE_PATH,
        },
        ibc::ibc_send,
        setup::deploy_contracts_on_chain,
        test_context::TestContext,
    },
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
    deploy_contracts_on_chain(test_ctx, VALENCE_PATH, NEUTRON_CHAIN);
    deploy_contracts_on_chain(test_ctx, ASTROPORT_PATH, NEUTRON_CHAIN);

    info!("Starting two party POL native tests...");
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

    let atom_on_neutron_denom = test_ctx
        .get_ibc_denoms()
        .src(GAIA_CHAIN)
        .dest(NEUTRON_CHAIN)
        .get();
    let neutron_on_atom = test_ctx
        .get_ibc_denoms()
        .src(NEUTRON_CHAIN)
        .dest(GAIA_CHAIN)
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
        neutron_request_builder,
        &factory_contract.address,
        ACC_0_KEY,
        &serde_json::to_string(&create_pair_msg).unwrap(),
        EXECUTE_FLAGS,
    )?;

    // Send some ATOM to NTRN
    let amount_to_send = 20_000_000_000;
    ibc_send(
        gaia_request_builder,
        ACC_0_KEY,
        &neutron_admin_acc,
        coin(amount_to_send, atom_denom.clone()),
        coin(100000, atom_denom.clone()),
        &test_ctx
            .get_transfer_channels()
            .src(GAIA_CHAIN)
            .dest(NEUTRON_CHAIN)
            .get(),
        None,
    )?;
    loop {
        info!("Waiting to receive IBC transfer...");
        let balance = get_balance(neutron_request_builder, &neutron_admin_acc);
        if balance
            .iter()
            .any(|c| c.denom == atom_on_neutron_denom && c.amount >= Uint128::new(amount_to_send))
        {
            break;
        }
        thread::sleep(Duration::from_secs(3));
    }

    // Provide the ATOM/NTRN liquidity to the pair
    let pool_addr = get_pool_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron_denom.clone(),
        },
        AssetInfo::NativeToken {
            denom: neutron_denom.clone(),
        },
    );

    let uatom_contribution_amount: u128 = 5_000_000_000;
    let untrn_contribution_amount: u128 = 50_000_000_000;
    let provide_liquidity_msg = astroport::pair::ExecuteMsg::ProvideLiquidity {
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: atom_on_neutron_denom.clone(),
                },
                amount: Uint128::from(uatom_contribution_amount*4),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: neutron_denom.clone(),
                },
                amount: Uint128::from(untrn_contribution_amount*4),
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
        &format!("--amount {uatom_contribution_amount}{atom_on_neutron_denom},{untrn_contribution_amount}{neutron_denom} {EXECUTE_FLAGS}"),
    ).unwrap();
    thread::sleep(Duration::from_secs(3));

    // Instantiate the covenant
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

    info!("Two party POL happy path...");
    let covenant_instantiate_msg = valence_covenant_two_party_pol::msg::InstantiateMsg {
        label: "two-party-pol-covenant-happy".to_string(),
        timeouts: Timeouts {
            ica_timeout: Uint64::new(10000),          // seconds
            ibc_transfer_timeout: Uint64::new(10000), // seconds
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
    info!("Covenant contract: {:?}", covenant_contract.address);

    // Query the covenant addresses
    let clock_address = query_clock_address(neutron_request_builder, &covenant_contract.address);
    let holder_address = query_holder_address(neutron_request_builder, &covenant_contract.address);
    let liquid_pooler_address =
        query_liquid_pooler_address(neutron_request_builder, &covenant_contract.address);
    let party_a_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );
    let party_a_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );

    info!("Fund covenant addresses with NTRN...");
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

    info!("Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        info!("Forwarder A state: {:?}", forwarder_a_state);
        if forwarder_a_state == "ica_created" {
            party_a_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_a".to_string(),
            );
            party_b_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_b".to_string(),
            );
            break;
        }
    }

    info!("Party A deposit address: {}", party_a_deposit_address);
    info!("Party B deposit address: {}", party_b_deposit_address);

    info!("Fund the forwarders with sufficient funds...");
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
        neutron_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(untrn_contribution_amount),
        }],
        &Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!("Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron_denom.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == neutron_denom.clone() && c.amount >= Uint128::new(untrn_contribution_amount)
        }) {
            info!("Holder received ATOM & NTRN");
            break;
        } else if holder_state == "active" {
            info!("Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron_denom.clone(),
        },
        AssetInfo::NativeToken {
            denom: neutron_denom.clone(),
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

    info!("Tick until holder expires...");
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        info!("Holder state: {:?}", holder_state);
        if holder_state == "expired" {
            break;
        }
    }

    info!("Party A claims and router receives the funds");
    let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
    info!("Router A balances: {:?}", router_a_balances);
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);
    let holder_balances = get_balance(neutron_request_builder, &holder_address);
    info!("Holder balances: {:?}", holder_balances);

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
    info!("Router A balances: {:?}", router_a_balances);
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);
    let holder_balances = get_balance(neutron_request_builder, &holder_address);
    info!("Holder balances: {:?}", holder_balances);

    info!("Tick until party A claim is distributed");
    info!("Hub receiver address: {}", ACC1_ADDRESS_GAIA);
    loop {
        let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
        info!("Router A balances: {:?}", router_a_balances);
        let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
        info!("Router B balances: {:?}", router_b_balances);
        let holder_balances = get_balance(neutron_request_builder, &holder_address);
        info!("Holder balances: {:?}", holder_balances);
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!("Hub receiver balances: {:?}", hub_receiver_balances);
        let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
        info!("Neutron receiver balances: {:?}", neutron_receiver_balances);
        if hub_receiver_balances
            .iter()
            .any(|c| c.denom == atom_denom.clone())
            && hub_receiver_balances
                .iter()
                .any(|c| c.denom == neutron_on_atom.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Party B claims and router receives the funds");
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_2_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    info!("Tick until both parties receive their funds");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!("Hub receiver balances: {:?}", hub_receiver_balances);
        let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
        info!("Neutron receiver balances: {:?}", neutron_receiver_balances);
        if neutron_receiver_balances
            .iter()
            .any(|c| c.denom == neutron_denom.clone())
            && neutron_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_neutron_denom.clone())
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
    let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
    for coin in neutron_receiver_balances {
        if coin.denom != neutron_denom.clone() {
            ibc_send(
                neutron_request_builder,
                ACC_2_KEY,
                &test_ctx.get_admin_addr().src(GAIA_CHAIN).get(),
                coin,
                Coin {
                    denom: neutron_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(NEUTRON_CHAIN)
                    .dest(GAIA_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    let current_block_height = chain.get_height();
    info!("Two party POL share based ragequit path...");
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
        deposit_deadline: Expiration::AtHeight(current_block_height + 200),
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
    info!("Covenant contract: {:?}", covenant_contract.address);

    // Query the covenant addresses
    let clock_address = query_clock_address(neutron_request_builder, &covenant_contract.address);
    let holder_address = query_holder_address(neutron_request_builder, &covenant_contract.address);
    let liquid_pooler_address =
        query_liquid_pooler_address(neutron_request_builder, &covenant_contract.address);
    let party_a_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );
    let party_a_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );

    info!("Fund covenant addresses with NTRN...");
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

    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        info!("Forwarder A state: {:?}", forwarder_a_state);
        if forwarder_a_state == "ica_created" {
            party_a_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_a".to_string(),
            );
            party_b_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_b".to_string(),
            );
            break;
        }
    }

    info!("Party A deposit address: {}", party_a_deposit_address);
    info!("Party B deposit address: {}", party_b_deposit_address);

    info!("Fund the forwarders with sufficient funds...");
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
        neutron_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(untrn_contribution_amount),
        }],
        &Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!("Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron_denom.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == neutron_denom.clone() && c.amount >= Uint128::new(untrn_contribution_amount)
        }) {
            info!("Holder received ATOM & NTRN");
            break;
        } else if holder_state == "active" {
            info!("Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron_denom.clone(),
        },
        AssetInfo::NativeToken {
            denom: neutron_denom.clone(),
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

    info!("Party A ragequits...");
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_1_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Ragequit {})
            .unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    loop {
        let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
        info!("Router A balances: {:?}", router_a_balances);
        let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
        info!("Router B balances: {:?}", router_b_balances);
        if router_a_balances
            .iter()
            .any(|c| c.denom == atom_on_neutron_denom.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Tick until party A ragequit is distributed...");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!("Hub receiver balances: {:?}", hub_receiver_balances);
        let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
        info!("Neutron receiver balances: {:?}", neutron_receiver_balances);
        if hub_receiver_balances
            .iter()
            .any(|c| c.denom == atom_denom.clone())
            && hub_receiver_balances
                .iter()
                .any(|c| c.denom == neutron_on_atom.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Party B claims and router receives the funds");
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_2_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    info!("Tick routers until both parties receive their funds");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!("Hub receiver balances: {:?}", hub_receiver_balances);
        let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
        info!("Neutron receiver balances: {:?}", neutron_receiver_balances);
        if neutron_receiver_balances
            .iter()
            .any(|c| c.denom == neutron_denom.clone())
            && neutron_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_neutron_denom.clone())
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
    let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
    for coin in neutron_receiver_balances {
        if coin.denom != neutron_denom.clone() {
            ibc_send(
                neutron_request_builder,
                ACC_2_KEY,
                &test_ctx.get_admin_addr().src(GAIA_CHAIN).get(),
                coin,
                Coin {
                    denom: neutron_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(NEUTRON_CHAIN)
                    .dest(GAIA_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    let current_block_height = chain.get_height();
    info!("Two party POL side based ragequit path...");
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
        deposit_deadline: Expiration::AtHeight(current_block_height + 200),
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
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(100)),
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(0)),
                    ]),
                },
            ),
            (
                neutron_denom.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(0)),
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(100)),
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
    info!("Covenant contract: {:?}", covenant_contract.address);

    // Query the covenant addresses
    let clock_address = query_clock_address(neutron_request_builder, &covenant_contract.address);
    let holder_address = query_holder_address(neutron_request_builder, &covenant_contract.address);
    let liquid_pooler_address =
        query_liquid_pooler_address(neutron_request_builder, &covenant_contract.address);
    let party_a_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );
    let party_a_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );

    info!("Fund covenant addresses with NTRN...");
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

    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        info!("Forwarder A state: {:?}", forwarder_a_state);
        if forwarder_a_state == "ica_created" {
            party_a_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_a".to_string(),
            );
            party_b_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_b".to_string(),
            );
            break;
        }
    }

    info!("Party A deposit address: {}", party_a_deposit_address);
    info!("Party B deposit address: {}", party_b_deposit_address);

    info!("Fund the forwarders with sufficient funds...");
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
        neutron_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(untrn_contribution_amount),
        }],
        &Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!("Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron_denom.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == neutron_denom.clone() && c.amount >= Uint128::new(untrn_contribution_amount)
        }) {
            info!("Holder received ATOM & NTRN");
            break;
        } else if holder_state == "active" {
            info!("Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron_denom.clone(),
        },
        AssetInfo::NativeToken {
            denom: neutron_denom.clone(),
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

    info!("Party A ragequits...");
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_1_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Ragequit {})
            .unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    loop {
        let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
        info!("Router A balances: {:?}", router_a_balances);
        let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
        info!("Router B balances: {:?}", router_b_balances);
        if router_a_balances
            .iter()
            .any(|c| c.denom == atom_on_neutron_denom.clone())
        {
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Party B claims and router receives the funds");
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_2_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    info!("Tick routers until both parties receive their funds");
    loop {
        let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
        info!("Router A balances: {:?}", router_a_balances);
        let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
        info!("Router B balances: {:?}", router_b_balances);
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!("Hub receiver balances: {:?}", hub_receiver_balances);
        let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
        info!("Neutron receiver balances: {:?}", neutron_receiver_balances);
        if neutron_receiver_balances
            .iter()
            .any(|c| c.denom == neutron_denom.clone())
            && neutron_receiver_balances
                .iter()
                .any(|c| c.denom == atom_on_neutron_denom.clone())
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
    let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
    for coin in neutron_receiver_balances {
        if coin.denom != neutron_denom.clone() {
            ibc_send(
                neutron_request_builder,
                ACC_2_KEY,
                &test_ctx.get_admin_addr().src(GAIA_CHAIN).get(),
                coin,
                Coin {
                    denom: neutron_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(NEUTRON_CHAIN)
                    .dest(GAIA_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    let current_block_height = chain.get_height();
    info!("Two party POL side based happy path...");
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
        lockup_config: Expiration::AtHeight(current_block_height + 200),
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
        deposit_deadline: Expiration::AtHeight(current_block_height + 180),
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
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(100)),
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(0)),
                    ]),
                },
            ),
            (
                neutron_denom.clone(),
                SplitConfig {
                    receivers: BTreeMap::from([
                        (ACC1_ADDRESS_GAIA.to_string(), Decimal::percent(0)),
                        (ACC2_ADDRESS_NEUTRON.to_string(), Decimal::percent(100)),
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
    info!("Covenant contract: {:?}", covenant_contract.address);

    // Query the covenant addresses
    let clock_address = query_clock_address(neutron_request_builder, &covenant_contract.address);
    let holder_address = query_holder_address(neutron_request_builder, &covenant_contract.address);
    let liquid_pooler_address =
        query_liquid_pooler_address(neutron_request_builder, &covenant_contract.address);
    let party_a_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_router_address = query_interchain_router_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );
    let party_a_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_a".to_string(),
    );
    let party_b_ibc_forwarder_address = query_ibc_forwarder_address(
        neutron_request_builder,
        &covenant_contract.address,
        "party_b".to_string(),
    );

    info!("Fund covenant addresses with NTRN...");
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

    info!("Tick until forwarders create ICA...");
    let party_a_deposit_address;
    let party_b_deposit_address;
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let forwarder_a_state =
            query_contract_state(neutron_request_builder, &party_a_ibc_forwarder_address);
        info!("Forwarder A state: {:?}", forwarder_a_state);
        if forwarder_a_state == "ica_created" {
            party_a_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_a".to_string(),
            );
            party_b_deposit_address = query_deposit_address(
                neutron_request_builder,
                &covenant_contract.address,
                "party_b".to_string(),
            );
            break;
        }
    }

    info!("Party A deposit address: {}", party_a_deposit_address);
    info!("Party B deposit address: {}", party_b_deposit_address);

    info!("Fund the forwarders with sufficient funds...");
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
        neutron_request_builder,
        ACC_0_KEY,
        &party_b_deposit_address,
        &[Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(untrn_contribution_amount),
        }],
        &Coin {
            denom: neutron_denom.clone(),
            amount: Uint128::new(5000),
        },
    )
    .unwrap();

    info!("Tick until forwarders forward the funds to the holder...");
    loop {
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        let holder_balance = get_balance(neutron_request_builder, &holder_address);
        if holder_balance.iter().any(|c| {
            c.denom == atom_on_neutron_denom.clone()
                && c.amount >= Uint128::new(uatom_contribution_amount)
        }) && holder_balance.iter().any(|c| {
            c.denom == neutron_denom.clone() && c.amount >= Uint128::new(untrn_contribution_amount)
        }) {
            info!("Holder received ATOM & NTRN");
            break;
        } else if holder_state == "active" {
            info!("Holder is active");
            break;
        } else {
            tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        }
    }

    info!("Tick until holder sends funds to LiquidPooler and LPer receives LP tokens...");
    let lp_token_address = get_lp_token_address(
        neutron_request_builder,
        &factory_contract.address,
        AssetInfo::NativeToken {
            denom: atom_on_neutron_denom.clone(),
        },
        AssetInfo::NativeToken {
            denom: neutron_denom.clone(),
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

    info!("Tick until holder expires...");
    loop {
        tick(neutron_request_builder, ACC_0_KEY, &clock_address);
        let holder_state = query_contract_state(neutron_request_builder, &holder_address);
        info!("Holder state: {:?}", holder_state);
        if holder_state == "expired" {
            break;
        }
    }

    info!("Party A claims and router receives the funds");
    let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
    info!("Router A balances: {:?}", router_a_balances);
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);
    let hub_receiver_balances_before_claim = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
    info!(
        "Hub receiver balances before claim: {:?}",
        hub_receiver_balances_before_claim
    );
    let neutron_receiver_balances_before_claim =
        get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
    info!(
        "Neutron receiver balances before claim: {:?}",
        neutron_receiver_balances_before_claim
    );

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

    info!("Party B claims and router receives the funds");
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);
    contract_execute(
        neutron_request_builder,
        &holder_address,
        ACC_2_KEY,
        &serde_json::to_string(&valence_two_party_pol_holder::msg::ExecuteMsg::Claim {}).unwrap(),
        EXECUTE_FLAGS,
    )
    .unwrap();
    thread::sleep(Duration::from_secs(5));

    let router_a_balances = get_balance(neutron_request_builder, &party_a_router_address);
    info!("Router A balances: {:?}", router_a_balances);
    let router_b_balances = get_balance(neutron_request_builder, &party_b_router_address);
    info!("Router B balances: {:?}", router_b_balances);

    info!("Tick until both parties receive their funds");
    loop {
        let hub_receiver_balances = get_balance(gaia_request_builder, ACC1_ADDRESS_GAIA);
        info!("Hub receiver balances: {:?}", hub_receiver_balances);
        let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
        info!("Neutron receiver balances: {:?}", neutron_receiver_balances);
        if hub_receiver_balances_before_claim != hub_receiver_balances
            && neutron_receiver_balances_before_claim != neutron_receiver_balances
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
    let neutron_receiver_balances = get_balance(neutron_request_builder, ACC2_ADDRESS_NEUTRON);
    for coin in neutron_receiver_balances {
        if coin.denom != neutron_denom.clone() {
            ibc_send(
                neutron_request_builder,
                ACC_2_KEY,
                &test_ctx.get_admin_addr().src(GAIA_CHAIN).get(),
                coin,
                Coin {
                    denom: neutron_denom.clone(),
                    amount: Uint128::new(5000),
                },
                &test_ctx
                    .get_transfer_channels()
                    .src(NEUTRON_CHAIN)
                    .dest(GAIA_CHAIN)
                    .get(),
                None,
            )
            .unwrap();
        }
    }

    info!("Finished two party POL native tests!");

    Ok(())
}
