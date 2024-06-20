use cosmwasm_schema::cw_serde;
use localic_std::transactions::ChainRequestBuilder;

#[cw_serde]
pub struct ValidatorSetEntry {
    pub address: String,
    pub voting_power: String,
    pub name: String,
}

#[cw_serde]
pub struct ValidatorsJson {
    pub validators: Vec<ValidatorSetEntry>,
}

pub fn query_block_height(_chain: &ChainRequestBuilder) -> u64 {
    // let query_cmd = format!("block --output=json");
    // let mut query_block_response = chain.q(&query_cmd, false);
    // let block_height = &chain_status_response.take()[0]["block"];
    // println!("block response : {:?}", block_height);

    // let block_height = chain_status_response["block"]["header"]["height"].as_u64().unwrap();

    // println!("chain status query response: {:?}", block_height);
    // block_height
    // TODO: Implement this function
    100
}

pub fn query_validator_set(chain: &ChainRequestBuilder) -> Vec<ValidatorSetEntry> {
    let height = query_block_height(chain);
    let query_valset_cmd = format!("tendermint-validator-set {height} --output=json",);

    let valset_resp = chain.q(&query_valset_cmd, false);

    let mut val_set_entries: Vec<ValidatorSetEntry> = Vec::new();

    for entry in valset_resp["validators"].as_array().unwrap() {
        let address = entry["address"].as_str().unwrap();
        let voting_power = entry["voting_power"].as_str().unwrap();

        val_set_entries.push(ValidatorSetEntry {
            name: format!("val{}", val_set_entries.len() + 1),
            address: address.to_string(),
            voting_power: voting_power.to_string(),
        });
    }
    val_set_entries
}

pub fn get_keyring_accounts(rb: &ChainRequestBuilder) {
    let accounts = rb.binary("keys list --keyring-backend=test", false);

    let addrs = accounts["addresses"].as_array();
    addrs.map_or_else(
        || {
            println!("No accounts found.");
        },
        |addrs| {
            for acc in addrs.iter() {
                let name = acc["name"].as_str().unwrap_or_default();
                let address = acc["address"].as_str().unwrap_or_default();
                println!("Key '{name}': {address}");
            }
        },
    );
}
