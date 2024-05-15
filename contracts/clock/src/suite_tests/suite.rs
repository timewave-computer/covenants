use cosmwasm_std::{Addr, Uint64};
use cw_multi_test::{App, AppResponse, Executor};
use valence_clock_tester::msg::Mode;

use crate::{
    contract::DEFAULT_TICK_MAX_GAS,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};

use super::{clock_contract, clock_tester_contract};

const ADMIN: &str = "admin";

pub struct Suite {
    pub app: App,
    pub clock: Addr,
    pub admin: Addr,

    /// code ID of the clock contract in use. used for migrate messages.
    pub clock_code_id: u64,
}

pub struct SuiteBuilder {
    pub app: App,
    pub instantiate: InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            app: App::default(),
            instantiate: InstantiateMsg {
                tick_max_gas: Some(DEFAULT_TICK_MAX_GAS),
                whitelist: vec![],
                initial_queue: vec![],
            },
        }
    }
}

impl SuiteBuilder {
    pub fn with_tick_max_gas(mut self, tmg: u64) -> Self {
        self.instantiate.tick_max_gas = Some(Uint64::new(tmg));
        self
    }

    pub fn with_whitelist(mut self, whitelist: Vec<Addr>) -> Self {
        self.instantiate.whitelist = whitelist.iter().map(|a| a.to_string()).collect();
        self
    }

    pub fn with_initial_queue(mut self, initial_queue: Vec<String>) -> Self {
        self.instantiate.initial_queue = initial_queue;
        self
    }

    pub fn build(mut self) -> Suite {
        let clock_code = self.app.store_code(clock_contract());
        let clock = self
            .app
            .instantiate_contract(
                clock_code,
                Addr::unchecked(ADMIN),
                &self.instantiate,
                &[],
                "clock",
                Some(ADMIN.to_string()),
            )
            .unwrap();
        Suite {
            app: self.app,
            clock,
            admin: Addr::unchecked(ADMIN),
            clock_code_id: clock_code,
        }
    }
}

//actions
impl SuiteBuilder {
    // Generate test contracts
    pub fn generate_tester(&mut self, mode: Mode) -> Addr {
        let code_id = self.app.store_code(clock_tester_contract());
        self.app
            .instantiate_contract(
                code_id,
                Addr::unchecked(ADMIN),
                &valence_clock_tester::msg::InstantiateMsg { mode },
                &[],
                "clock-tester",
                Some(ADMIN.to_string()),
            )
            .unwrap()
    }
}

// actions
impl Suite {
    // enqueue's `who` and returns the queried queue after enqueueing
    // if no error occurs.
    pub fn enqueue(&mut self, who: &str) -> anyhow::Result<Vec<Addr>> {
        self.app.execute_contract(
            Addr::unchecked(who),
            self.clock.clone(),
            &ExecuteMsg::Enqueue {},
            &[],
        )?;
        Ok(self.query_queue_in_order_of_output())
    }

    // sends a message on WHO's behalf which removes them from the
    // queue. returns the queue's contents after dequeueing in order
    // of removal.
    pub fn dequeue(&mut self, who: &str) -> anyhow::Result<Vec<Addr>> {
        self.app.execute_contract(
            Addr::unchecked(who),
            self.clock.clone(),
            &ExecuteMsg::Dequeue {},
            &[],
        )?;
        Ok(self.query_queue_in_order_of_output())
    }

    // sends a tick to the clock.
    pub fn tick(&mut self) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            self.admin.clone(),
            self.clock.clone(),
            &ExecuteMsg::Tick {},
            &[],
        )
    }

    // pauses the clock.
    pub fn pause(&mut self) -> anyhow::Result<AppResponse> {
        self.app.migrate_contract(
            self.admin.clone(),
            self.clock.clone(),
            &MigrateMsg::Pause {},
            self.clock_code_id,
        )
    }

    // unpauses the clock.
    pub fn unpause(&mut self) -> anyhow::Result<AppResponse> {
        self.app.migrate_contract(
            self.admin.clone(),
            self.clock.clone(),
            &MigrateMsg::Unpause {},
            self.clock_code_id,
        )
    }

    // updates tick_max_gas.
    pub fn update_tick_max_gas(&mut self, new_value: Uint64) -> anyhow::Result<AppResponse> {
        self.app.migrate_contract(
            self.admin.clone(),
            self.clock.clone(),
            &MigrateMsg::UpdateTickMaxGas { new_value },
            self.clock_code_id,
        )
    }

    pub fn manage_whitelisted(
        &mut self,
        add: Option<Vec<String>>,
        remove: Option<Vec<String>>,
    ) -> anyhow::Result<AppResponse> {
        self.app.migrate_contract(
            self.admin.clone(),
            self.clock.clone(),
            &MigrateMsg::ManageWhitelist { add, remove },
            self.clock_code_id,
        )
    }
}

// queries
impl Suite {
    pub fn query_tick_max_gas(&self) -> Uint64 {
        self.app
            .wrap()
            .query_wasm_smart(&self.clock, &QueryMsg::TickMaxGas {})
            .unwrap()
    }

    pub fn query_paused(&self) -> bool {
        self.app
            .wrap()
            .query_wasm_smart(&self.clock, &QueryMsg::Paused {})
            .unwrap()
    }

    pub fn query_full_queue(&self) -> Vec<(Addr, u64)> {
        self.app
            .wrap()
            .query_wasm_smart(
                &self.clock,
                &QueryMsg::Queue {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap()
    }

    // queries the queue for all elements and returns addresses in the
    // order that they will leave the queue (smallest timestamp
    // first).
    pub fn query_queue_in_order_of_output(&self) -> Vec<Addr> {
        let mut queue = self.query_full_queue();
        queue.sort_by_key(|(_, time)| *time);
        queue.into_iter().map(|(addr, _)| addr).collect()
    }

    pub fn query_tester_tick_count(&self, tester: &Addr) -> u64 {
        let res: Uint64 = self
            .app
            .wrap()
            .query_wasm_smart(
                tester.to_string(),
                &valence_clock_tester::msg::QueryMsg::TickCount {},
            )
            .unwrap();
        res.u64()
    }

    pub fn query_whitelist(&self) -> Vec<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(&self.clock, &QueryMsg::Whitelist {})
            .unwrap()
    }
}
