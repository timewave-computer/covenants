use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128, Uint64, Decimal};
use covenant_clock_derive::clocked;
use neutron_sdk::bindings::{msg::IbcFee, query::QueryInterchainAccountAddressResponse};

#[cw_serde]
pub struct InstantiateMsg {
    /// weighted receiver information used to determine
    /// where and how many funds should be sent from depositor
    pub st_atom_receiver: WeightedReceiver,
    pub atom_receiver: WeightedReceiver,
    /// address for the clock. this contract verifies
    /// that only the clock can execute ticks
    pub clock_address: String,
    /// ibc transfer channel on gaia for neutron
    /// this is used to ibc transfer uatom on gaia
    /// to the LP contract
    pub gaia_neutron_ibc_transfer_channel_id: String,
    /// IBC connection ID on neutron for gaia
    /// We make an Interchain Account over this connection
    pub neutron_gaia_connection_id: String,
    /// ibc transfer channel on gaia for stride
    /// This is used to ibc transfer uatom on gaia
    /// to the ica on stride
    pub gaia_stride_ibc_transfer_channel_id: String,
    /// address of the liquid staker module that will be used
    /// to query for the ICA address on stride
    pub ls_address: String,
    /// json formatted string meant to be used for one-click
    /// liquid staking on stride
    pub autopilot_format: String,
    /// neutron requires fees to be set to refund relayers for
    /// submission of ack and timeout messages.
    /// recv_fee and ack_fee paid in untrn from this contract
    pub ibc_fee: IbcFee,
    /// ibc denom of uatom on neutron
    pub neutron_atom_ibc_denom: String,
    /// timeout in seconds. this is used to craft a timeout timestamp
    /// that will be attached to the IBC transfer message from the ICA
    /// on the host chain (gaia) to its destination. typically
    /// this timeout should be greater than the ICA timeout, otherwise
    /// if the ICA times out, the destination chain receiving the funds
    /// will also receive the IBC packet with an expired timestamp.
    pub ibc_transfer_timeout: Uint64,
    /// time in seconds for ICA SubmitTX messages from neutron
    /// note that ICA uses ordered channels, a timeout implies
    /// channel closed. We can reopen the channel by reregistering
    /// the ICA with the same port id and connection id
    pub ica_timeout: Uint64,
}

#[cw_serde]
pub struct PresetDepositorFields {
    pub gaia_neutron_ibc_transfer_channel_id: String,
    pub neutron_gaia_connection_id: String,
    pub gaia_stride_ibc_transfer_channel_id: String,
    pub depositor_code: u64,
    pub label: String,
    pub st_atom_receiver_amount: WeightedReceiverAmount,
    pub atom_receiver_amount: WeightedReceiverAmount,
    pub autopilot_format: String,
    pub neutron_atom_ibc_denom: String,
}

#[cw_serde]
pub struct WeightedReceiverAmount {
    pub amount: Uint128,
}

impl WeightedReceiverAmount {
    /// builds an `InstantiateMsg` by taking in any fields not known on instantiation
    pub fn to_weighted_receiver(self, addr: String) -> WeightedReceiver {
        WeightedReceiver {
            amount: self.amount,
            address: addr,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl PresetDepositorFields {
    pub fn to_instantiate_msg(
        self,
        st_atom_receiver_addr: String,
        clock_address: String,
        ls_address: String,
        lp_address: String,
        ibc_fee: IbcFee,
        ibc_transfer_timeout: Uint64,
        ica_timeout: Uint64,
    ) -> InstantiateMsg {
        InstantiateMsg {
            st_atom_receiver: self
                .st_atom_receiver_amount
                .to_weighted_receiver(st_atom_receiver_addr),
            atom_receiver: self.atom_receiver_amount.to_weighted_receiver(lp_address),
            clock_address,
            gaia_neutron_ibc_transfer_channel_id: self.gaia_neutron_ibc_transfer_channel_id,
            neutron_gaia_connection_id: self.neutron_gaia_connection_id,
            gaia_stride_ibc_transfer_channel_id: self.gaia_stride_ibc_transfer_channel_id,
            ls_address,
            autopilot_format: self.autopilot_format,
            ibc_fee,
            neutron_atom_ibc_denom: self.neutron_atom_ibc_denom,
            ibc_transfer_timeout,
            ica_timeout,
        }
    }
}

#[cw_serde]
pub struct WeightedReceiver {
    pub amount: Uint128,
    pub address: String,
}

#[clocked]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(WeightedReceiver)]
    StAtomReceiver {},
    #[returns(WeightedReceiver)]
    AtomReceiver {},
    #[returns(Addr)]
    ClockAddress {},
    #[returns(ContractState)]
    ContractState {},
    #[returns(QueryInterchainAccountAddressResponse)]
    DepositorInterchainAccountAddress {},
    /// this query goes to neutron and get stored ICA with a specific query
    #[returns(QueryInterchainAccountAddressResponse)]
    InterchainAccountAddress {
        interchain_account_id: String,
        connection_id: String,
    },
    // this query returns ICA from contract store, which saved from acknowledgement
    #[returns((String, String))]
    InterchainAccountAddressFromContract { interchain_account_id: String },
    // this query returns acknowledgement result after interchain transaction
    #[returns(Option<AcknowledgementResult>)]
    AcknowledgementResult {
        interchain_account_id: String,
        sequence_id: u64,
    },
    // this query returns non-critical errors list
    #[returns(Vec<(Vec<u8>, String)>)]
    ErrorsQueue {},
    #[returns(String)]
    AutopilotFormat {},
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum MigrateMsg {
    UpdateConfig {
        clock_addr: Option<String>,
        st_atom_receiver: Option<WeightedReceiver>,
        atom_receiver: Option<WeightedReceiver>,
        gaia_neutron_ibc_transfer_channel_id: Option<String>,
        neutron_gaia_connection_id: Option<String>,
        gaia_stride_ibc_transfer_channel_id: Option<String>,
        ls_address: Option<String>,
        autopilot_format: Option<String>,
        ibc_config: Option<IbcConfig>,
    },
    UpdateCodeId {
        data: Option<Binary>,
    },
}

#[cw_serde]
pub struct OpenAckVersion {
    pub version: String,
    pub controller_connection_id: String,
    pub host_connection_id: String,
    pub address: String,
    pub encoding: String,
    pub tx_type: String,
}

#[cw_serde]
pub enum ContractState {
    /// Contract was instantiated, create ica
    Instantiated,
    /// ICA was created, send native token to lper
    ICACreated,
    /// Verify native token was sent to lper and send ls msg
    VerifyNativeToken,
    /// Verify the lper entered a position, if not try to resend ls msg again
    VerifyLp,
    /// Depositor completed his mission.
    Complete,
}


/// SudoPayload is a type that stores information about a transaction that we try to execute
/// on the host chain. This is a type introduced for our convenience.
#[cw_serde]
pub struct SudoPayload {
    pub message: String,
    pub port_id: String,
}

/// Serves for storing acknowledgement calls for interchain transactions
#[cw_serde]
pub enum AcknowledgementResult {
    /// Success - Got success acknowledgement in sudo with array of message item types in it
    Success(Vec<String>),
    /// Error - Got error acknowledgement in sudo with payload message in it and error details
    Error((String, String)),
    /// Timeout - Got timeout acknowledgement in sudo with payload message in it
    Timeout(String),
}

#[cw_serde]
pub struct LpConfig {
    /// the native token amount we expect to be funded with
    pub expected_native_token_amount: Uint128,
    /// stride redemption rate is variable so we set the expected ls token amount 
    pub expected_ls_token_amount: Uint128,
    /// accepted return amount fluctuation that gets applied to EXPECTED_LS_TOKEN_AMOUNT
    pub allowed_return_delta: Uint128,
    /// address of the liquidity pool we plan to enter
    pub pool_address: Addr,
    /// amounts of native and ls tokens we consider ok to single-side lp
    pub single_side_lp_limits: SingleSideLpLimits,
    /// boolean flag for enabling autostaking of LP tokens upon liquidity provisioning
    pub autostake: Option<bool>,
    /// slippage tolerance parameter for liquidity provisioning 
    pub slippage_tolerance: Option<Decimal>,
}

/// single side lp limits define the highest amount (in `Uint128`) that
/// we consider acceptable to provide single-sided. 
/// if asset balance exceeds these limits, double-sided liquidity should be provided.
#[cw_serde]
pub struct SingleSideLpLimits {
    pub native_asset_limit: Uint128,
    pub ls_asset_limit: Uint128,
}

#[cw_serde]
pub struct IbcConfig {
    pub ibc_fee: IbcFee,
    pub ibc_transfer_timeout: Uint64,
    pub ica_timeout: Uint64,
}