use super::suite::{CaseTestContextBuilder, PoolKind};
use localic_std::errors::LocalError;
use localic_utils::utils::test_context::TestContext;

pub fn test_osmo_lp_outpost(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    log::info!("Starting osmo LP outpost tests...");

    test_double_sided_lp_xyk(test_ctx)?;

    log::info!("Finished osmo LP outpost tests!");

    Ok(())
}

fn test_double_sided_lp_xyk(test_ctx: &mut TestContext) -> Result<(), LocalError> {
    let case_ctx = CaseTestContextBuilder::new(test_ctx)
        .with_has_pool(PoolKind::Xyk)
        .build();

    Ok(())
}
