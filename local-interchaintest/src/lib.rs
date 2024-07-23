use localic_std::modules::bank::get_balance;
use localic_utils::utils::test_context::TestContext;

pub mod helpers;
pub mod tests;

pub fn send_non_native_balances(
    test_ctx: &mut TestContext,
    chain_name: &str,
    key: &str,
    source: &str,
    destination: &str,
    native_denom: &str,
) {
    let balances = get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(chain_name),
        source,
    );
    for coin in balances {
        if coin.denom != native_denom {
            test_ctx
                .build_tx_transfer()
                .with_chain_name(chain_name)
                .with_amount(coin.amount.u128())
                .with_recipient(destination)
                .with_denom(&coin.denom)
                .with_key(key)
                .send()
                .unwrap();
        }
    }
}
