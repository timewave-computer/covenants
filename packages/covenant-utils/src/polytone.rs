use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Empty, QuerierWrapper, QueryRequest, StdError, StdResult,
    Uint64,
};
use polytone::callbacks::CallbackRequest;

#[cw_serde]
pub enum PolytoneExecuteMsg {
    Query {
        msgs: Vec<QueryRequest<Empty>>,
        callback: CallbackRequest,
        timeout_seconds: Uint64,
    },
    Execute {
        msgs: Vec<CosmosMsg<Empty>>,
        callback: Option<CallbackRequest>,
        timeout_seconds: Uint64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum PolytoneQueryMsg {
    #[returns(Option<String>)]
    RemoteAddress { local_address: String },
    #[returns(Uint64)]
    BlockMaxGas,
}

pub fn get_polytone_execute_msg_binary(
    msgs: Vec<CosmosMsg>,
    callback: Option<CallbackRequest>,
    timeout_seconds: Uint64,
) -> StdResult<Binary> {
    let execute_msg = PolytoneExecuteMsg::Execute {
        msgs,
        callback,
        timeout_seconds,
    };
    to_json_binary(&execute_msg)
}

pub fn get_polytone_query_msg_binary(
    msgs: Vec<QueryRequest<Empty>>,
    callback: CallbackRequest,
    timeout_seconds: Uint64,
) -> StdResult<Binary> {
    let query_msg = PolytoneExecuteMsg::Query {
        msgs,
        callback,
        timeout_seconds,
    };
    to_json_binary(&query_msg)
}

pub fn query_polytone_proxy_address(
    local_address: String,
    note_address: String,
    querier: QuerierWrapper,
) -> Result<Option<String>, StdError> {
    let remote_address_query = PolytoneQueryMsg::RemoteAddress { local_address };

    querier.query_wasm_smart(note_address, &remote_address_query)
}
