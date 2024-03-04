use cosmwasm_std::Addr;

use crate::setup::{base_suite::BaseSuiteMut, suite_builder::SuiteBuilder, CustomApp};

pub(super) struct Suite {
    pub app: CustomApp,

    pub faucet: Addr,
    pub admin: Addr,
    pub outpost: Addr,
}

impl BaseSuiteMut for Suite {
    fn get_app(&mut self) -> &mut CustomApp {
        &mut self.app
    }

    fn get_clock_addr(&mut self) -> Addr {
        // outpost is not clocked
        Addr::unchecked("")
    }
}

impl Suite {
    pub fn build(mut builder: SuiteBuilder, outpost: Addr) -> Self {
        let faucet = builder.faucet.clone();
        let admin = builder.admin.clone();

        Self {
            app: builder.build(),
            faucet,
            admin,
            outpost,
        }
    }
}

impl Suite {
    pub fn new_default() -> Self {
        let mut builder = SuiteBuilder::new();

        let outpost_addr = builder.contract_init(
            builder.osmo_lp_outpost_code_id,
            "outpost".to_string(),
            &covenant_outpost_osmo_liquid_pooler::msg::InstantiateMsg {},
            &vec![],
        );

        Self::build(builder, outpost_addr)
    }
}
