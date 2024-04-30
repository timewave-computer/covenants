use cosmwasm_schema::cw_serde;
use cosmwasm_std::BlockInfo;
use cw_utils::{Duration, Expiration};

#[cw_serde]
#[serde(untagged)]
pub enum Deadline {
    Expiration(Expiration),
    Duration(Duration),
}

impl Default for Deadline {
    fn default() -> Self {
        Deadline::Expiration(Expiration::default())
    }
}

impl Deadline {
    pub fn into_expiration(self, block: &BlockInfo) -> Expiration {
        match self {
            Deadline::Expiration(expiration) => expiration,
            Deadline::Duration(duration) => duration.after(block),
        }
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::from_json;

    use super::Deadline;

    #[cw_serde]
    struct Example {
        expires: Deadline,
    }

    #[test]
    fn test() {
        let json_string = "{\"expires\": {\"never\": {}}}";
        println!("{:?}", from_json::<Example>(&json_string).unwrap());
    }
}
