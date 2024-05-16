use cosmwasm_schema::cw_serde;
use cosmwasm_std::to_json_vec;
use hex::encode;
use sha2::{Digest, Sha256};

#[cw_serde]
pub struct DenomTrace {
    pub path: String,
    pub base_denom: String,
}

impl DenomTrace {
    pub fn get_full_denom_path(&self) -> String {
        if self.path.is_empty() {
            return self.base_denom.to_string();
        }
        format!("ibc/{}", self.base_denom)
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        let trace = self.clone();

        hasher.update(&to_json_vec(&trace).unwrap());
        let result = hasher.finalize();

        let result_vec = result.to_vec();

        encode(result_vec)
    }

    pub fn ibc_denom(&self) -> String {
        if !self.path.is_empty() {
            return format!("ibc/{}", self.hash());
        }
        self.base_denom.to_string()
    }
}

pub fn parse_denom_trace(raw_denom: String) -> DenomTrace {
    let denom_split = raw_denom.split('/').collect::<Vec<&str>>();

    if denom_split[0] == raw_denom {
        return DenomTrace {
            path: "".to_string(),
            base_denom: raw_denom.to_string(),
        };
    }

    let (path, base_denom) = extract_path_and_base_from_full_denom(denom_split);

    DenomTrace { path, base_denom }
}

pub fn extract_path_and_base_from_full_denom(full_denom_items: Vec<&str>) -> (String, String) {
    let mut path: Vec<&str> = Vec::new();
    let mut base_denom: Vec<&str> = Vec::new();

    let length = full_denom_items.len();
    let mut i = 0;
    while i < length {
        // todo: validate channel here?
        if i < length - 1 && length > 2 {
            path.push(full_denom_items[i]);
            path.push(full_denom_items[i + 1]);
        } else {
            base_denom = full_denom_items[i..].to_vec();
            break;
        }
        i += 2;
    }

    (path.join("/"), base_denom.join("/"))
}

pub fn get_prefixed_denom(port_id: String, channel_id: String, native_denom: String) -> String {
    format!("{}/{}/{}", port_id, channel_id, native_denom)
}

pub fn get_ibc_denom(native_denom: String, channel_id: String) -> String {
    let prefixed_denom = get_prefixed_denom("transfer".to_string(), channel_id, native_denom);
    println!("prefixed_denom: {:?}", prefixed_denom);

    let src_denom_trace = parse_denom_trace(prefixed_denom);
    println!("src_denom_trace: {:?}", src_denom_trace);

    src_denom_trace.ibc_denom()
}
