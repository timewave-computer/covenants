#![allow(dead_code, unused_must_use)]

use local_ictest_e2e::{
    tests::two_party_pol::two_party_pol_native::test_two_party_pol_native,
    utils::{
        constants::{API_URL, CHAIN_CONFIG_PATH},
        file_system::read_json_file,
        liquid_staking::set_up_host_zone,
        test_context::TestContext,
    },
};
use localic_std::polling::poll_for_start;
use reqwest::blocking::Client;

// local-ic start neutron_gaia --api-port 42069
fn main() {
    env_logger::init();
    let client = Client::new();
    poll_for_start(&client, API_URL, 300);

    let configured_chains = read_json_file(CHAIN_CONFIG_PATH).unwrap();

    let mut test_ctx = TestContext::from(configured_chains);

    set_up_host_zone(&mut test_ctx);

    test_two_party_pol_native(&mut test_ctx);
}
