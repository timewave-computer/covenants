use std::{thread, time::Duration};

use cosmwasm_std::{coin, Binary, Decimal, Uint128};
use localic_std::{
    errors::LocalError,
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query},
    node::Chain,
};

use crate::utils::{
    constants::{
        ACC_0_KEY, ASTROPORT_PATH, EXECUTE_FLAGS, GAIA_CHAIN, NEUTRON_CHAIN, VALENCE_PATH,
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

    // Instantiate the native coin registry contract
    let native_coin_registry_instantiate_msg = NativeCoinRegistryInstantiateMsg {
        owner: test_ctx.get_admin_addr().src(NEUTRON_CHAIN).get(),
    };
    let native_coin_registry_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        ACC_0_KEY,
        /*test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_native_coin_registry")
        .unwrap()
        .clone(),*/
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
            /*code_id: test_ctx
            .get_chain(NEUTRON_CHAIN)
            .contract_codes
            .get("astroport_pair_stable")
            .unwrap()
            .clone(),*/
            code_id: 29,
            pair_type: PairType::Stable {},
            total_fee_bps: 0,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: true,
        }],
        /*token_code_id: test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_token")
        .unwrap()
        .clone(),*/
        token_code_id: 28,
        fee_address: None,
        generator_address: None,
        owner: test_ctx.get_admin_addr().src(NEUTRON_CHAIN).get(),
        /*whitelist_code_id: test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_whitelist")
        .unwrap()
        .clone(),*/
        whitelist_code_id: 20,
        coin_registry_address: native_coin_registry_contract.address.to_string(),
    };
    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN),
        ACC_0_KEY,
        /*test_ctx
        .get_chain(NEUTRON_CHAIN)
        .contract_codes
        .get("astroport_factory")
        .unwrap()
        .clone(),*/
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
        coin(
            amount_to_send,
            test_ctx.get_native_denom().src(GAIA_CHAIN).get(),
        ),
        coin(100000, test_ctx.get_native_denom().src(GAIA_CHAIN).get()),
        &test_ctx
            .get_transfer_channels()
            .src(GAIA_CHAIN)
            .dest(NEUTRON_CHAIN)
            .get(),
        None,
    )?;
    thread::sleep(Duration::from_secs(3));

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

    Ok(())
}
