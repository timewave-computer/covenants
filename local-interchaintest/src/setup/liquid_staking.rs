use localic_std::transactions::ChainRequestBuilder;

use crate::{
    pretty_print,
    utils::{
        file_system::{write_json_file, write_str_to_container_file},
        ibc::get_ibc_denom,
        queries::{query_validator_set, ValidatorsJson},
        stride::{
            add_stakeibc_validator, query_host_zone, query_stakeibc_validators,
            register_stride_host_zone,
        },
        test_context::TestContext,
    },
    ADMIN_KEY, GAIA_CHAIN, GAIA_CHAIN_ID, STRIDE_CHAIN, STRIDE_CHAIN_ID,
};

pub fn set_up_host_zone(test_ctx: &mut TestContext) {
    let stride = test_ctx.get_chain(STRIDE_CHAIN);
    let stride_rb = &stride.rb;

    let stride_to_gaia_channel_id = test_ctx
        .get_transfer_channels()
        .src(STRIDE_CHAIN)
        .dest(GAIA_CHAIN)
        .get();
    let atom_on_stride = get_ibc_denom("uatom", &stride_to_gaia_channel_id);

    if query_host_zone(stride_rb, GAIA_CHAIN_ID) {
        println!("Host zone registered.");
    } else {
        println!("Host zone not registered.");
        register_stride_host_zone(
            stride_rb,
            &test_ctx
                .get_connections()
                .src(STRIDE_CHAIN)
                .dest(GAIA_CHAIN)
                .get(),
            "uatom",
            "cosmos",
            &atom_on_stride,
            &stride_to_gaia_channel_id,
            ADMIN_KEY,
        )
        .unwrap();
    }

    register_gaia_validators_on_stride(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN),
        test_ctx
            .get_request_builder()
            .get_request_builder(STRIDE_CHAIN),
    );
}

pub fn register_gaia_validators_on_stride(
    gaia: &ChainRequestBuilder,
    stride: &ChainRequestBuilder,
) {
    let val_set_entries = query_validator_set(gaia);

    if query_stakeibc_validators(stride, GAIA_CHAIN_ID)
        .validators
        .is_empty()
    {
        println!("Validators registered.");
        return;
    }

    let validators_json = serde_json::to_value(ValidatorsJson {
        validators: val_set_entries,
    })
    .unwrap();

    println!("\nvalidators_json:\n");
    pretty_print(&validators_json);
    write_json_file("validators.json", &validators_json.to_string());

    let stride_path = format!("/var/cosmos-chain/{STRIDE_CHAIN_ID}/config/validators.json");

    write_str_to_container_file(stride, "validators.json", &validators_json.to_string());

    let stakeibc_vals_response = query_stakeibc_validators(stride, GAIA_CHAIN_ID);
    if stakeibc_vals_response.validators.is_empty() {
        println!("Registering validator.");
        add_stakeibc_validator(stride, &stride_path, GAIA_CHAIN_ID);
    } else {
        println!("Validators registered.");
    }
}
