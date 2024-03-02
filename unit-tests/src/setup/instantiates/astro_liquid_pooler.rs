use astroport::factory::PairType;
use cosmwasm_std::Decimal;
use covenant_utils::{PoolPriceConfig, SingleSideLpLimits};


pub struct AstroLiquidPoolerInstantiate {
    pub msg: covenant_astroport_liquid_pooler::msg::InstantiateMsg,
}

impl From<AstroLiquidPoolerInstantiate> for covenant_astroport_liquid_pooler::msg::InstantiateMsg {
    fn from(value: AstroLiquidPoolerInstantiate) -> Self {
        value.msg
    }
}

impl AstroLiquidPoolerInstantiate {
    pub fn new(
        pool_address: String,
        clock_address: String,
        slippage_tolerance: Option<Decimal>,
        assets: covenant_astroport_liquid_pooler::msg::AssetData,
        single_side_lp_limits: SingleSideLpLimits,
        pool_price_config: PoolPriceConfig,
        pair_type: PairType,
        holder_address: String,
    ) -> Self {
        Self {
            msg: covenant_astroport_liquid_pooler::msg::InstantiateMsg {
                pool_address,
                clock_address,
                slippage_tolerance,
                assets,
                single_side_lp_limits,
                pool_price_config,
                pair_type,
                holder_address,
            },
        }
    }

    pub fn with_pool_address(&mut self, pool_address: String) -> &mut Self {
        self.msg.pool_address = pool_address;
        self
    }

    pub fn with_clock_address(&mut self, clock_address: String) -> &mut Self {
        self.msg.clock_address = clock_address;
        self
    }

    pub fn with_slippage_tolerance(&mut self, slippage_tolerance: Option<Decimal>) -> &mut Self {
        self.msg.slippage_tolerance = slippage_tolerance;
        self
    }

    pub fn with_assets(&mut self, assets: covenant_astroport_liquid_pooler::msg::AssetData) -> &mut Self {
        self.msg.assets = assets;
        self
    }

    pub fn with_single_side_lp_limits(&mut self, single_side_lp_limits: SingleSideLpLimits) -> &mut Self {
        self.msg.single_side_lp_limits = single_side_lp_limits;
        self
    }

    pub fn with_pool_price_config(&mut self, pool_price_config: PoolPriceConfig) -> &mut Self {
        self.msg.pool_price_config = pool_price_config;
        self
    }

    pub fn with_pair_type(&mut self, pair_type: PairType) -> &mut Self {
        self.msg.pair_type = pair_type;
        self
    }

    pub fn with_holder_address(&mut self, holder_address: String) -> &mut Self {
        self.msg.holder_address = holder_address;
        self
    }
}

impl AstroLiquidPoolerInstantiate {
    pub fn default(
        pool_address: String,
        clock_address: String,
        slippage_tolerance: Option<Decimal>,
        assets: covenant_astroport_liquid_pooler::msg::AssetData,
        single_side_lp_limits: SingleSideLpLimits,
        pool_price_config: PoolPriceConfig,
        pair_type: PairType,
        holder_address: String,
    ) -> Self {
        Self {
            msg: covenant_astroport_liquid_pooler::msg::InstantiateMsg {
                pool_address,
                clock_address,
                slippage_tolerance,
                assets,
                single_side_lp_limits,
                pool_price_config,
                pair_type,
                holder_address,
            },
        }
    }
}
