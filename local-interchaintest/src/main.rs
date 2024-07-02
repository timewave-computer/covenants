#![allow(dead_code, unused_must_use)]

use local_ictest_e2e::{tests::two_party_pol::two_party_pol_native::test_two_party_pol_native, utils::{constants::{API_URL, CHAIN_CONFIG_PATH, GAIA_CHAIN, NEUTRON_CHAIN, STRIDE_CHAIN}, file_system::read_json_file, liquid_staking::set_up_host_zone, test_context::TestContext}};
use localic_std::polling::poll_for_start;
use reqwest::blocking::Client;

// local-ic start neutron_gaia --api-port 42069
fn main() {
    let client = Client::new();
    poll_for_start(&client, API_URL, 300);

    let configured_chains = read_json_file(CHAIN_CONFIG_PATH).unwrap();

    let mut test_ctx = TestContext::from(configured_chains);
    set_up_host_zone(&mut test_ctx);
    
    println!("Stride channels: {}", test_ctx.get_transfer_channels().src(STRIDE_CHAIN).get_all().join(" ,"));
    println!("Neutron channels: {}", test_ctx.get_transfer_channels().src(NEUTRON_CHAIN).get_all().join(" ,"));
    println!("Gaia channels: {}", test_ctx.get_transfer_channels().src(GAIA_CHAIN).get_all().join(" ,"));
    println!("Stride connections: {}", test_ctx.get_connections().src(STRIDE_CHAIN).get_all().join(" ,"));
    println!("Neutron connections: {}", test_ctx.get_connections().src(NEUTRON_CHAIN).get_all().join(" ,"));
    println!("Gaia connections: {}", test_ctx.get_connections().src(GAIA_CHAIN).get_all().join(" ,"));

    println!("Gaia to NTRN channel: {}", test_ctx
    .get_transfer_channels()
    .src(GAIA_CHAIN)
    .dest(NEUTRON_CHAIN)
    .get());

    println!("Gaia to Stride channel: {}", test_ctx
    .get_transfer_channels()
    .src(GAIA_CHAIN)
    .dest(STRIDE_CHAIN)
    .get());
    //test_two_party_pol_native(&mut test_ctx);
}
