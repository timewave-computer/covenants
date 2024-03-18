use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{AppResponse, Executor};

use super::{CustomApp, ADMIN};

pub trait BaseSuiteMut {
    fn get_app(&mut self) -> &mut CustomApp;
    fn get_clock_addr(&mut self) -> Addr;
    fn get_faucet_addr(&mut self) -> Addr;

    fn tick_clock_debug(&mut self) {
        let clock_addr = self.get_clock_addr();
        let app = self.get_app();

        let res = app
            .execute_contract(
                app.api().addr_make(ADMIN),
                clock_addr,
                &covenant_clock::msg::ExecuteMsg::Tick {},
                &[],
            )
            .unwrap();

        println!("res: {:?}", res);
    }

    fn tick(&mut self, msg: &str) -> AppResponse {
        println!("Tick: {}", msg);
        let clock_addr = self.get_clock_addr();
        let app = self.get_app();

        let res = app
            .execute_contract(
                app.api().addr_make(ADMIN),
                clock_addr,
                &covenant_clock::msg::ExecuteMsg::Tick {},
                &[],
            )
            .unwrap();
        res
    }

    fn tick_contract(&mut self, contract: Addr) -> AppResponse {
        let clock_addr = self.get_clock_addr();
        let app = self.get_app();

        app.execute_contract(
            clock_addr,
            contract,
            &covenant_clock::msg::ExecuteMsg::Tick {},
            &[],
        )
        .unwrap()
    }

    fn fund_contract(&mut self, amount: &[Coin], to: Addr) {
        let faucet = self.get_faucet_addr().clone();
        let app = self.get_app();

        app.send_tokens(faucet, to, amount).unwrap();
    }
}

pub trait BaseSuite {
    fn get_app(&self) -> &CustomApp;

    fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        let app = self.get_app();
        app.wrap().query_balance(addr, denom).unwrap()
    }

    fn query_all_balances(&self, addr: &Addr) -> Vec<Coin> {
        let app = self.get_app();
        app.wrap().query_all_balances(addr).unwrap()
    }

    fn assert_balance(&self, addr: impl Into<String>, coin: Coin) {
        let app = self.get_app();
        let bal = app.wrap().query_balance(addr, &coin.denom).unwrap();
        assert_eq!(bal, coin);
    }
}
