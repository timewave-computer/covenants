use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, StdResult};
use cw_multi_test::Executor;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::single_party_covenant::SinglePartyCovenantInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, DENOM_ATOM, DENOM_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_NTRN, DENOM_LS_ATOM_ON_STRIDE,
    HUB_STRIDE_CHANNEL, NTRN_HUB_CHANNEL, NTRN_STRIDE_CHANNEL, SINGLE_PARTY_COVENANT_SALT,
};

pub(super) struct Suite {
    pub fuacet: Addr,
    pub admin: Addr,

    pub app: CustomApp,

    pub covenant_addr: Addr,
    pub clock_addr: Addr,
    pub holder_addr: Addr,
    pub splitter_addr: Addr,
    pub lser_addr: Addr,
    pub lper_addr: Addr,
    pub ls_forwarder_addr: Addr,
    pub lp_forwarder_addr: Addr,
    pub router_addr: Addr,

    // The receiver address
    pub party_receiver: Addr,
    pub party_local_receiver: Addr,
    pub ls_receiver: Addr,

    // Astro
    pub pool_addr: Addr,
    pub lp_token_addr: Addr,
}

impl BaseSuiteMut for Suite {
    fn get_app(&mut self) -> &mut CustomApp {
        &mut self.app
    }

    fn get_clock_addr(&mut self) -> Addr {
        self.clock_addr.clone()
    }
}

impl BaseSuite for Suite {
    fn get_app(&self) -> &CustomApp {
        &self.app
    }
}

impl Suite {
    fn build(
        mut builder: SuiteBuilder,
        covenant_addr: Addr,
        party_receiver: Addr,
        party_local_receiver: Addr,
        ls_receiver: Addr,
        pool_addr: Addr,
        lp_token_addr: Addr,
    ) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let holder_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::HolderAddress {},
            )
            .unwrap();

        let splitter_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::SplitterAddress {},
            )
            .unwrap();
        builder.fund_with_ntrn(&splitter_addr, 2_000_000_u128);

        let router_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::InterchainRouterAddress {},
            )
            .unwrap();
        builder.fund_with_ntrn(&router_addr, 2_000_000_u128);

        let lser_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::LiquidStakerAddress {},
            )
            .unwrap();
        builder.fund_with_ntrn(&lser_addr, 2_000_000_u128);

        let ls_forwarder_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::IbcForwarderAddress {
                    ty: "ls".to_string(),
                },
            )
            .unwrap();
        builder.fund_with_ntrn(&ls_forwarder_addr, 2_000_000_u128);

        let lper_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::LiquidPoolerAddress {},
            )
            .unwrap();
        let lp_forwarder_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_single_party_pol::msg::QueryMsg::IbcForwarderAddress {
                    ty: "lp".to_string(),
                },
            )
            .unwrap();
        builder.fund_with_ntrn(&lp_forwarder_addr, 2_000_000_u128);

        Self {
            fuacet: builder.fuacet.clone(),
            admin: builder.admin.clone(),

            covenant_addr,
            clock_addr,
            holder_addr,
            splitter_addr,
            lser_addr,
            lper_addr,
            ls_forwarder_addr,
            lp_forwarder_addr,
            router_addr,

            party_receiver,
            party_local_receiver,
            ls_receiver,

            pool_addr,
            lp_token_addr,

            // Make sure its the last one, because build() consume the builder
            app: builder.build(),
        }
    }
}

impl Suite {
    pub fn new_with_stable_pool() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr = builder.get_contract_addr(
            builder.single_party_covenant_code_id,
            SINGLE_PARTY_COVENANT_SALT,
        );

        // init astro pools
        let (pool_addr, lp_token_addr) = builder.init_astro_pool(
            astroport::factory::PairType::Stable {},
            coin(10_000_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(10_000_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        );

        let party_receiver = builder.get_random_addr();
        let party_receiver_on_ntrn = builder.get_random_addr();
        let ls_receiver = builder.get_random_addr();
        let ls_receiver_on_ntrn = builder.get_random_addr();

        let mut pfm = BTreeMap::new();
        pfm.insert(
            DENOM_LS_ATOM_ON_NTRN.to_string(),
            covenant_utils::PacketForwardMiddlewareConfig {
                local_to_hop_chain_channel_id: NTRN_STRIDE_CHANNEL.0.to_string(),
                hop_to_destination_chain_channel_id: HUB_STRIDE_CHANNEL.1.to_string(),
                hop_chain_receiver_address: ls_receiver.to_string(),
            },
        );

        let covenant_party = SinglePartyCovenantInstantiate::get_covenant_party(
            &party_receiver,
            &party_receiver_on_ntrn,
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            1_000_000_000_000_u128,
            pfm,
        );

        let pooler_config = SinglePartyCovenantInstantiate::get_astro_pooler_config(
            DENOM_ATOM_ON_NTRN,
            DENOM_LS_ATOM_ON_NTRN,
            &pool_addr,
            astroport::factory::PairType::Stable {},
            covenant_utils::SingleSideLpLimits {
                asset_a_limit: 500_000_000_u128.into(),
                asset_b_limit: 500_000_000_u128.into(),
            },
        );
        let ls_forwarder_config = SinglePartyCovenantInstantiate::get_forwarder_config_interchain(
            &ls_receiver,
            &ls_receiver_on_ntrn,
            DENOM_ATOM,
            DENOM_LS_ATOM_ON_STRIDE,
            HUB_STRIDE_CHANNEL.1,
            HUB_STRIDE_CHANNEL.0,
            500_000_000_000_u128,
        );
        let lp_forwarder_config = SinglePartyCovenantInstantiate::get_forwarder_config_interchain(
            &Addr::unchecked("not_used"),
            &Addr::unchecked("not_used"),
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            500_000_000_000_u128,
        );
        let remote_splitter = SinglePartyCovenantInstantiate::get_remote_splitter_config(
            NTRN_HUB_CHANNEL.0,
            DENOM_ATOM,
            1_000_000_000_000_u128,
            Decimal::bps(5000),
            Decimal::bps(5000),
        );
        let pool_price_config = SinglePartyCovenantInstantiate::get_pool_price_config(
            Decimal::from_str("1").unwrap(),
            Decimal::bps(5000),
        );
        let init_msg = SinglePartyCovenantInstantiate::default(
            &builder,
            ls_forwarder_config,
            lp_forwarder_config,
            remote_splitter,
            covenant_party,
            pooler_config,
            pool_price_config,
        );

        builder.contract_init2(
            builder.single_party_covenant_code_id,
            SINGLE_PARTY_COVENANT_SALT,
            &init_msg.msg,
            &[],
        );

        Self::build(
            builder,
            covenant_addr,
            party_receiver,
            party_receiver_on_ntrn,
            ls_receiver,
            pool_addr,
            lp_token_addr,
        )
    }

    pub fn new_with_xyk_pool() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr = builder.get_contract_addr(
            builder.single_party_covenant_code_id,
            SINGLE_PARTY_COVENANT_SALT,
        );

        // init astro pools
        let (pool_addr, lp_token_addr) = builder.init_astro_pool(
            astroport::factory::PairType::Xyk {},
            coin(10_000_000_000_000, DENOM_ATOM_ON_NTRN),
            coin(10_000_000_000_000, DENOM_LS_ATOM_ON_NTRN),
        );

        let party_receiver = builder.get_random_addr();
        let party_receiver_on_ntrn = builder.get_random_addr();
        let ls_receiver = builder.get_random_addr();
        let ls_receiver_on_ntrn = builder.get_random_addr();

        let mut pfm = BTreeMap::new();
        pfm.insert(
            DENOM_LS_ATOM_ON_NTRN.to_string(),
            covenant_utils::PacketForwardMiddlewareConfig {
                local_to_hop_chain_channel_id: NTRN_STRIDE_CHANNEL.0.to_string(),
                hop_to_destination_chain_channel_id: HUB_STRIDE_CHANNEL.1.to_string(),
                hop_chain_receiver_address: ls_receiver.to_string(),
            },
        );

        let covenant_party = SinglePartyCovenantInstantiate::get_covenant_party(
            &party_receiver,
            &party_receiver_on_ntrn,
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            1_000_000_000_000_u128,
            pfm,
        );

        let pooler_config = SinglePartyCovenantInstantiate::get_astro_pooler_config(
            DENOM_ATOM_ON_NTRN,
            DENOM_LS_ATOM_ON_NTRN,
            &pool_addr,
            astroport::factory::PairType::Xyk {},
            covenant_utils::SingleSideLpLimits {
                asset_a_limit: 10_000_000_u128.into(),
                asset_b_limit: 10_000_000_u128.into(),
            },
        );
        let ls_forwarder_config = SinglePartyCovenantInstantiate::get_forwarder_config_interchain(
            &ls_receiver,
            &ls_receiver_on_ntrn,
            DENOM_ATOM,
            DENOM_LS_ATOM_ON_STRIDE,
            HUB_STRIDE_CHANNEL.1,
            HUB_STRIDE_CHANNEL.0,
            500_000_000_000_u128,
        );
        let lp_forwarder_config = SinglePartyCovenantInstantiate::get_forwarder_config_interchain(
            &Addr::unchecked("not_used"),
            &Addr::unchecked("not_used"),
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            500_000_000_000_u128,
        );
        let remote_splitter = SinglePartyCovenantInstantiate::get_remote_splitter_config(
            NTRN_HUB_CHANNEL.0,
            DENOM_ATOM,
            1_000_000_000_000_u128,
            Decimal::bps(5000),
            Decimal::bps(5000),
        );
        let pool_price_config = SinglePartyCovenantInstantiate::get_pool_price_config(
            Decimal::from_str("1").unwrap(),
            Decimal::bps(5000),
        );
        let init_msg = SinglePartyCovenantInstantiate::default(
            &builder,
            ls_forwarder_config,
            lp_forwarder_config,
            remote_splitter,
            covenant_party,
            pooler_config,
            pool_price_config,
        );

        builder.contract_init2(
            builder.single_party_covenant_code_id,
            SINGLE_PARTY_COVENANT_SALT,
            &init_msg.msg,
            &[],
        );

        Self::build(
            builder,
            covenant_addr,
            party_receiver,
            party_receiver_on_ntrn,
            ls_receiver,
            pool_addr,
            lp_token_addr,
        )
    }
}

// helpers
impl Suite {
    pub fn get_and_fund_depositors(&mut self, a: Coin) -> Addr {
        while self.query_deposit_addr().is_err() {
            self.tick_clock_debug();
        }

        let depositor = self.query_deposit_addr().unwrap();

        self.app
            .send_tokens(self.fuacet.clone(), depositor.clone(), &[a])
            .unwrap();

        depositor
    }

    pub fn get_ica(&mut self, addr: Addr) -> Addr {
        while self
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                &addr,
                &covenant_remote_chain_splitter::msg::QueryMsg::IcaAddress {},
            )
            .is_err()
        {
            self.tick("Wait for ICA");
        }

        self.app
            .wrap()
            .query_wasm_smart::<Addr>(
                addr,
                &covenant_remote_chain_splitter::msg::QueryMsg::IcaAddress {},
            )
            .unwrap()
    }

    pub fn astro_swap(&mut self, coin: Coin) {
        self.app
            .execute_contract(
                self.fuacet.clone(),
                self.pool_addr.clone(),
                &astroport::pair::ExecuteMsg::Swap {
                    offer_asset: astroport::asset::Asset {
                        info: astroport::asset::AssetInfo::NativeToken {
                            denom: coin.denom.clone(),
                        },
                        amount: coin.amount,
                    },
                    ask_asset_info: None,
                    belief_price: None,
                    max_spread: Some(Decimal::bps(5000)),
                    to: None,
                },
                &vec![coin],
            )
            .unwrap();
    }
}
// queries
impl Suite {
    pub fn query_deposit_addr(&self) -> StdResult<Addr> {
        self.app.wrap().query_wasm_smart(
            self.covenant_addr.clone(),
            &covenant_single_party_pol::msg::QueryMsg::PartyDepositAddress {},
        )
    }
}
