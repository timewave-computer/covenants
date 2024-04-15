pub struct OsmoLpOutpostInstantiate {
    pub msg: valence_outpost_osmo_liquid_pooler::msg::InstantiateMsg,
}

impl From<OsmoLpOutpostInstantiate> for valence_outpost_osmo_liquid_pooler::msg::InstantiateMsg {
    fn from(value: OsmoLpOutpostInstantiate) -> Self {
        value.msg
    }
}

impl OsmoLpOutpostInstantiate {
    pub fn new() -> Self {
        Self {
            msg: valence_outpost_osmo_liquid_pooler::msg::InstantiateMsg {},
        }
    }
}

impl Default for OsmoLpOutpostInstantiate {
    fn default() -> Self {
        Self::new()
    }
}
