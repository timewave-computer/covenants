use cosmwasm_schema::{write_api};
use covenant_ls::msg::{InstantiateMsg, ExecuteMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg,
    }
}