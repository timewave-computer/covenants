use cosmwasm_std::Addr;
use localic_std::modules::cosmwasm::CosmWasm;
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_NAME,
};
use std::{path::PathBuf, time::SystemTime};

// Consider pulling this from the contract
const VALENCE_OUTPOST_CONTRACT_NAME: &str = "valence_outpost_osmo_liquid_pooler";

const TEST_N_MINT_TOKENS: u128 = 1_000_000_000;

// Even split for testing
const TEST_WEIGHT_PER_DENOM: u64 = 1;
const TEST_DEPOSIT_PER_DENOM: u64 = TEST_N_MINT_TOKENS as u64;

/// Useful for making labels and other unique elements unique.
fn case_unique_prefix() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}

type PoolId = u64;

type OwnedFullDenom = String;

/// Deployed components, unique per test case.
/// Constructed by building a PoolBuilder and a CaseTestContextBuilder
#[derive(Clone)]
pub struct CaseTestContext {
    outpost: Addr,

    // Deployed pool for testing liquidity provisioning against
    // Only one of these should be tested at a time
    // Must be an XYK or PCL pool ID
    pool: Option<PoolId>,

    // Unique per case.
    pool_asset_a: OwnedFullDenom,

    // Unique per case.
    pool_asset_b: OwnedFullDenom,
}

/// Lazily creates a pool.
/// This needs to be built in order to result in creation of the pool.
pub enum PoolKind {
    Xyk,
    Pcl,
}

pub struct CaseTestContextBuilder<'a> {
    test_ctx: &'a mut TestContext,

    // Not strictly required, but necessary for a proper test
    has_pool: Option<PoolKind>,
}

impl<'a> CaseTestContextBuilder<'a> {
    pub fn new(test_ctx: &'a mut TestContext) -> Self {
        Self {
            test_ctx,
            has_pool: Option::default(),
        }
    }

    /// Registers a pool for creation upon building of the case context.
    pub fn with_has_pool(mut self, kind: PoolKind) -> Self {
        self.has_pool = Some(kind);

        self
    }

    /// Instantiates/mints/deploys unique components for the individual testing case.
    /// Will never cause a panic due to duplicate instantiation, but does not introduce
    /// only label/denom uniqueness
    pub fn build(mut self) -> CaseTestContext {
        let outpost = self.make_outpost();

        let pool_asset_a = self.make_mint_denom("A");
        let pool_asset_b = self.make_mint_denom("B");

        let pool = self.make_pool_if_required(&pool_asset_a, &pool_asset_b);

        CaseTestContext {
            outpost,
            pool,
            pool_asset_a,
            pool_asset_b,
        }
    }

    fn get_outpost_cw(&'a self) -> CosmWasm<'a> {
        let chain = self.test_ctx.get_chain(OSMOSIS_CHAIN_NAME);

        let code_id = chain
            .contract_codes
            .get(VALENCE_OUTPOST_CONTRACT_NAME)
            .unwrap();

        let artifacts_path = &self.test_ctx.artifacts_dir;

        CosmWasm::new_from_existing(
            &chain.rb,
            Some(PathBuf::from(format!(
                "{artifacts_path}/{VALENCE_OUTPOST_CONTRACT_NAME}.wasm"
            ))),
            Some(*code_id),
            None,
        )
    }

    fn make_mint_denom(&mut self, denom_num: &str) -> OwnedFullDenom {
        // To prevent test from failing due to duplicate denoms
        let denom_prefix = case_unique_prefix();
        let subdenom_name = format!("{denom_prefix}{denom_num}");

        self.test_ctx
            .build_tx_create_tokenfactory_token()
            .with_subdenom(subdenom_name.as_str())
            .with_chain_name(OSMOSIS_CHAIN_NAME)
            .send()
            .unwrap();

        let full_denom = self
            .test_ctx
            .get_tokenfactory_denom()
            .subdenom(subdenom_name)
            .src(OSMOSIS_CHAIN_ADMIN_ADDR)
            .get();

        // See above on testing quantities
        self.test_ctx
            .build_tx_mint_tokenfactory_token()
            .with_denom(&full_denom)
            .with_amount(TEST_N_MINT_TOKENS)
            .send();

        full_denom
    }

    fn make_outpost(&mut self) -> Addr {
        let mut outpost_cw = self.get_outpost_cw();
        let unique_label = format!("{}outpost", case_unique_prefix());

        outpost_cw
            .instantiate(DEFAULT_KEY, "", &unique_label, None, "")
            .unwrap();

        Addr::unchecked(outpost_cw.contract_addr.unwrap())
    }

    fn make_pool_if_required(
        &mut self,
        asset_a: &OwnedFullDenom,
        asset_b: &OwnedFullDenom,
    ) -> Option<PoolId> {
        self.has_pool.as_ref().map(|pool| match pool {
            PoolKind::Pcl => todo!(),
            PoolKind::Xyk => {
                self.test_ctx
                    .build_tx_create_osmo_pool()
                    .with_weight(asset_a, TEST_WEIGHT_PER_DENOM)
                    .with_weight(asset_b, TEST_WEIGHT_PER_DENOM)
                    .with_initial_deposit(asset_a, TEST_DEPOSIT_PER_DENOM)
                    .with_initial_deposit(asset_b, TEST_DEPOSIT_PER_DENOM)
                    .send()
                    .unwrap();

                self.test_ctx
                    .get_osmo_pool()
                    .denoms(asset_a.clone(), asset_b.clone())
                    .get_u64()
            }
        })
    }
}
