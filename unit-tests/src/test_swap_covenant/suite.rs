use std::vec;

use cosmwasm_std::{coins, Addr, Coin, Decimal, StdResult};
use cw_multi_test::Executor;

use crate::setup::{
    base_suite::{BaseSuite, BaseSuiteMut},
    instantiates::swap_covenant::SwapCovenantInstantiate,
    suite_builder::SuiteBuilder,
    CustomApp, DENOM_ATOM, DENOM_ATOM_ON_NTRN, DENOM_NTRN, DENOM_OSMO, DENOM_OSMO_ON_NTRN,
    NTRN_HUB_CHANNEL, NTRN_OSMO_CHANNEL, SWAP_COVENANT_SALT,
};

pub(super) struct Suite {
    pub fuacet: Addr,
    pub admin: Addr,

    pub app: CustomApp,

    pub covenant_addr: Addr,
    pub clock_addr: Addr,
    pub holder_addr: Addr,
    pub splitter_addr: Addr,
    pub router_a_addr: Addr,
    pub router_b_addr: Addr,

    // The address that should receive the final amount of the swap
    pub party_a_receiver: Addr,
    pub party_b_receiver: Addr,
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
        party_a_receiver: Addr,
        party_b_receiver: Addr,
    ) -> Self {
        let clock_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                covenant_addr.clone(),
                &covenant_swap::msg::QueryMsg::ClockAddress {},
            )
            .unwrap();

        let holder_addr = builder
            .app
            .wrap()
            .query_wasm_smart(
                covenant_addr.clone(),
                &covenant_swap::msg::QueryMsg::HolderAddress {},
            )
            .unwrap();

        let router_a_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_swap::msg::QueryMsg::InterchainRouterAddress {
                    party: "party_a".to_string(),
                },
            )
            .unwrap();

        let router_b_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_swap::msg::QueryMsg::InterchainRouterAddress {
                    party: "party_b".to_string(),
                },
            )
            .unwrap();

        let splitter_addr = builder
            .app
            .wrap()
            .query_wasm_smart::<Addr>(
                covenant_addr.clone(),
                &covenant_swap::msg::QueryMsg::SplitterAddress {},
            )
            .unwrap();

        if let Ok(ibc_forwarder) = builder.app.wrap().query_wasm_smart::<Addr>(
            covenant_addr.clone(),
            &covenant_swap::msg::QueryMsg::IbcForwarderAddress {
                party: "party_a".to_string(),
            },
        ) {
            builder
                .app
                .send_tokens(
                    builder.fuacet.clone(),
                    ibc_forwarder,
                    &coins(2_000_000, DENOM_NTRN),
                )
                .unwrap();
        };

        if let Ok(ibc_forwarder) = builder.app.wrap().query_wasm_smart::<Addr>(
            covenant_addr.clone(),
            &covenant_swap::msg::QueryMsg::IbcForwarderAddress {
                party: "party_b".to_string(),
            },
        ) {
            builder
                .app
                .send_tokens(
                    builder.fuacet.clone(),
                    ibc_forwarder,
                    &coins(2_000_000, DENOM_NTRN),
                )
                .unwrap();
        };

        // fund routers
        if let Ok(router) = builder.app.wrap().query_wasm_smart::<Addr>(
            covenant_addr.clone(),
            &covenant_swap::msg::QueryMsg::InterchainRouterAddress {
                party: "party_a".to_string(),
            },
        ) {
            builder
                .app
                .send_tokens(builder.fuacet.clone(), router, &coins(400_000, DENOM_NTRN))
                .unwrap();
        };

        if let Ok(router) = builder.app.wrap().query_wasm_smart::<Addr>(
            covenant_addr.clone(),
            &covenant_swap::msg::QueryMsg::InterchainRouterAddress {
                party: "party_b".to_string(),
            },
        ) {
            builder
                .app
                .send_tokens(builder.fuacet.clone(), router, &coins(400_000, DENOM_NTRN))
                .unwrap();
        };

        Self {
            fuacet: builder.fuacet.clone(),
            admin: builder.admin.clone(),

            covenant_addr,
            clock_addr,
            holder_addr,
            splitter_addr,
            router_a_addr,
            router_b_addr,

            party_a_receiver,
            party_b_receiver,

            // Make sure its the last one, because build() consume the builder
            app: builder.build(),
        }
    }
}

impl Suite {
    pub fn new() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr =
            builder.get_contract_addr(builder.swap_covenant_code_id, SWAP_COVENANT_SALT);

        let party_a_receiver = builder.get_random_addr();
        let party_a_on_ntrn = builder.get_random_addr();
        let party_b_receiver = builder.get_random_addr();

        let recievers = vec![
            (&party_a_receiver, Decimal::bps(5000)),
            (&party_b_receiver, Decimal::bps(5000)),
        ];
        let splits = SwapCovenantInstantiate::get_split_custom(vec![
            (DENOM_ATOM_ON_NTRN, &recievers),
            (DENOM_NTRN, &recievers),
        ]);
        let party_a_config = SwapCovenantInstantiate::get_party_config_interchain(
            &party_a_receiver,
            &party_a_on_ntrn,
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            10_000_000_u128,
        );
        let party_b_config = SwapCovenantInstantiate::get_party_config_native(
            &party_b_receiver,
            DENOM_NTRN,
            10_000_000_u128,
        );
        let init_msg =
            SwapCovenantInstantiate::default(&builder, party_a_config, party_b_config, splits).msg;

        builder.contract_init2(
            builder.swap_covenant_code_id,
            SWAP_COVENANT_SALT,
            &init_msg,
            &[],
        );

        Self::build(builder, covenant_addr, party_a_receiver, party_b_receiver)
    }

    /// Init covenant with 2 native parties and 50% split of both denoms
    pub fn new_with_2_native_configs() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr =
            builder.get_contract_addr(builder.swap_covenant_code_id, SWAP_COVENANT_SALT);

        let party_a_receiver = builder.get_random_addr();
        let party_b_receiver = builder.get_random_addr();

        let recievers = vec![
            (&party_a_receiver, Decimal::bps(5000)),
            (&party_b_receiver, Decimal::bps(5000)),
        ];
        let splits = SwapCovenantInstantiate::get_split_custom(vec![
            (DENOM_ATOM, &recievers),
            (DENOM_NTRN, &recievers),
        ]);
        let party_a_config = SwapCovenantInstantiate::get_party_config_native(
            &party_a_receiver,
            DENOM_ATOM,
            10_000_000_u128,
        );
        let party_b_config = SwapCovenantInstantiate::get_party_config_native(
            &party_b_receiver,
            DENOM_NTRN,
            10_000_000_u128,
        );
        let init_msg =
            SwapCovenantInstantiate::default(&builder, party_a_config, party_b_config, splits).msg;

        builder.contract_init2(
            builder.swap_covenant_code_id,
            SWAP_COVENANT_SALT,
            &init_msg,
            &[],
        );

        Self::build(builder, covenant_addr, party_a_receiver, party_b_receiver)
    }

    pub fn new_with_2_interchain_configs() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr =
            builder.get_contract_addr(builder.swap_covenant_code_id, SWAP_COVENANT_SALT);

        let party_a_receiver = builder.get_random_addr();
        let party_a_on_ntrn = builder.get_random_addr();
        let party_b_receiver = builder.get_random_addr();
        let party_b_on_ntrn = builder.get_random_addr();

        let recievers = vec![
            (&party_a_receiver, Decimal::bps(5000)),
            (&party_b_receiver, Decimal::bps(5000)),
        ];
        let splits = SwapCovenantInstantiate::get_split_custom(vec![
            (DENOM_ATOM_ON_NTRN, &recievers),
            (DENOM_OSMO_ON_NTRN, &recievers),
        ]);
        let party_a_config = SwapCovenantInstantiate::get_party_config_interchain(
            &party_a_receiver,
            &party_a_on_ntrn,
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            10_000_000_u128,
        );
        let party_b_config = SwapCovenantInstantiate::get_party_config_interchain(
            &party_b_receiver,
            &party_b_on_ntrn,
            DENOM_OSMO,
            DENOM_OSMO_ON_NTRN,
            NTRN_OSMO_CHANNEL.0,
            NTRN_OSMO_CHANNEL.1,
            10_000_000_u128,
        );
        let init_msg =
            SwapCovenantInstantiate::default(&builder, party_a_config, party_b_config, splits).msg;

        builder.contract_init2(
            builder.swap_covenant_code_id,
            SWAP_COVENANT_SALT,
            &init_msg,
            &[],
        );

        Self::build(builder, covenant_addr, party_a_receiver, party_b_receiver)
    }

    pub fn new_with_100_percent_split() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr =
            builder.get_contract_addr(builder.swap_covenant_code_id, SWAP_COVENANT_SALT);

        let party_a_receiver = builder.get_random_addr();
        let party_a_on_ntrn = builder.get_random_addr();
        let party_b_receiver = builder.get_random_addr();
        let party_b_on_ntrn = builder.get_random_addr();

        let recievers_1 = vec![
            (&party_a_receiver, Decimal::one()),
            (&party_b_receiver, Decimal::zero()),
        ];
        let recievers_2 = vec![
            (&party_b_receiver, Decimal::one()),
            (&party_a_receiver, Decimal::zero()),
        ];
        let splits = SwapCovenantInstantiate::get_split_custom(vec![
            (DENOM_ATOM_ON_NTRN, &recievers_2),
            (DENOM_OSMO_ON_NTRN, &recievers_1),
        ]);
        let party_a_config = SwapCovenantInstantiate::get_party_config_interchain(
            &party_a_receiver,
            &party_a_on_ntrn,
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            10_000_000_u128,
        );
        let party_b_config = SwapCovenantInstantiate::get_party_config_interchain(
            &party_b_receiver,
            &party_b_on_ntrn,
            DENOM_OSMO,
            DENOM_OSMO_ON_NTRN,
            NTRN_OSMO_CHANNEL.0,
            NTRN_OSMO_CHANNEL.1,
            10_000_000_u128,
        );
        let init_msg =
            SwapCovenantInstantiate::default(&builder, party_a_config, party_b_config, splits).msg;

        builder.contract_init2(
            builder.swap_covenant_code_id,
            SWAP_COVENANT_SALT,
            &init_msg,
            &[],
        );

        Self::build(builder, covenant_addr, party_a_receiver, party_b_receiver)
    }

    pub fn new_with_fallback() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr =
            builder.get_contract_addr(builder.swap_covenant_code_id, SWAP_COVENANT_SALT);

        let party_a_receiver = builder.get_random_addr();
        let party_b_receiver = builder.get_random_addr();

        let recievers = vec![
            (&party_a_receiver, Decimal::bps(5000)),
            (&party_b_receiver, Decimal::bps(5000)),
        ];
        let splits = SwapCovenantInstantiate::get_split_custom(vec![
            (DENOM_ATOM, &recievers),
            (DENOM_NTRN, &recievers),
        ]);
        let party_a_config = SwapCovenantInstantiate::get_party_config_native(
            &party_a_receiver,
            DENOM_ATOM,
            10_000_000_u128,
        );
        let party_b_config = SwapCovenantInstantiate::get_party_config_native(
            &party_b_receiver,
            DENOM_NTRN,
            10_000_000_u128,
        );
        let mut init_msg =
            SwapCovenantInstantiate::default(&builder, party_a_config, party_b_config, splits);
        init_msg.with_fallback_split(&recievers);

        builder.contract_init2(
            builder.swap_covenant_code_id,
            SWAP_COVENANT_SALT,
            &init_msg.msg,
            &[],
        );

        Self::build(builder, covenant_addr, party_a_receiver, party_b_receiver)
    }

    pub fn new_with_interchain_fallback() -> Self {
        let mut builder = SuiteBuilder::new();

        let covenant_addr =
            builder.get_contract_addr(builder.swap_covenant_code_id, SWAP_COVENANT_SALT);

        let party_a_receiver = builder.get_random_addr();
        let party_a_on_ntrn = builder.get_random_addr();
        let party_b_receiver = builder.get_random_addr();
        let party_b_on_ntrn = builder.get_random_addr();

        let recievers = vec![
            (&party_a_receiver, Decimal::bps(5000)),
            (&party_b_receiver, Decimal::bps(5000)),
        ];
        let splits = SwapCovenantInstantiate::get_split_custom(vec![
            (DENOM_ATOM_ON_NTRN, &recievers),
            (DENOM_OSMO_ON_NTRN, &recievers),
        ]);
        let party_a_config = SwapCovenantInstantiate::get_party_config_interchain(
            &party_a_receiver,
            &party_a_on_ntrn,
            DENOM_ATOM,
            DENOM_ATOM_ON_NTRN,
            NTRN_HUB_CHANNEL.0,
            NTRN_HUB_CHANNEL.1,
            10_000_000_u128,
        );
        let party_b_config = SwapCovenantInstantiate::get_party_config_interchain(
            &party_b_receiver,
            &party_b_on_ntrn,
            DENOM_OSMO,
            DENOM_OSMO_ON_NTRN,
            NTRN_OSMO_CHANNEL.0,
            NTRN_OSMO_CHANNEL.1,
            10_000_000_u128,
        );
        let mut init_msg =
            SwapCovenantInstantiate::default(&builder, party_a_config, party_b_config, splits);
        init_msg.with_fallback_split(&recievers);

        builder.contract_init2(
            builder.swap_covenant_code_id,
            SWAP_COVENANT_SALT,
            &init_msg.msg,
            &[],
        );

        Self::build(builder, covenant_addr, party_a_receiver, party_b_receiver)
    }
}

// helpers
impl Suite {
    pub fn get_and_fund_depositors(&mut self, a: Coin, b: Coin) -> (Addr, Addr) {
        while self.query_deposit_addr("party_a").is_err() {
            self.tick("Wait depositor_a is ready");
        }

        while self.query_deposit_addr("party_b").is_err() {
            self.tick("Wait depositor_b is ready");
        }

        let depositor_a = self.query_deposit_addr("party_a").unwrap();
        let depositor_b = self.query_deposit_addr("party_b").unwrap();

        self.app
            .send_tokens(self.fuacet.clone(), depositor_a.clone(), &vec![a])
            .unwrap();
        self.app
            .send_tokens(self.fuacet.clone(), depositor_b.clone(), &vec![b])
            .unwrap();

        (depositor_a, depositor_b)
    }
}
// queries
impl Suite {
    pub fn query_deposit_addr(&self, party: &str) -> StdResult<Addr> {
        self.app.wrap().query_wasm_smart(
            self.covenant_addr.clone(),
            &covenant_swap::msg::QueryMsg::PartyDepositAddress {
                party: party.to_string(),
            },
        )
    }
}
