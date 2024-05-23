use serde::Serialize;
use serde_json::Value;

pub mod setup;
pub mod utils;

pub const API_URL: &str = "http://127.0.0.1:42069";
pub const WASM_EXTENSION: &str = "wasm";

pub const NEUTRON_CHAIN: &str = "neutron";
pub const NEUTRON_CHAIN_ID: &str = "localneutron-1";

pub const GAIA_CHAIN: &str = "gaia";
pub const GAIA_CHAIN_ID: &str = "localcosmos-1";

pub const STRIDE_CHAIN: &str = "stride";
pub const STRIDE_CHAIN_ID: &str = "localstride-3";

pub const CHAIN_CONFIG_PATH: &str = "chains/neutron_gaia.json";
pub const ARTIFACTS_PATH: &str = "../artifacts";

pub const TRANSFER_PORT: &str = "transfer";

pub const ADMIN_KEY: &str = "admin";
pub const ACC_0_KEY: &str = "acc0";

pub fn pretty_print(obj: &Value) {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    obj.serialize(&mut ser).unwrap();
    println!("{}", String::from_utf8(buf).unwrap());
}
