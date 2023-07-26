use cosmwasm_schema::write_api;
use covenant_depositor::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg};
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
