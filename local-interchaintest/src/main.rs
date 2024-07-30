#![allow(dead_code, unused_must_use)]

use std::error::Error;

use local_ictest_e2e::tests::{
    remote_chain_splitter::remote_chain_splitter::test_remote_chain_splitter,
    single_party_pol::single_party_pol_stride::test_single_party_pol_stride,
    swap::token_swap::test_token_swap,
    two_party_pol::{
        two_party_pol_native::test_two_party_pol_native,
        two_party_pol_not_native::test_two_party_pol, two_party_pol_osmo::test_two_party_pol_osmo,
    },
};

use localic_std::polling::poll_for_start;
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME, LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_NAME, STRIDE_CHAIN_NAME,
};
use reqwest::blocking::Client;

// Run `local-ic start neutron_gaia --api-port 42069` before running this test inside the local-interchaintest directory to spin up the environment
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let client = Client::new();
    poll_for_start(&client, LOCAL_IC_API_URL, 300);

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_chain(ConfigChainBuilder::default_stride().build()?)
        .with_chain(ConfigChainBuilder::default_gaia().build()?)
        .with_artifacts_dir("artifacts")
        .with_transfer_channels(OSMOSIS_CHAIN_NAME, NEUTRON_CHAIN_NAME)
        .with_transfer_channels(OSMOSIS_CHAIN_NAME, GAIA_CHAIN_NAME)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, GAIA_CHAIN_NAME)
        .with_transfer_channels(STRIDE_CHAIN_NAME, GAIA_CHAIN_NAME)
        .with_transfer_channels(STRIDE_CHAIN_NAME, NEUTRON_CHAIN_NAME)
        .build()?;

    test_ctx.set_up_stride_host_zone(GAIA_CHAIN_NAME);

    test_single_party_pol_stride(&mut test_ctx);
    test_token_swap(&mut test_ctx);
    test_two_party_pol_osmo(&mut test_ctx);
    test_two_party_pol_native(&mut test_ctx);
    test_two_party_pol(&mut test_ctx);
    test_remote_chain_splitter(&mut test_ctx);

    Ok(())
}
