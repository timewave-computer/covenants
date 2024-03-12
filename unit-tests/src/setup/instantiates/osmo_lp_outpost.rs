pub struct OsmpLpOutpostInstantiate {
    pub msg: covenant_outpost_osmo_liquid_pooler::msg::InstantiateMsg,
}

impl From<OsmpLpOutpostInstantiate> for covenant_outpost_osmo_liquid_pooler::msg::InstantiateMsg {
    fn from(value: OsmpLpOutpostInstantiate) -> Self {
        value.msg
    }
}

impl OsmpLpOutpostInstantiate {
    pub fn new() -> Self {
        Self {
            msg: covenant_outpost_osmo_liquid_pooler::msg::InstantiateMsg {},
        }
    }
}

impl OsmpLpOutpostInstantiate {
    pub fn default() -> Self {
        Self::new()
    }
}
