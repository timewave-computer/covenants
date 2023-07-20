use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Item;

use crate::msg::{AssetData, LPInfo, SingleSideLpLimits};

// store the clock address to verify calls
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
pub const LP_POSITION: Item<LPInfo> = Item::new("lp_position");
pub const HOLDER_ADDRESS: Item<Addr> = Item::new("holder_address");
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
pub const AUTOSTAKE: Item<bool> = Item::new("autostake");
pub const SLIPPAGE_TOLERANCE: Item<Decimal> = Item::new("slippage_tolerance");
pub const ASSETS: Item<AssetData> = Item::new("assets");
pub const DEPOSITOR_ADDR: Item<Addr> = Item::new("depositor_addr");

pub const SINGLE_SIDED_LP_LIMITS: Item<SingleSideLpLimits> = Item::new("single_side_lp_limit");
pub const PROVIDED_LIQUIDITY_INFO: Item<ProvidedLiquidityInfo> =
    Item::new("provided_liquidity_info");
pub const EXPECTED_NATIVE_TOKEN_AMOUNT: Item<Uint128> = Item::new("expected_native_token_amount");

#[cw_serde]
pub struct ProvidedLiquidityInfo {
    pub provided_amount_ls: Uint128,
    pub provided_amount_native: Uint128,
}

#[cw_serde]
pub enum ContractState {
    Instantiated,
    NativeTokenReceived,
    WithdrawComplete,
}
