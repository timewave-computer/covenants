use astroport::asset::{PairInfo, Asset};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Binary, Deps, Empty, Env, StdResult, Addr, Uint128, DepsMut, MessageInfo, Response, BankMsg, Coin};
use covenant_macros::covenant_deposit_address;
use cw20::{Cw20QueryMsg, BalanceResponse, Cw20ExecuteMsg};
use cw_multi_test::{Contract, ContractWrapper};

mod suite;
mod tests;

pub fn two_party_pol_holder_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn mock_deposit_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        query,
    );
    Box::new(contract)
}


#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;

use self::suite::{DENOM_A, DENOM_B};

#[covenant_deposit_address]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DepositAddress {} => Ok(to_binary(&"splitter")?),
    }
}


pub fn mock_astro_pool_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        query_astro_pool,
    );
    Box::new(contract)
}

pub fn mock_astro_lp_token_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        execute_lp_token,
        crate::contract::instantiate,
        query_astro_lp_token,
    );
    Box::new(contract)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute_lp_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ExecuteMsg,
) -> Result<Response, ContractError> {
    let msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![
            Coin::new(200, DENOM_A),
            Coin::new(200, DENOM_B),
        ],
    };
    Ok(Response::default().add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query_astro_pool(_deps: Deps, _env: Env, msg: astroport::pair::QueryMsg) -> StdResult<Binary> {
    match msg {
        astroport::pair::QueryMsg::Pair {} => Ok(to_binary(&PairInfo {
            asset_infos: vec![],
            contract_addr: Addr::unchecked("contract0"),
            liquidity_token: Addr::unchecked("contract0"),
            pair_type: astroport::factory::PairType::Xyk {  },
        })?),
        astroport::pair::QueryMsg::Share { amount } => Ok(to_binary(&vec![
            Asset { info: astroport::asset::AssetInfo::NativeToken { denom: DENOM_A.to_string() }, amount: Uint128::new(200) },
            Asset { info: astroport::asset::AssetInfo::NativeToken { denom: DENOM_B.to_string() }, amount: Uint128::new(200) },

        ])?),
        _ => Ok(to_binary(&"-")?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query_astro_lp_token(_deps: Deps, _env: Env, msg: cw20::Cw20QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw20QueryMsg::Balance { address } => Ok(to_binary(&BalanceResponse {
            balance: Uint128::new(100)
        })?),
        _ => Ok(to_binary(&"-")?),
    }
}
