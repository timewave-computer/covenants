use crate::utils::{constants::ADMIN_KEY, stride::register_stride_host_zone};

use super::{
    constants::{GAIA_CHAIN, GAIA_CHAIN_ID, STRIDE_CHAIN},
    ibc::get_ibc_denom,
    stride::query_host_zone,
    test_context::TestContext,
};

pub fn set_up_host_zone(test_ctx: &mut TestContext) {
    let stride = test_ctx.get_chain(STRIDE_CHAIN);
    let stride_rb = &stride.rb;

    let stride_to_gaia_channel_id = test_ctx
        .get_transfer_channels()
        .src(STRIDE_CHAIN)
        .dest(GAIA_CHAIN)
        .get();

    let atom_on_stride = get_ibc_denom(
        &test_ctx.get_native_denom().src(GAIA_CHAIN).get(),
        &stride_to_gaia_channel_id,
    );

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
            &test_ctx.get_native_denom().src(GAIA_CHAIN).get(),
            "cosmos",
            &atom_on_stride,
            &stride_to_gaia_channel_id,
            ADMIN_KEY,
        )
        .unwrap();
    }
}
