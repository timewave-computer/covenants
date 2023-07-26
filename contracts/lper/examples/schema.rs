use cosmwasm_schema::write_api;
use covenant_lp::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg};
use neutron_sdk::sudo::msg::SudoMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg,
        sudo: SudoMsg,
    }
}
