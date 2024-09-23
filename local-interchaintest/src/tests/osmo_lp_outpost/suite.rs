use cosmwasm_std::Addr;
use localic_utils::{utils::test_context::TestContext, DEFAULT_KEY, OSMOSIS_CHAIN_NAME};
use std::time::SystemTime;

const TEST_N_MINT_TOKENS: u128 = 1_000_000_000;

// Even split for testing
const TEST_WEIGHT_PER_DENOM: u64 = 1;

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

pub struct CaseTestContextBuilder<'a> {
    test_ctx: &'a mut TestContext,

    // Not strictly required, but necessary for a proper test
    pool: Option<PoolBuilder<'a>>,
}

impl CaseTestContextBuilder {
    pub fn new(test_ctx: &'a mut TestContext) -> Self {
        Self {
            test_ctx,
            pool: PoolBuilder::default(),
        }
    }

    /// Registers a pool for creation upon building of the case context.
    pub fn with_pool(&mut self, kind: CreatePool) -> &mut Self {
        self.pool = Some(kind);

        self
    }

    /// Instantiates/mints/deploys unique components for the individual testing case.
    /// Will never cause a panic due to duplicate instantiation, but does not introduce
    /// only label/denom uniqueness
    pub fn build(mut self) -> CaseTestContext {
        let outpost = self.make_outpost();

        let pool_asset_a = self.make_mint_denom("A");
        let pool_asset_b = self.make_mint_denom("B");

        let pool = self.make_pool_if_required();

        CaseTestContext {
            outpost,
            pool,
            pool_asset_a,
            pool_asset_b,
        }
    }

    // Mint and create denom unique to the case
    fn make_mint_denom(&mut self, denom_num: &str) -> OwnedFullDenom {
        // To prevent test from failing due to duplicate denoms
        let denom_prefix = case_unique_prefix();
        let subdenom_name = format!("{denom_prefix}{denom_num}");

        test_ctx
            .build_tx_create_tokenfactory_token()
            .with_subdenom(denom_name.as_str())
            .with_chain_name(OMOSIS_CHAIN_NAME)
            .send()
            .unwrap();

        let full_denom = test_ctx
            .get_tokenfactory_denom()
            .subdenom(subdenom_name)
            .src(OSMOSIS_CHAIN_NAME)
            .get();

        // See above on testing quantities
        test_ctx
            .build_tx_mint_tokenfactory_token()
            .with_denom(&full_denom)
            .with_amount(TEST_N_MINT_TOKENS)
            .send();

        full_denom
    }

    fn make_outpost(&mut self) -> Addr {
        let outpost_cw = self
            .test_ctx
            .get_contract()
            .contract("valence_outpost_osmo_liquid_pooler")
            .get_cw();
        outpost_cw
            .instantiate(DEFAULT_KEY, "", "outpost", None, "")
            .unwrap();

        outpost_cw.contract_addr.unwrap()
    }

    fn make_pool_if_required(&mut self) -> Option<PoolId> {
        self.pool.map(|pool_builder| pool_builder.build())
    }
}

/// Lazily creates a pool.
/// This needs to be built in order to result in creation of the pool.
pub struct PoolBuilder<'a> {
    test_ctx: &'a mut TestContext,
    kind: PoolKind,
}

pub enum PoolBuilder {
    Xyk {
        test_ctx: &'a mut TestContext,

        weight_denom_a: u64,
        weight_denom_b: u64,

        deposit_denom_a: u64,
        deposit_denom_b: u64,
    },
    Pcl {
        test_ctx: &'a mut TestContext,
    },
}

impl PoolBuilder {
    /// Deploys a pool for the specified pool kind
    fn build(self) -> PoolId {
        match self {
            Self::Pcl => todo!(),
            Self::Xyk {
                test_ctx,
                weight_denom_a,
                weight_denom_b,
                deposit_denom_a,
                deposit_denom_b,
            } => {
                test_ctx
                    .build_tx_create_osmo_pool()
                    .with_weight(denom_a.denom, denom_a.weight)
                    .with_weight(denom_b.denom, denom_b.weight)
                    .with_initial_deposit(denom_a.denom, denom_a.deposit)
                    .with_initial_deposit(denom_b.denoom, denom_b.deposit)
                    .send()
                    .unwrap();

                test_ctx
                    .get_osmo_pool()
                    .denoms(denom_a.denom.to_owned(), denom_b.denom.to_owned())
                    .get_u64()
            }
        }
    }
}
