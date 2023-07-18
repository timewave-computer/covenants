use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Binary, Reply, ReplyOn, SubMsg, SubMsgResponse, SubMsgResult, Uint128, Uint64,
    WasmMsg,
};
use covenant_lp::msg::AssetData;
use neutron_sdk::bindings::msg::IbcFee;
use prost::Message;

use crate::{
    contract::{
        instantiate, query, reply, CLOCK_REPLY_ID, DEFAULT_TIMEOUT_SECONDS, DEPOSITOR_REPLY_ID,
        HOLDER_REPLY_ID, LP_REPLY_ID, LS_REPLY_ID,
    },
    msg::InstantiateMsg,
    state::{CLOCK_CODE, DEPOSITOR_CODE, HOLDER_CODE, IBC_FEE, IBC_TIMEOUT, LP_CODE, LS_CODE},
};

use super::suite::{CREATOR_ADDR, TODO};

fn get_init_msg() -> InstantiateMsg {
    InstantiateMsg {
    preset_clock_fields: covenant_clock::msg::PresetClockFields {
        tick_max_gas: Some(Uint64::new(10000)),
        clock_code: 1,
        label: "covenant_clock_contract".to_string(),
        whitelist: vec![],
    },
    preset_ls_fields: covenant_ls::msg::PresetLsFields {
        ls_code: 1,
        label: "covenant_ls_contract".to_string(),
        ls_denom: "stuatom".to_string(),
        stride_neutron_ibc_transfer_channel_id: TODO.to_string(),
        neutron_stride_ibc_connection_id: TODO.to_string(),
    },
    preset_depositor_fields: covenant_depositor::msg::PresetDepositorFields {
        gaia_neutron_ibc_transfer_channel_id: TODO.to_string(),
        neutron_gaia_connection_id: TODO.to_string(),
        gaia_stride_ibc_transfer_channel_id: TODO.to_string(),
        depositor_code: 1,
        label: "covenant_depositor_contract".to_string(),
        st_atom_receiver_amount: covenant_depositor::msg::WeightedReceiverAmount {
            amount: 1,
        },
        atom_receiver_amount: covenant_depositor::msg::WeightedReceiverAmount {
            amount: 1,
        },
        autopilot_format: "{{\"autopilot\": {{\"receiver\": \"{st_ica}\",\"stakeibc\": {{\"stride_address\": \"{st_ica}\",\"action\": \"LiquidStake\"}}}}}}".to_string(),
    },
    preset_lp_fields: covenant_lp::msg::PresetLpFields {
        slippage_tolerance: None,
        autostake: Some(false),
        lp_code: 1,
        label: "covenant_lp_contract".to_string(),
        single_side_lp_limits: None,
        assets: AssetData {
            native_asset_denom: "uatom".to_string(),
            ls_asset_denom: "stuatom".to_string(),
        },
    },
    preset_holder_fields: covenant_holder::msg::PresetHolderFields {
        withdrawer: CREATOR_ADDR.to_string(),
        holder_code: 1,
        label: "covenant_holder_contract".to_string(),
    },
    label: "covenant_contract".to_string(),
    pool_address: TODO.to_string(),
    ibc_msg_transfer_timeout_timestamp: None,
    // preset_ibc_fee: PresetIbcFee {
    //     ack_fee: cosmwasm_std::Coin {
    //         denom: NEUTRON_DENOM.to_string(),
    //         amount: Uint128::new(1000u128),
    //     },
    //     timeout_fee: cosmwasm_std::Coin {
    //         denom: NEUTRON_DENOM.to_string(),
    //         amount: Uint128::new(1000u128),
    //     },
    // },
  }
}

/// Turn struct to protobuf
fn to_proto(item: impl Message) -> Vec<u8> {
    let mut buf = Vec::new();
    item.encode(&mut buf).unwrap();
    buf
}

#[derive(Message)]
pub struct MsgInstantiateContractResponse {
    #[prost(string, tag = "1")]
    pub contract_address: String,
    #[prost(bytes, optional, tag = "2")]
    pub data: Option<Vec<u8>>,
}

#[test]
fn test_init() {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR_ADDR, &[]);

    let init_msg = get_init_msg();
    let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(res.messages[0].id, CLOCK_REPLY_ID);

    // Verify ibc timeout and fee are saved correctly
    // TODO: change code to actually get it from user
    let ibc_timeout = IBC_TIMEOUT.load(&deps.storage).unwrap();
    assert_eq!(ibc_timeout, DEFAULT_TIMEOUT_SECONDS);

    let ibc_fee = IBC_FEE.load(&deps.storage).unwrap();
    assert_eq!(
        ibc_fee,
        IbcFee {
            recv_fee: vec![],
            ack_fee: vec![cosmwasm_std::Coin {
                denom: "untrn".to_string(),
                amount: Uint128::new(1000u128),
            }],
            timeout_fee: vec![cosmwasm_std::Coin {
                denom: "untrn".to_string(),
                amount: Uint128::new(1000u128),
            }],
        }
    );

    // Test clock reply
    let clock_reply_res = MsgInstantiateContractResponse {
        contract_address: "contract_clock".to_string(),
        data: None,
    };

    let reply_clock = Reply {
        id: CLOCK_REPLY_ID,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(Binary(to_proto(clock_reply_res))),
        }),
    };

    let reply_res = reply(deps.as_mut(), mock_env(), reply_clock).unwrap();

    assert_eq!(reply_res.messages.len(), 1);
    assert_eq!(reply_res.messages[0].id, HOLDER_REPLY_ID);

    // Test holder reply
    let holder_reply_res = MsgInstantiateContractResponse {
        contract_address: "contract_holder".to_string(),
        data: None,
    };

    let reply_holder = Reply {
        id: HOLDER_REPLY_ID,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(Binary(to_proto(holder_reply_res))),
        }),
    };

    let reply_res = reply(deps.as_mut(), mock_env(), reply_holder).unwrap();

    assert_eq!(reply_res.messages.len(), 1);
    assert_eq!(reply_res.messages[0].id, LP_REPLY_ID);

    // Test LP reply
    let lp_reply_res = MsgInstantiateContractResponse {
        contract_address: "contract_lp".to_string(),
        data: None,
    };

    let reply_lp = Reply {
        id: LP_REPLY_ID,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(Binary(to_proto(lp_reply_res))),
        }),
    };

    let reply_res = reply(deps.as_mut(), mock_env(), reply_lp).unwrap();

    assert_eq!(reply_res.messages.len(), 1);
    assert_eq!(reply_res.messages[0].id, LS_REPLY_ID);

    // Test LS reply
    let ls_reply_res = MsgInstantiateContractResponse {
        contract_address: "contract_ls".to_string(),
        data: None,
    };

    let reply_ls = Reply {
        id: LS_REPLY_ID,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(Binary(to_proto(ls_reply_res))),
        }),
    };

    let reply_res = reply(deps.as_mut(), mock_env(), reply_ls).unwrap();

    assert_eq!(reply_res.messages.len(), 1);
    assert_eq!(reply_res.messages[0].id, DEPOSITOR_REPLY_ID);

    // Test depositor reply
    let depositor_reply_res = MsgInstantiateContractResponse {
        contract_address: "contract_depositor".to_string(),
        data: None,
    };

    let reply_depositor = Reply {
        id: DEPOSITOR_REPLY_ID,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(Binary(to_proto(depositor_reply_res))),
        }),
    };

    let reply_res = reply(deps.as_mut(), mock_env(), reply_depositor).unwrap();

    assert_eq!(reply_res.messages.len(), 1);
    assert_eq!(
        reply_res.messages[0],
        SubMsg {
            id: 0,
            msg: WasmMsg::Migrate {
                contract_addr: "contract_clock".to_string(),
                new_code_id: 1,
                msg: to_binary(&covenant_clock::msg::MigrateMsg::ManageWhitelist {
                    add: Some(vec![
                        "contract_lp".to_string(),
                        "contract_ls".to_string(),
                        "contract_depositor".to_string()
                    ]),
                    remove: None
                })
                .unwrap()
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // After we init everything, lets verify our storage holds correct data
    // Basically test queries and direct storage
    let clock_addr = query(
        deps.as_ref(),
        mock_env(),
        crate::msg::QueryMsg::ClockAddress {},
    )
    .unwrap();
    assert_eq!(
        from_binary::<Addr>(&clock_addr).unwrap().as_ref(),
        "contract_clock"
    );

    let depositor_addr = query(
        deps.as_ref(),
        mock_env(),
        crate::msg::QueryMsg::DepositorAddress {},
    )
    .unwrap();
    assert_eq!(
        from_binary::<Addr>(&depositor_addr).unwrap().as_ref(),
        "contract_depositor"
    );

    let lp_addr = query(
        deps.as_ref(),
        mock_env(),
        crate::msg::QueryMsg::LpAddress {},
    )
    .unwrap();
    assert_eq!(
        from_binary::<Addr>(&lp_addr).unwrap().as_ref(),
        "contract_lp"
    );

    let ls_addr = query(
        deps.as_ref(),
        mock_env(),
        crate::msg::QueryMsg::LsAddress {},
    )
    .unwrap();
    assert_eq!(
        from_binary::<Addr>(&ls_addr).unwrap().as_ref(),
        "contract_ls"
    );

    let holder_addr = query(
        deps.as_ref(),
        mock_env(),
        crate::msg::QueryMsg::HolderAddress {},
    )
    .unwrap();
    assert_eq!(
        from_binary::<Addr>(&holder_addr).unwrap().as_ref(),
        "contract_holder"
    );

    let pool_addr = query(
        deps.as_ref(),
        mock_env(),
        crate::msg::QueryMsg::PoolAddress {},
    )
    .unwrap();
    assert_eq!(from_binary::<Addr>(&pool_addr).unwrap().as_ref(), TODO);

    // Verify code ids are saved, in our case the id are the same and is 1 for all of them
    let clock_code_id = CLOCK_CODE.load(deps.as_ref().storage).unwrap();
    assert_eq!(clock_code_id, 1);

    let holder_code_id = HOLDER_CODE.load(deps.as_ref().storage).unwrap();
    assert_eq!(holder_code_id, 1);

    let depositor_code_id = DEPOSITOR_CODE.load(deps.as_ref().storage).unwrap();
    assert_eq!(depositor_code_id, 1);

    let lp_code_id = LP_CODE.load(deps.as_ref().storage).unwrap();
    assert_eq!(lp_code_id, 1);

    let ls_code_id = LS_CODE.load(deps.as_ref().storage).unwrap();
    assert_eq!(ls_code_id, 1);
}
