use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct AutopilotConfig {
    pub receiver: String,
    pub stakeibc: Stakeibc,
}

#[cw_serde]
pub struct Autopilot {
    pub autopilot: AutopilotConfig,
}

#[cw_serde]
pub struct Stakeibc {
    pub action: String,
    pub stride_address: String,
    // pub ibc_receiver: String,
    // pub transfer_channel: String,
}
