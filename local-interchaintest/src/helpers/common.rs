use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Messages {
    ContractState {},
    Tick {},
}
