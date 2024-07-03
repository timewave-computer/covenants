use astroport::asset::AssetInfo;
use localic_std::{modules::cosmwasm::contract_query, transactions::ChainRequestBuilder};

pub fn get_pool_address(
    rb: &ChainRequestBuilder,
    factory_address: &str,
    asset1: AssetInfo,
    asset2: AssetInfo,
) -> String {
    let pair_info = contract_query(
        rb,
        factory_address,
        &serde_json::to_string(&astroport::factory::QueryMsg::Pair {
            asset_infos: vec![asset1, asset2],
        })
        .unwrap(),
    );
    pair_info["data"]["contract_addr"]
        .as_str()
        .unwrap()
        .to_string()
}

pub fn get_lp_token_address(
    rb: &ChainRequestBuilder,
    factory_address: &str,
    asset1: AssetInfo,
    asset2: AssetInfo,
) -> String {
    let pair_info = contract_query(
        rb,
        factory_address,
        &serde_json::to_string(&astroport::factory::QueryMsg::Pair {
            asset_infos: vec![asset1, asset2],
        })
        .unwrap(),
    );
    pair_info["data"]["liquidity_token"]
        .as_str()
        .unwrap()
        .to_string()
}

pub fn get_lp_token_balance(
    rb: &ChainRequestBuilder,
    token_address: &str,
    account_address: &str,
) -> String {
    let balance = contract_query(
        rb,
        token_address,
        &serde_json::to_string(&cw20::Cw20QueryMsg::Balance {
            address: account_address.to_string(),
        })
        .unwrap(),
    );
    balance["balance"].as_str().unwrap_or_default().to_string()
}
