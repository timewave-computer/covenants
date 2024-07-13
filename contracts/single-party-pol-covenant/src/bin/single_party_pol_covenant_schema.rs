use cosmwasm_schema::write_api;
use valence_covenant_single_party_pol::msg::{InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        migrate: MigrateMsg,
    }
}
