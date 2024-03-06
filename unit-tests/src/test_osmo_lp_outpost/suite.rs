use cosmwasm_std::Addr;

use crate::setup::{base_suite::BaseSuiteMut, instantiates::osmo_lp_outpost::OsmpLpOutpostInstantiate, suite_builder::SuiteBuilder, CustomApp};

pub struct OsmoLpOutpostBuilder {
    pub builder: SuiteBuilder,
    pub instantiate_msg: OsmpLpOutpostInstantiate,
}

impl Default for OsmoLpOutpostBuilder {
    fn default() -> Self {

        Self {
            builder: SuiteBuilder::new(),
            instantiate_msg: OsmpLpOutpostInstantiate::default(),
        }
    }
}

impl OsmoLpOutpostBuilder {
    pub fn build(mut self) -> Suite {
        let outpost_addr = self.builder.contract_init(
            self.builder.osmo_lp_outpost_code_id,
            "outpost".to_string(),
            &self.instantiate_msg.msg,
            &vec![],
        );

        Suite {
            faucet: self.builder.faucet.clone(),
            admin: self.builder.admin.clone(),
            outpost: outpost_addr,
            app: self.builder.build(),
        }
    }
}

#[allow(dead_code)]
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

    fn get_faucet_addr(&mut self) -> Addr {
        self.faucet.clone()
    }
}
