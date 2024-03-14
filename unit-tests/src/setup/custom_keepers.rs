use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Api, Binary, BlockInfo, CustomMsg, CustomQuery, Querier,
    Storage,
};
use covenant_utils::neutron::{Params, QueryParamsResponse};
use cw_multi_test::error::{AnyError, AnyResult};
use cw_multi_test::{AppResponse, CosmosRouter, Module, StargateQuery};
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::gamm::v1beta1::{
    PoolAsset, QueryCalcExitPoolCoinsFromSharesResponse, QueryCalcJoinPoolNoSwapSharesResponse,
    QueryCalcJoinPoolSharesResponse, QueryPoolResponse,
};
use prost::Message;

use std::fmt::Debug;
use std::marker::PhantomData;

use crate::setup::DENOM_LS_ATOM_ON_NTRN;

use super::{DENOM_ATOM, DENOM_FALLBACK};

pub struct CustomStargateKeeper<ExecT, QueryT, SudoT>(
    PhantomData<(ExecT, QueryT, SudoT)>,
    &'static str,
    &'static str,
    &'static str,
);

impl<ExecT, QueryT, SudoT> CustomStargateKeeper<ExecT, QueryT, SudoT> {
    pub fn new(execute_msg: &'static str, query_msg: &'static str, sudo_msg: &'static str) -> Self {
        Self(Default::default(), execute_msg, query_msg, sudo_msg)
    }
}

impl<ExecT, QueryT, SudoT> Module for CustomStargateKeeper<ExecT, QueryT, SudoT>
where
    ExecT: Debug + Serialize,
    QueryT: Debug + Serialize,
    SudoT: Debug,
{
    type ExecT = ExecT;
    type QueryT = QueryT;
    type SudoT = SudoT;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: QueryT,
    ) -> AnyResult<Binary> {
        let query: StargateQuery = from_json(to_json_binary(&request).unwrap()).unwrap();
        // TODO: these mocks should be configurable on top-level SuiteBuilder config, pre build
        if query.path == "/neutron.interchaintxs.v1.Query/Params" {
            let response = QueryParamsResponse {
                params: Params {
                    msg_submit_tx_max_messages: cosmwasm_std::Uint64::new(1000),
                    register_fee: vec![cosmwasm_std::Coin {
                        amount: cosmwasm_std::Uint128::new(1000000),
                        denom: "untrn".to_string(),
                    }],
                },
            };

            return Ok(to_json_binary(&response).unwrap());
        }

        if query.path == "/osmosis.gamm.v1beta1.Query/Pool" {
            let pool = osmosis_std::types::osmosis::gamm::v1beta1::Pool {
                address: "address".to_string(),
                id: 1,
                pool_params: None,
                future_pool_governor: "governor".to_string(),
                total_shares: Some(Coin {
                    amount: "101010".to_string(),
                    denom: DENOM_FALLBACK.to_string(),
                }),
                pool_assets: vec![
                    PoolAsset {
                        token: Some(Coin {
                            amount: "100".to_string(),
                            denom: DENOM_ATOM.to_string(),
                        }),
                        weight: "50".to_string(),
                    },
                    PoolAsset {
                        token: Some(Coin {
                            amount: "100".to_string(),
                            denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                        }),
                        weight: "50".to_string(),
                    },
                ],
                total_weight: "123123".to_string(),
            };

            let pool_shim = osmosis_std::shim::Any {
                type_url: "/osmosis.gamm.v1beta1.Pool".to_string(),
                value: pool.encode_to_vec(),
            };

            let response = QueryPoolResponse {
                pool: Some(pool_shim),
            };

            return Ok(to_json_binary(&response).unwrap());
        }

        if query.path == "/osmosis.gamm.v1beta1.Query/CalcExitPoolCoinsFromShares" {
            let tokens_out = vec![
                Coin {
                    amount: "1".to_string(),
                    denom: DENOM_ATOM.to_string(),
                },
                Coin {
                    amount: "1".to_string(),
                    denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                },
            ];
            let response = QueryCalcExitPoolCoinsFromSharesResponse { tokens_out };

            return Ok(to_json_binary(&response).unwrap());
        }

        if query.path == "/osmosis.gamm.v1beta1.Query/CalcJoinPoolShares" {
            let tokens_out = vec![
                Coin {
                    amount: "1".to_string(),
                    denom: DENOM_ATOM.to_string(),
                },
                Coin {
                    amount: "1".to_string(),
                    denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                },
            ];
            let response = QueryCalcJoinPoolSharesResponse {
                tokens_out,
                share_out_amount: "1".to_string(),
            };

            return Ok(to_json_binary(&response).unwrap());
        }

        if query.path == "/osmosis.gamm.v1beta1.Query/CalcJoinPoolNoSwapShares" {
            let tokens_out = vec![
                Coin {
                    amount: "1".to_string(),
                    denom: DENOM_ATOM.to_string(),
                },
                Coin {
                    amount: "1".to_string(),
                    denom: DENOM_LS_ATOM_ON_NTRN.to_string(),
                },
            ];
            let response = QueryCalcJoinPoolNoSwapSharesResponse {
                tokens_out,
                shares_out: "1".to_string(),
            };

            return Ok(to_json_binary(&response).unwrap());
        }

        Err(AnyError::msg(self.2))
    }
}
