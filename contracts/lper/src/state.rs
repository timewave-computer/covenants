use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Item;

use crate::msg::{AssetData, SingleSideLpLimits, ProvidedLiquidityInfo, ContractState};

/// contract state tracks the state machine progress
pub const CONTRACT_STATE: Item<ContractState> = Item::new("contract_state");
/// native and ls asset denom information
pub const ASSETS: Item<AssetData> = Item::new("assets");

/// clock module address to verify the incoming ticks sender
pub const CLOCK_ADDRESS: Item<Addr> = Item::new("clock_address");
/// address of the liquidity pool we plan to enter
pub const POOL_ADDRESS: Item<Addr> = Item::new("pool_address");
/// holder module address to verify withdrawal requests
pub const HOLDER_ADDRESS: Item<Addr> = Item::new("holder_address");

/// boolean flag for enabling autostaking of LP tokens upon liquidity provisioning
pub const AUTOSTAKE: Item<bool> = Item::new("autostake");
/// slippage tolerance parameter for liquidity provisioning 
pub const SLIPPAGE_TOLERANCE: Item<Decimal> = Item::new("slippage_tolerance");

/// amounts of native and ls tokens we consider ok to single-side lp
pub const SINGLE_SIDED_LP_LIMITS: Item<SingleSideLpLimits> = Item::new("single_side_lp_limit");
/// keeps track of ls and native token amounts we provided to the pool
pub const PROVIDED_LIQUIDITY_INFO: Item<ProvidedLiquidityInfo> =
    Item::new("provided_liquidity_info");
/// the native token amount we expect to receive from depositor 
pub const EXPECTED_NATIVE_TOKEN_AMOUNT: Item<Uint128> = Item::new("expected_native_token_amount");
/// stride redemption rate is variable so we set the expected ls token amount 
pub const EXPECTED_LS_TOKEN_AMOUNT: Item<Uint128> = Item::new("expected_ls_token_amount");
/// accepted return amount fluctuation that gets applied to EXPECTED_LS_TOKEN_AMOUNT
pub const ALLOWED_RETURN_DELTA: Item<Uint128> = Item::new("allowed_return_delta");

