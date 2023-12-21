package ibc_test

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"
	"testing"
	"time"

	"github.com/cosmos/cosmos-sdk/crypto/keyring"

	ibctest "github.com/strangelove-ventures/interchaintest/v4"
	"github.com/strangelove-ventures/interchaintest/v4/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v4/ibc"
	"github.com/strangelove-ventures/interchaintest/v4/relayer"
	"github.com/strangelove-ventures/interchaintest/v4/testreporter"
	"github.com/strangelove-ventures/interchaintest/v4/testutil"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap"
	"go.uber.org/zap/zaptest"
)

// PARTY_B
const neutronContributionAmount uint64 = 50_000_000_000 // in untrn
var hubNeutronIbcDenom string

// sets up and tests a two party pol between hub and osmo facilitated by neutron
func TestTwoPartyNativePartyPol(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping in short mode")
	}

	ctx := context.Background()

	// Modify the the timeout_commit in the config.toml node files
	// to reduce the block commit times. This speeds up the tests
	// by about 35%
	configFileOverrides := make(map[string]any)
	configTomlOverrides := make(testutil.Toml)
	consensus := make(testutil.Toml)
	consensus["timeout_commit"] = "1s"
	configTomlOverrides["consensus"] = consensus
	configFileOverrides["config/config.toml"] = configTomlOverrides

	// Chain Factory
	cf := ibctest.NewBuiltinChainFactory(zaptest.NewLogger(t, zaptest.Level(zap.WarnLevel)), []*ibctest.ChainSpec{
		{Name: "gaia", Version: "v9.1.0", ChainConfig: ibc.ChainConfig{
			GasAdjustment:       1.3,
			GasPrices:           "0.0atom",
			ModifyGenesis:       setupGaiaGenesis(getDefaultInterchainGenesisMessages()),
			ConfigFileOverrides: configFileOverrides,
		}},
		{
			ChainConfig: ibc.ChainConfig{
				Type:    "cosmos",
				Name:    "neutron",
				ChainID: "neutron-2",
				Images: []ibc.DockerImage{
					{
						Repository: "ghcr.io/strangelove-ventures/heighliner/neutron",
						Version:    "v2.0.0",
						UidGid:     "1025:1025",
					},
				},
				Bin:            "neutrond",
				Bech32Prefix:   "neutron",
				Denom:          nativeNtrnDenom,
				GasPrices:      "0.0untrn,0.0uatom",
				GasAdjustment:  1.3,
				TrustingPeriod: "1197504s",
				NoHostMount:    false,
				ModifyGenesis: setupNeutronGenesis(
					"0.05",
					[]string{nativeNtrnDenom},
					[]string{nativeAtomDenom},
					getDefaultNeutronInterchainGenesisMessages(),
				),
				ConfigFileOverrides: configFileOverrides,
			},
		},
		{
			Name:    "osmosis",
			Version: "v14.0.0",
			ChainConfig: ibc.ChainConfig{
				Type:         "cosmos",
				Bin:          "osmosisd",
				Bech32Prefix: "osmo",
				Denom:        nativeOsmoDenom,
				ModifyGenesis: setupOsmoGenesis(
					append(getDefaultInterchainGenesisMessages(), "/ibc.applications.interchain_accounts.v1.InterchainAccount"),
				),
				GasPrices:     "0.0uosmo",
				GasAdjustment: 1.3,
				Images: []ibc.DockerImage{
					{
						Repository: "ghcr.io/strangelove-ventures/heighliner/osmosis",
						Version:    "v14.0.0",
						UidGid:     "1025:1025",
					},
				},
				TrustingPeriod:      "336h",
				NoHostMount:         false,
				ConfigFileOverrides: configFileOverrides,
			},
		},
	})

	chains, err := cf.Chains(t.Name())
	require.NoError(t, err)

	// We have three chains
	atom, neutron, osmosis := chains[0], chains[1], chains[2]
	cosmosAtom, cosmosNeutron, cosmosOsmosis := atom.(*cosmos.CosmosChain), neutron.(*cosmos.CosmosChain), osmosis.(*cosmos.CosmosChain)

	// Relayer Factory
	client, network := ibctest.DockerSetup(t)
	r := ibctest.NewBuiltinRelayerFactory(
		ibc.CosmosRly,
		zaptest.NewLogger(t, zaptest.Level(zap.InfoLevel)),
		relayer.CustomDockerImage("ghcr.io/cosmos/relayer", "v2.4.0", "1000:1000"),
		relayer.RelayerOptionExtraStartFlags{Flags: []string{"-p", "events", "-b", "100", "-d", "--log-format", "console"}},
	).Build(t, client, network)

	// Prep Interchain
	ic := ibctest.NewInterchain().
		AddChain(cosmosAtom).
		AddChain(cosmosNeutron).
		AddChain(cosmosOsmosis).
		AddRelayer(r, "relayer").
		AddProviderConsumerLink(ibctest.ProviderConsumerLink{
			Provider: cosmosAtom,
			Consumer: cosmosNeutron,
			Relayer:  r,
			Path:     gaiaNeutronICSPath,
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  cosmosAtom,
			Chain2:  cosmosNeutron,
			Relayer: r,
			Path:    gaiaNeutronIBCPath,
		})

	// Log location
	f, err := ibctest.CreateLogFile(fmt.Sprintf("%d.json", time.Now().Unix()))
	require.NoError(t, err)
	// Reporter/logs
	rep := testreporter.NewReporter(f)
	eRep := rep.RelayerExecReporter(t)

	// Build interchain
	require.NoError(
		t,
		ic.Build(ctx, eRep, ibctest.InterchainBuildOptions{
			TestName:          t.Name(),
			Client:            client,
			NetworkID:         network,
			BlockDatabaseFile: ibctest.DefaultBlockDatabaseFilepath(),
			SkipPathCreation:  true,
		}),
		"failed to build interchain")

	testCtx := &TestContext{
		Neutron:                   cosmosNeutron,
		Hub:                       cosmosAtom,
		Osmosis:                   cosmosOsmosis,
		OsmoClients:               []*ibc.ClientOutput{},
		GaiaClients:               []*ibc.ClientOutput{},
		NeutronClients:            []*ibc.ClientOutput{},
		GaiaConnections:           []*ibc.ConnectionOutput{},
		NeutronConnections:        []*ibc.ConnectionOutput{},
		NeutronTransferChannelIds: make(map[string]string),
		GaiaTransferChannelIds:    make(map[string]string),
		GaiaIcsChannelIds:         make(map[string]string),
		NeutronIcsChannelIds:      make(map[string]string),
		t:                         t,
		ctx:                       ctx,
	}

	testCtx.skipBlocks(5)

	t.Run("generate IBC paths", func(t *testing.T) {
		generatePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosNeutron.Config().ChainID, gaiaNeutronIBCPath)
		generatePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosAtom.Config().ChainID, gaiaNeutronICSPath)
	})

	t.Run("setup neutron-gaia ICS", func(t *testing.T) {
		generateClient(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)
		neutronClients := testCtx.getChainClients(cosmosNeutron.Config().Name)
		atomClients := testCtx.getChainClients(cosmosAtom.Config().Name)

		err = r.UpdatePath(ctx, eRep, gaiaNeutronICSPath, ibc.PathUpdateOptions{
			SrcClientID: &neutronClients[0].ClientID,
			DstClientID: &atomClients[0].ClientID,
		})
		require.NoError(t, err)

		atomNeutronICSConnectionId, neutronAtomICSConnectionId = generateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)

		generateICSChannel(t, ctx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)

		createValidator(t, ctx, r, eRep, atom, neutron)
		testCtx.skipBlocks(2)
	})

	t.Run("setup IBC interchain clients, connections, and links", func(t *testing.T) {
		generateClient(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		atomNeutronIBCConnId, neutronAtomIBCConnId = generateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		linkPath(t, ctx, r, eRep, cosmosAtom, cosmosNeutron, gaiaNeutronIBCPath)
		testCtx.skipBlocks(10)
	})

	// Start the relayer and clean it up when the test ends.
	err = r.StartRelayer(ctx, eRep, gaiaNeutronICSPath, gaiaNeutronIBCPath)
	require.NoError(t, err, "failed to start relayer with given paths")
	t.Cleanup(func() {
		err = r.StopRelayer(ctx, eRep)
		if err != nil {
			t.Logf("failed to stop relayer: %s", err)
		}
	})
	testCtx.skipBlocks(2)

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron)
	gaiaUser, neutronUser := users[0], users[1]
	_, _ = gaiaUser, neutronUser
	hubNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]
	osmoNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]

	// rqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	// sideBasedRqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	happyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	// sideBasedHappyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	// neutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]

	// rqCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	// sideBasedRqCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	happyCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	// sideBasedHappyCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	testCtx.skipBlocks(5)

	t.Run("determine ibc channels", func(t *testing.T) {
		neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)

		// Find all pairwise channels
		getPairwiseTransferChannelIds(testCtx, gaiaChannelInfo, neutronChannelInfo, atomNeutronIBCConnId, neutronAtomIBCConnId, cosmosAtom.Config().Name, neutron.Config().Name)
		getPairwiseCCVChannelIds(testCtx, gaiaChannelInfo, neutronChannelInfo, atomNeutronICSConnectionId, neutronAtomICSConnectionId, cosmosAtom.Config().Name, cosmosNeutron.Config().Name)
	})

	t.Run("determine ibc denoms", func(t *testing.T) {
		// We can determine the ibc denoms of:
		// 1. ATOM on Neutron
		neutronAtomIbcDenom = testCtx.getIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
			nativeAtomDenom,
		)
		// 2. neutron => hub
		hubNeutronIbcDenom = testCtx.getIbcDenom(
			testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
			cosmosNeutron.Config().Denom,
		)
		println("hub neutron ibc denom: ", hubNeutronIbcDenom)
		println("neutron atom ibc denom: ", neutronAtomIbcDenom)
		println("atom denom: ", nativeAtomDenom)
		println("neutron denom: ", cosmosNeutron.Config().Denom)
	})

	t.Run("two party pol covenant setup", func(t *testing.T) {
		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_two_party_pol.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const interchainRouterContractPath = "wasms/covenant_interchain_router.wasm"
		const nativeRouterContractPath = "wasms/covenant_native_router.wasm"
		const ibcForwarderContractPath = "wasms/covenant_ibc_forwarder.wasm"
		const holderContractPath = "wasms/covenant_two_party_pol_holder.wasm"
		const liquidPoolerPath = "wasms/covenant_astroport_liquid_pooler.wasm"

		// After storing on Neutron, we will receive a code id
		// We parse all the subcontracts into uint64
		// The will be required when we instantiate the covenant.
		var clockCodeId uint64
		var interchainRouterCodeId uint64
		var nativeRouterCodeId uint64
		var ibcForwarderCodeId uint64
		var holderCodeId uint64
		var lperCodeId uint64
		var covenantCodeId uint64
		// var covenantRqCodeId uint64
		// var covenantSideBasedRqCodeId uint64

		t.Run("deploy covenant contracts", func(t *testing.T) {
			// something was going wrong with instantiating the same code twice,
			// hence this weird workaround
			covenantCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, covenantContractPath)
			// covenantRqCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, covenantContractPath)
			// covenantSideBasedRqCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, covenantContractPath)

			// store clock and get code id
			clockCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, clockContractPath)

			// store routers and get code id
			interchainRouterCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, interchainRouterContractPath)
			nativeRouterCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, nativeRouterContractPath)

			// store forwarder and get code id
			ibcForwarderCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, ibcForwarderContractPath)

			// store lper, get code
			lperCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, liquidPoolerPath)

			// store holder and get code id
			holderCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, holderContractPath)

			testCtx.skipBlocks(5)
		})

		t.Run("deploy astroport contracts", func(t *testing.T) {
			stablePairCodeId := testCtx.storeContract(cosmosNeutron, neutronUser, "wasms/astroport_pair_stable.wasm")
			factoryCodeId := testCtx.storeContract(cosmosNeutron, neutronUser, "wasms/astroport_factory.wasm")
			whitelistCodeId := testCtx.storeContract(cosmosNeutron, neutronUser, "wasms/astroport_whitelist.wasm")
			tokenCodeId := testCtx.storeContract(cosmosNeutron, neutronUser, "wasms/astroport_token.wasm")
			coinRegistryCodeId := testCtx.storeContract(cosmosNeutron, neutronUser, "wasms/astroport_native_coin_registry.wasm")

			t.Run("astroport token", func(t *testing.T) {
				msg := NativeTokenInstantiateMsg{
					Name:            "nativetoken",
					Symbol:          "ntk",
					Decimals:        5,
					InitialBalances: []Cw20Coin{},
					Mint:            nil,
					Marketing:       nil,
				}
				str, _ := json.Marshal(msg)

				tokenAddress, err = cosmosNeutron.InstantiateContract(
					ctx, neutronUser.KeyName, strconv.FormatUint(tokenCodeId, 10), string(str), true)
				require.NoError(t, err, "Failed to instantiate nativetoken")
				println("astroport token: ", tokenAddress)
			})

			t.Run("whitelist", func(t *testing.T) {
				msg := WhitelistInstantiateMsg{
					Admins:  []string{neutronUser.Bech32Address(neutron.Config().Bech32Prefix)},
					Mutable: false,
				}
				str, _ := json.Marshal(msg)

				whitelistAddress, err = cosmosNeutron.InstantiateContract(
					ctx, neutronUser.KeyName, strconv.FormatUint(whitelistCodeId, 10), string(str), true)
				require.NoError(t, err, "Failed to instantiate Whitelist")
				println("astroport whitelist: ", whitelistAddress)

			})

			t.Run("native coins registry", func(t *testing.T) {
				msg := NativeCoinRegistryInstantiateMsg{
					Owner: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				}
				str, _ := json.Marshal(msg)

				nativeCoinRegistryAddress, err := cosmosNeutron.InstantiateContract(
					ctx, neutronUser.KeyName, strconv.FormatUint(coinRegistryCodeId, 10), string(str), true)
				require.NoError(t, err, "Failed to instantiate NativeCoinRegistry")
				coinRegistryAddress = nativeCoinRegistryAddress
				println("astroport native coins registry: ", coinRegistryAddress)
			})

			t.Run("add coins to registry", func(t *testing.T) {
				// Add ibc native tokens for uosmo and uatom to the native coin registry
				// each of these tokens has a precision of 6
				addMessage := fmt.Sprintf(
					`{"add":{"native_coins":[["%s",6],["%s",6]]}}`,
					neutronAtomIbcDenom,
					cosmosNeutron.Config().Denom)
				_, err = cosmosNeutron.ExecuteContract(ctx, neutronUser.KeyName, coinRegistryAddress, addMessage)
				require.NoError(t, err, err)
				testCtx.skipBlocks(2)
			})

			t.Run("factory", func(t *testing.T) {
				factoryAddress = testCtx.instantiateAstroportFactory(
					stablePairCodeId, tokenCodeId, whitelistCodeId, factoryCodeId, coinRegistryAddress, neutronUser)
				println("astroport factory: ", factoryAddress)
				testCtx.skipBlocks(2)
			})

			t.Run("create pair on factory", func(t *testing.T) {
				testCtx.createAstroportFactoryPair(3, cosmosNeutron.Config().Denom, neutronAtomIbcDenom, factoryAddress, neutronUser, keyring.BackendTest)
			})
		})

		t.Run("add liquidity to the atom-neutron stableswap pool", func(t *testing.T) {
			liquidityTokenAddress, stableswapAddress = testCtx.queryAstroLpTokenAndStableswapAddress(
				factoryAddress, cosmosNeutron.Config().Denom, neutronAtomIbcDenom)
			// set up the pool with 1:10 ratio of atom/osmo
			_, err := atom.SendIBCTransfer(ctx,
				testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				gaiaUser.KeyName,
				ibc.WalletAmount{
					Address: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
					Denom:   cosmosAtom.Config().Denom,
					Amount:  int64(atomContributionAmount),
				},
				ibc.TransferOptions{})
			require.NoError(t, err)

			testCtx.skipBlocks(2)

			testCtx.provideAstroportLiquidity(
				neutronAtomIbcDenom, cosmosNeutron.Config().Denom, atomContributionAmount, neutronContributionAmount, neutronUser, stableswapAddress)

			testCtx.skipBlocks(2)
			testCtx.queryLpTokenBalance(liquidityTokenAddress, neutronUser.Bech32Address(neutron.Config().Bech32Prefix))
		})

		t.Run("two party POL happy path", func(t *testing.T) {
			var depositBlock Block
			var lockupBlock Block

			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "10000", // sec
					IbcTransferTimeout: "10000", // sec
				}

				currentHeight := testCtx.getNeutronHeight()
				depositBlock = Block(currentHeight + 180)
				lockupBlock = Block(currentHeight + 180)

				lockupConfig := Expiration{
					AtHeight: &lockupBlock,
				}
				depositDeadline := Expiration{
					AtHeight: &depositBlock,
				}
				presetIbcFee := PresetIbcFee{
					AckFee:     "100000",
					TimeoutFee: "100000",
				}

				atomCoin := Coin{
					Denom:  cosmosAtom.Config().Denom,
					Amount: strconv.FormatUint(atomContributionAmount, 10),
				}

				neutronCoin := Coin{
					Denom:  cosmosNeutron.Config().Denom,
					Amount: strconv.FormatUint(neutronContributionAmount, 10),
				}

				hubReceiverAddr := happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				neutronReceiverAddr := happyCaseNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix)

				println("hub receiver address: ", hubReceiverAddr)

				partyAConfig := InterchainCovenantParty{
					Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					NativeDenom:               neutronAtomIbcDenom,
					RemoteChainDenom:          "uatom",
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyReceiverAddr:         hubReceiverAddr,
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
					Contribution:              atomCoin,
				}
				partyBConfig := NativeCovenantParty{
					Addr:              neutronReceiverAddr,
					NativeDenom:       cosmosNeutron.Config().Denom,
					PartyReceiverAddr: neutronReceiverAddr,
					Contribution:      neutronCoin,
				}
				codeIds := ContractCodeIds{
					IbcForwarderCode:     ibcForwarderCodeId,
					InterchainRouterCode: interchainRouterCodeId,
					NativeRouterCode:     nativeRouterCodeId,
					ClockCode:            clockCodeId,
					HolderCode:           holderCodeId,
					LiquidPoolerCode:     lperCodeId,
				}

				ragequitTerms := RagequitTerms{
					Penalty: "0.1",
				}

				ragequitConfig := RagequitConfig{
					Enabled: &ragequitTerms,
				}

				poolAddress := stableswapAddress
				pairType := PairType{
					Stable: struct{}{},
				}

				denomSplits := []DenomSplit{
					{
						Denom: neutronAtomIbcDenom,
						Type: SplitType{
							Custom: SplitConfig{
								Receivers: map[string]string{
									hubReceiverAddr:     "0.5",
									neutronReceiverAddr: "0.5",
								},
							},
						},
					},
					{
						Denom: cosmosNeutron.Config().Denom,
						Type: SplitType{
							Custom: SplitConfig{
								Receivers: map[string]string{
									hubReceiverAddr:     "0.5",
									neutronReceiverAddr: "0.5",
								},
							},
						},
					},
				}

				covenantMsg := CovenantInstantiateMsg{
					Label:           "two-party-pol-covenant-happy",
					Timeouts:        timeouts,
					PresetIbcFee:    presetIbcFee,
					ContractCodeIds: codeIds,
					LockupConfig:    lockupConfig,
					PartyAConfig: CovenantPartyConfig{
						Interchain: &partyAConfig,
					},
					PartyBConfig: CovenantPartyConfig{
						Native: &partyBConfig,
					},
					PoolAddress:              poolAddress,
					RagequitConfig:           &ragequitConfig,
					DepositDeadline:          depositDeadline,
					PartyAShare:              "50",
					PartyBShare:              "50",
					ExpectedPoolRatio:        "0.1",
					AcceptablePoolRatioDelta: "0.09",
					CovenantType:             "share",
					PairType:                 pairType,
					Splits:                   denomSplits,
					FallbackSplit:            nil,
				}

				covenantAddress = testCtx.manualInstantiate(covenantCodeId, covenantMsg, neutronUser, keyring.BackendTest)

				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.queryClockAddress(covenantAddress)
				holderAddress = testCtx.queryHolderAddress(covenantAddress)
				liquidPoolerAddress = testCtx.queryLiquidPoolerAddress(covenantAddress)
				partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
				partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
				partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
				partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
			})

			t.Run("fund contracts with neutron", func(t *testing.T) {
				addrs := []string{
					clockAddress,
					partyARouterAddress,
					partyBRouterAddress,
					holderAddress,
					liquidPoolerAddress,
				}
				if partyAIbcForwarderAddress != "" {
					addrs = append(addrs, partyAIbcForwarderAddress)
				}
				if partyBIbcForwarderAddress != "" {
					addrs = append(addrs, partyBIbcForwarderAddress)
				}
				testCtx.fundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				for {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
					// forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)
					println("forwarderAState: ", forwarderAState)
					// println("forwarderBState: ", forwarderBState)
					if forwarderAState == "ica_created" {
						partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
						partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
				testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosNeutron, happyCaseNeutronAccount, int64(neutronContributionAmount))
				testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, happyCaseHubAccount, int64(atomContributionAmount))

				testCtx.skipBlocks(3)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					holderNeutronBal := testCtx.queryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
					holderAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
					holderState := testCtx.queryContractState(holderAddress)
					println("holder ibc atom balance: ", holderAtomBal)
					println("holder neutron balance: ", holderNeutronBal)
					println("holder state: ", holderState)

					if holderAtomBal == atomContributionAmount && holderNeutronBal == neutronContributionAmount {
						println("holder received atom & neutron")
						break
					} else if holderState == "active" {
						println("holder: active")
						break
					}
				}
			})

			t.Run("tick until holder sends funds to LiquidPooler and receives LP tokens in return", func(t *testing.T) {
				for {
					if testCtx.queryLpTokenBalance(liquidityTokenAddress, holderAddress) == 0 {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("tick until holder expires", func(t *testing.T) {
				for {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					holderState := testCtx.queryContractState(holderAddress)
					println("holder state: ", holderState)

					if holderState == "expired" {
						break
					}
				}
			})

			t.Run("party A claims and router receives the funds", func(t *testing.T) {
				routerNeutronBalA := testCtx.queryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
				routerAtomBalA := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
				println("routerAtomBalA: ", routerAtomBalA)
				println("routerNeutronBalA: ", routerNeutronBalA)
				testCtx.skipBlocks(10)
				testCtx.holderClaim(holderAddress, hubNeutronAccount, keyring.BackendTest)
				testCtx.skipBlocks(5)
				for {
					routerNeutronBalA := testCtx.queryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
					routerAtomBalA := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
					println("routerAtomBalA: ", routerAtomBalA)
					println("routerNeutronBalA: ", routerNeutronBalA)
					if routerAtomBalA != 0 && routerNeutronBalA != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until party A claim is distributed", func(t *testing.T) {
				routerNeutronBalA := testCtx.queryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
				routerAtomBalA := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
				println("routerAtomBalA: ", routerAtomBalA)
				println("routerNeutronBalA: ", routerNeutronBalA)
				for {
					atomBalPartyA, _ := cosmosAtom.GetBalance(
						ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom)
					neutronBalPartyA, _ := cosmosAtom.GetBalance(
						ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), hubNeutronIbcDenom)

					println("party A atom bal: ", atomBalPartyA)
					println("party A neutron bal: ", neutronBalPartyA)

					if atomBalPartyA != 0 && neutronBalPartyA != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("party B claims and router receives the funds", func(t *testing.T) {
				routerNeutronBalB := testCtx.queryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
				routerAtomBalB := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
				println("routerAtomBalB: ", routerAtomBalB)
				println("routerNeutronBalB: ", routerNeutronBalB)
				testCtx.holderClaim(holderAddress, osmoNeutronAccount, keyring.BackendTest)
				for {
					routerNeutronBalB := testCtx.queryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
					routerAtomBalB := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
					println("routerAtomBalB: ", routerAtomBalB)
					println("routerNeutronBalB: ", routerNeutronBalB)
					if routerAtomBalB != 0 && routerNeutronBalB != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					neutronBalPartyA, _ := cosmosAtom.GetBalance(
						ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), hubNeutronIbcDenom)
					neutronBalPartyB, _ := cosmosNeutron.GetBalance(
						ctx, happyCaseNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix), cosmosNeutron.Config().Denom)
					atomBalPartyA, _ := cosmosAtom.GetBalance(
						ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom)
					atomBalPartyB, _ := cosmosNeutron.GetBalance(
						ctx, happyCaseNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix), neutronAtomIbcDenom)

					println("party A neutron bal: ", neutronBalPartyA)
					println("party A atom bal: ", atomBalPartyA)
					println("party B neutron bal: ", neutronBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					if atomBalPartyA != 0 && neutronBalPartyB != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})
		})

		// t.Run("two party share based POL ragequit path", func(t *testing.T) {

		// 	t.Run("instantiate covenant", func(t *testing.T) {
		// 		timeouts := Timeouts{
		// 			IcaTimeout:         "100", // sec
		// 			IbcTransferTimeout: "100", // sec
		// 		}

		// 		currentHeight := testCtx.getNeutronHeight()
		// 		depositBlock := Block(currentHeight + 200)
		// 		lockupBlock := Block(currentHeight + 300)

		// 		lockupConfig := Expiration{
		// 			AtHeight: &lockupBlock,
		// 		}
		// 		depositDeadline := Expiration{
		// 			AtHeight: &depositBlock,
		// 		}
		// 		presetIbcFee := PresetIbcFee{
		// 			AckFee:     "10000",
		// 			TimeoutFee: "10000",
		// 		}

		// 		atomCoin := Coin{
		// 			Denom:  cosmosAtom.Config().Denom,
		// 			Amount: strconv.FormatUint(atomContributionAmount, 10),
		// 		}

		// 		osmoCoin := Coin{
		// 			Denom:  cosmosOsmosis.Config().Denom,
		// 			Amount: strconv.FormatUint(osmoContributionAmount, 10),
		// 		}
		// 		hubReceiverAddr := rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
		// 		osmoReceiverAddr := rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)
		// 		partyAConfig := InterchainCovenantParty{
		// 			RemoteChainDenom:          "uatom",
		// 			PartyReceiverAddr:         hubReceiverAddr,
		// 			Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			Contribution:              atomCoin,
		// 			NativeDenom:               neutronAtomIbcDenom,
		// 			PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
		// 			PartyChainConnectionId:    neutronAtomIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 		}
		// 		partyBConfig := InterchainCovenantParty{
		// 			RemoteChainDenom:          "uosmo",
		// 			PartyReceiverAddr:         osmoReceiverAddr,
		// 			Addr:                      osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			Contribution:              osmoCoin,
		// 			NativeDenom:               neutronOsmoIbcDenom,
		// 			PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
		// 			PartyChainConnectionId:    neutronOsmosisIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 		}
		// 		codeIds := ContractCodeIds{
		// 			IbcForwarderCode:     ibcForwarderCodeId,
		// 			InterchainRouterCode: interchainRouterCodeId,
		// 			NativeRouterCode:     nativeRouterCodeId,
		// 			ClockCode:            clockCodeId,
		// 			HolderCode:           holderCodeId,
		// 			LiquidPoolerCode:     lperCodeId,
		// 		}

		// 		ragequitTerms := RagequitTerms{
		// 			Penalty: "0.1",
		// 		}

		// 		ragequitConfig := RagequitConfig{
		// 			Enabled: &ragequitTerms,
		// 		}

		// 		poolAddress := stableswapAddress
		// 		pairType := PairType{
		// 			Stable: struct{}{},
		// 		}

		// 		covenantMsg := CovenantInstantiateMsg{
		// 			Label:                    "two-party-pol-covenant-ragequit",
		// 			Timeouts:                 timeouts,
		// 			PresetIbcFee:             presetIbcFee,
		// 			ContractCodeIds:          codeIds,
		// 			LockupConfig:             lockupConfig,
		// 			PartyAConfig:             CovenantPartyConfig{Interchain: &partyAConfig},
		// 			PartyBConfig:             CovenantPartyConfig{Interchain: &partyBConfig},
		// 			PoolAddress:              poolAddress,
		// 			RagequitConfig:           &ragequitConfig,
		// 			DepositDeadline:          depositDeadline,
		// 			PartyAShare:              "50",
		// 			PartyBShare:              "50",
		// 			ExpectedPoolRatio:        "0.1",
		// 			AcceptablePoolRatioDelta: "0.09",
		// 			CovenantType:             "share",
		// 			PairType:                 pairType,
		// 			Splits: []DenomSplit{
		// 				{
		// 					Denom: neutronAtomIbcDenom,
		// 					Type: SplitType{
		// 						Custom: SplitConfig{
		// 							Receivers: map[string]string{
		// 								hubReceiverAddr:  "0.5",
		// 								osmoReceiverAddr: "0.5",
		// 							},
		// 						},
		// 					},
		// 				},
		// 				{
		// 					Denom: neutronOsmoIbcDenom,
		// 					Type: SplitType{
		// 						Custom: SplitConfig{
		// 							Receivers: map[string]string{
		// 								hubReceiverAddr:  "0.5",
		// 								osmoReceiverAddr: "0.5",
		// 							},
		// 						},
		// 					},
		// 				},
		// 			},
		// 			FallbackSplit: nil,
		// 		}

		// 		covenantAddress = testCtx.manualInstantiate(covenantRqCodeId, covenantMsg, neutronUser, keyring.BackendTest)
		// 		println("covenant address: ", covenantAddress)
		// 	})

		// 	t.Run("query covenant contracts", func(t *testing.T) {
		// 		clockAddress = testCtx.queryClockAddress(covenantAddress)
		// 		holderAddress = testCtx.queryHolderAddress(covenantAddress)
		// 		liquidPoolerAddress = testCtx.queryLiquidPoolerAddress(covenantAddress)
		// 		partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
		// 		partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
		// 		partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
		// 		partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
		// 	})

		// 	t.Run("fund contracts with neutron", func(t *testing.T) {
		// 		addrs := []string{
		// 			partyAIbcForwarderAddress,
		// 			partyBIbcForwarderAddress,
		// 			clockAddress,
		// 			partyARouterAddress,
		// 			partyBRouterAddress,
		// 			holderAddress,
		// 			liquidPoolerAddress,
		// 		}
		// 		println("funding addresses with 5000000000untrn")
		// 		testCtx.fundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)
		// 	})

		// 	t.Run("tick until forwarders create ICA", func(t *testing.T) {
		// 		testCtx.skipBlocks(5)
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
		// 			forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

		// 			if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
		// 				testCtx.skipBlocks(3)
		// 				partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
		// 				partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
		// 		testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, rqCaseOsmoAccount, int64(osmoContributionAmount))
		// 		testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, rqCaseHubAccount, int64(atomContributionAmount))

		// 		testCtx.skipBlocks(3)
		// 	})

		// 	t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
		// 		for {
		// 			holderOsmoBal := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
		// 			holderAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
		// 			holderState := testCtx.queryContractState(holderAddress)

		// 			println("holder atom bal: ", holderAtomBal)
		// 			println("holder osmo bal: ", holderOsmoBal)
		// 			println("holder state: ", holderState)

		// 			if holderAtomBal == atomContributionAmount && holderOsmoBal == osmoContributionAmount {
		// 				println("holder received atom & osmo")
		// 				break
		// 			} else if holderState == "active" {
		// 				println("holder is active")
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick until holder sends funds to LPer and receives LP tokens in return", func(t *testing.T) {
		// 		for {
		// 			holderLpTokenBal := testCtx.queryLpTokenBalance(liquidityTokenAddress, holderAddress)

		// 			if holderLpTokenBal == 0 {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			} else {
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("party A ragequits", func(t *testing.T) {
		// 		testCtx.skipBlocks(10)
		// 		testCtx.holderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
		// 		testCtx.skipBlocks(5)
		// 		for {
		// 			routerAtomBalA := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
		// 			routerOsmoBalB := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyBRouterAddress)

		// 			println("routerAtomBalA: ", routerAtomBalA)
		// 			println("routerOsmoBalB: ", routerOsmoBalB)

		// 			if routerAtomBalA != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick until party A ragequit is distributed", func(t *testing.T) {
		// 		for {
		// 			osmoBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom)
		// 			osmoBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom)
		// 			atomBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom)
		// 			atomBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom)

		// 			println("party A osmo bal: ", osmoBalPartyA)
		// 			println("party A atom bal: ", atomBalPartyA)
		// 			println("party B osmo bal: ", osmoBalPartyB)
		// 			println("party B atom bal: ", atomBalPartyB)

		// 			if atomBalPartyA != 0 && osmoBalPartyA != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("party B claims and router receives the funds", func(t *testing.T) {
		// 		testCtx.holderClaim(holderAddress, osmoNeutronAccount, keyring.BackendTest)
		// 		for {
		// 			routerAtomBalB := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
		// 			routerOsmoBalB := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyBRouterAddress)

		// 			println("routerAtomBalB: ", routerAtomBalB)
		// 			println("routerOsmoBalB: ", routerOsmoBalB)

		// 			if routerOsmoBalB != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
		// 		for {
		// 			osmoBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom)
		// 			atomBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom)
		// 			atomBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom)

		// 			println("party A atom bal: ", atomBalPartyA)
		// 			println("party B osmo bal: ", osmoBalPartyB)
		// 			println("party B atom bal: ", atomBalPartyB)

		// 			if atomBalPartyA != 0 && osmoBalPartyB != 0 && atomBalPartyB != 0 {
		// 				println("nice")
		// 				break
		// 			}
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 		}
		// 	})
		// })

		// t.Run("two party POL side-based ragequit path", func(t *testing.T) {

		// 	t.Run("instantiate covenant", func(t *testing.T) {
		// 		timeouts := Timeouts{
		// 			IcaTimeout:         "100", // sec
		// 			IbcTransferTimeout: "100", // sec
		// 		}

		// 		currentHeight := testCtx.getNeutronHeight()
		// 		depositBlock := Block(currentHeight + 200)
		// 		lockupBlock := Block(currentHeight + 300)

		// 		lockupConfig := Expiration{
		// 			AtHeight: &lockupBlock,
		// 		}
		// 		depositDeadline := Expiration{
		// 			AtHeight: &depositBlock,
		// 		}
		// 		presetIbcFee := PresetIbcFee{
		// 			AckFee:     "10000",
		// 			TimeoutFee: "10000",
		// 		}

		// 		atomCoin := Coin{
		// 			Denom:  cosmosAtom.Config().Denom,
		// 			Amount: strconv.FormatUint(atomContributionAmount, 10),
		// 		}

		// 		osmoCoin := Coin{
		// 			Denom:  cosmosOsmosis.Config().Denom,
		// 			Amount: strconv.FormatUint(osmoContributionAmount, 10),
		// 		}
		// 		hubReceiverAddr := sideBasedRqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
		// 		osmoReceiverAddr := sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)
		// 		partyAConfig := InterchainCovenantParty{
		// 			RemoteChainDenom:          "uatom",
		// 			PartyReceiverAddr:         hubReceiverAddr,
		// 			Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			Contribution:              atomCoin,
		// 			NativeDenom:               neutronAtomIbcDenom,
		// 			PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
		// 			PartyChainConnectionId:    neutronAtomIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 		}
		// 		partyBConfig := InterchainCovenantParty{
		// 			RemoteChainDenom:          "uosmo",
		// 			PartyReceiverAddr:         osmoReceiverAddr,
		// 			Addr:                      osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			Contribution:              osmoCoin,
		// 			NativeDenom:               neutronOsmoIbcDenom,
		// 			PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
		// 			PartyChainConnectionId:    neutronOsmosisIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 		}
		// 		codeIds := ContractCodeIds{
		// 			IbcForwarderCode:     ibcForwarderCodeId,
		// 			InterchainRouterCode: interchainRouterCodeId,
		// 			NativeRouterCode:     nativeRouterCodeId,
		// 			ClockCode:            clockCodeId,
		// 			HolderCode:           holderCodeId,
		// 			LiquidPoolerCode:     lperCodeId,
		// 		}

		// 		ragequitTerms := RagequitTerms{
		// 			Penalty: "0.1",
		// 		}

		// 		ragequitConfig := RagequitConfig{
		// 			Enabled: &ragequitTerms,
		// 		}

		// 		poolAddress := stableswapAddress
		// 		pairType := PairType{
		// 			Stable: struct{}{},
		// 		}

		// 		covenantMsg := CovenantInstantiateMsg{
		// 			Label:                    "two-party-pol-covenant-side-ragequit",
		// 			Timeouts:                 timeouts,
		// 			PresetIbcFee:             presetIbcFee,
		// 			ContractCodeIds:          codeIds,
		// 			LockupConfig:             lockupConfig,
		// 			PartyAConfig:             CovenantPartyConfig{Interchain: &partyAConfig},
		// 			PartyBConfig:             CovenantPartyConfig{Interchain: &partyBConfig},
		// 			PoolAddress:              poolAddress,
		// 			RagequitConfig:           &ragequitConfig,
		// 			DepositDeadline:          depositDeadline,
		// 			PartyAShare:              "50",
		// 			PartyBShare:              "50",
		// 			ExpectedPoolRatio:        "0.1",
		// 			AcceptablePoolRatioDelta: "0.09",
		// 			PairType:                 pairType,
		// 			CovenantType:             "side",
		// 			Splits: []DenomSplit{
		// 				{
		// 					Denom: neutronAtomIbcDenom,
		// 					Type: SplitType{
		// 						Custom: SplitConfig{
		// 							Receivers: map[string]string{
		// 								hubReceiverAddr:  "1.0",
		// 								osmoReceiverAddr: "0.0",
		// 							},
		// 						},
		// 					},
		// 				},
		// 				{
		// 					Denom: neutronOsmoIbcDenom,
		// 					Type: SplitType{
		// 						Custom: SplitConfig{
		// 							Receivers: map[string]string{
		// 								hubReceiverAddr:  "0.0",
		// 								osmoReceiverAddr: "1.0",
		// 							},
		// 						},
		// 					},
		// 				},
		// 			},
		// 			FallbackSplit: nil,
		// 		}

		// 		covenantAddress = testCtx.manualInstantiate(covenantSideBasedRqCodeId, covenantMsg, neutronUser, keyring.BackendTest)
		// 		println("covenant address: ", covenantAddress)
		// 	})

		// 	t.Run("query covenant contracts", func(t *testing.T) {
		// 		clockAddress = testCtx.queryClockAddress(covenantAddress)
		// 		holderAddress = testCtx.queryHolderAddress(covenantAddress)
		// 		liquidPoolerAddress = testCtx.queryLiquidPoolerAddress(covenantAddress)
		// 		partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
		// 		partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
		// 		partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
		// 		partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
		// 	})

		// 	t.Run("fund contracts with neutron", func(t *testing.T) {
		// 		addrs := []string{
		// 			partyAIbcForwarderAddress,
		// 			partyBIbcForwarderAddress,
		// 			clockAddress,
		// 			partyARouterAddress,
		// 			partyBRouterAddress,
		// 			holderAddress,
		// 			liquidPoolerAddress,
		// 		}
		// 		testCtx.fundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)

		// 		testCtx.skipBlocks(2)
		// 	})

		// 	t.Run("tick until forwarders create ICA", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
		// 			forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

		// 			if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
		// 				testCtx.skipBlocks(5)
		// 				partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
		// 				partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
		// 		testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, sideBasedRqCaseOsmoAccount, int64(osmoContributionAmount))
		// 		testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, sideBasedRqCaseHubAccount, int64(atomContributionAmount))

		// 		testCtx.skipBlocks(3)

		// 		atomBal, _ := cosmosAtom.GetBalance(ctx, partyADepositAddress, nativeAtomDenom)
		// 		require.Equal(t, int64(atomContributionAmount), atomBal)
		// 		osmoBal, _ := cosmosOsmosis.GetBalance(ctx, partyBDepositAddress, nativeOsmoDenom)
		// 		require.Equal(t, int64(osmoContributionAmount), osmoBal)
		// 	})

		// 	t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
		// 		for {
		// 			holderOsmoBal := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
		// 			holderAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
		// 			holderState := testCtx.queryContractState(holderAddress)

		// 			println("holder atom bal: ", holderAtomBal)
		// 			println("holder osmo bal: ", holderOsmoBal)
		// 			println("holder state: ", holderState)

		// 			if holderAtomBal == atomContributionAmount && holderOsmoBal == osmoContributionAmount {
		// 				println("holder received atom & osmo")
		// 				break
		// 			} else if holderState == "active" {
		// 				println("holderState: ", holderState)
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick until holder sends the funds to LPer and receives LP tokens in return", func(t *testing.T) {
		// 		for {
		// 			holderLpTokenBal := testCtx.queryLpTokenBalance(liquidityTokenAddress, holderAddress)
		// 			println("holder lp token balance: ", holderLpTokenBal)

		// 			if holderLpTokenBal == 0 {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			} else {
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("party A ragequits", func(t *testing.T) {
		// 		testCtx.skipBlocks(10)
		// 		testCtx.holderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
		// 		testCtx.skipBlocks(5)
		// 		for {
		// 			routerAtomBalA := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
		// 			routerOsmoBalB := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyBRouterAddress)

		// 			println("routerAtomBalA: ", routerAtomBalA)
		// 			println("routerOsmoBalB: ", routerOsmoBalB)

		// 			if routerAtomBalA != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
		// 		for {
		// 			osmoBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
		// 			)
		// 			atomBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, sideBasedRqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
		// 			)
		// 			atomBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom,
		// 			)

		// 			println("party A atom bal: ", atomBalPartyA)
		// 			println("party B osmo bal: ", osmoBalPartyB)
		// 			println("party B atom bal: ", atomBalPartyB)

		// 			if atomBalPartyA != 0 && osmoBalPartyB != 0 && atomBalPartyB != 0 {
		// 				println("nice")
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})
		// })

		// t.Run("two party POL side-based happy path", func(t *testing.T) {
		// 	var expirationHeight Block
		// 	t.Run("instantiate covenant", func(t *testing.T) {
		// 		timeouts := Timeouts{
		// 			IcaTimeout:         "100", // sec
		// 			IbcTransferTimeout: "100", // sec
		// 		}

		// 		currentHeight := testCtx.getNeutronHeight()
		// 		depositBlock := Block(currentHeight + 180)
		// 		lockupBlock := Block(currentHeight + 180)
		// 		expirationHeight = lockupBlock
		// 		lockupConfig := Expiration{
		// 			AtHeight: &lockupBlock,
		// 		}
		// 		depositDeadline := Expiration{
		// 			AtHeight: &depositBlock,
		// 		}
		// 		presetIbcFee := PresetIbcFee{
		// 			AckFee:     "10000",
		// 			TimeoutFee: "10000",
		// 		}

		// 		atomCoin := Coin{
		// 			Denom:  cosmosAtom.Config().Denom,
		// 			Amount: strconv.FormatUint(atomContributionAmount, 10),
		// 		}

		// 		osmoCoin := Coin{
		// 			Denom:  cosmosOsmosis.Config().Denom,
		// 			Amount: strconv.FormatUint(osmoContributionAmount, 10),
		// 		}
		// 		hubReceiverAddr := sideBasedHappyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
		// 		osmoReceiverAddr := sideBasedHappyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)
		// 		partyAConfig := InterchainCovenantParty{
		// 			RemoteChainDenom:          "uatom",
		// 			PartyReceiverAddr:         hubReceiverAddr,
		// 			Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			Contribution:              atomCoin,
		// 			NativeDenom:               neutronAtomIbcDenom,
		// 			PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
		// 			PartyChainConnectionId:    neutronAtomIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 		}
		// 		partyBConfig := InterchainCovenantParty{
		// 			RemoteChainDenom:          "uosmo",
		// 			PartyReceiverAddr:         osmoReceiverAddr,
		// 			Addr:                      osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			Contribution:              osmoCoin,
		// 			NativeDenom:               neutronOsmoIbcDenom,
		// 			PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
		// 			PartyChainConnectionId:    neutronOsmosisIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 		}
		// 		codeIds := ContractCodeIds{
		// 			IbcForwarderCode:     ibcForwarderCodeId,
		// 			InterchainRouterCode: interchainRouterCodeId,
		// 			NativeRouterCode:     nativeRouterCodeId,
		// 			ClockCode:            clockCodeId,
		// 			HolderCode:           holderCodeId,
		// 			LiquidPoolerCode:     lperCodeId,
		// 		}

		// 		ragequitTerms := RagequitTerms{
		// 			Penalty: "0.1",
		// 		}

		// 		ragequitConfig := RagequitConfig{
		// 			Enabled: &ragequitTerms,
		// 		}

		// 		poolAddress := stableswapAddress
		// 		pairType := PairType{
		// 			Stable: struct{}{},
		// 		}

		// 		covenantMsg := CovenantInstantiateMsg{
		// 			Label:                    "two-party-pol-covenant-side-happy",
		// 			Timeouts:                 timeouts,
		// 			PresetIbcFee:             presetIbcFee,
		// 			ContractCodeIds:          codeIds,
		// 			LockupConfig:             lockupConfig,
		// 			PartyAConfig:             CovenantPartyConfig{Interchain: &partyAConfig},
		// 			PartyBConfig:             CovenantPartyConfig{Interchain: &partyBConfig},
		// 			PoolAddress:              poolAddress,
		// 			RagequitConfig:           &ragequitConfig,
		// 			DepositDeadline:          depositDeadline,
		// 			PartyAShare:              "50",
		// 			PartyBShare:              "50",
		// 			ExpectedPoolRatio:        "0.1",
		// 			AcceptablePoolRatioDelta: "0.09",
		// 			PairType:                 pairType,
		// 			CovenantType:             "side",
		// 			Splits: []DenomSplit{
		// 				{
		// 					Denom: neutronAtomIbcDenom,
		// 					Type: SplitType{
		// 						Custom: SplitConfig{
		// 							Receivers: map[string]string{
		// 								hubReceiverAddr:  "1.0",
		// 								osmoReceiverAddr: "0.0",
		// 							},
		// 						},
		// 					},
		// 				},
		// 				{
		// 					Denom: neutronOsmoIbcDenom,
		// 					Type: SplitType{
		// 						Custom: SplitConfig{
		// 							Receivers: map[string]string{
		// 								hubReceiverAddr:  "0.0",
		// 								osmoReceiverAddr: "1.0",
		// 							},
		// 						},
		// 					},
		// 				},
		// 			},
		// 			FallbackSplit: nil,
		// 		}

		// 		covenantAddress = testCtx.manualInstantiate(covenantSideBasedRqCodeId, covenantMsg, neutronUser, keyring.BackendTest)
		// 		println("covenant address: ", covenantAddress)
		// 	})

		// 	t.Run("query covenant contracts", func(t *testing.T) {
		// 		clockAddress = testCtx.queryClockAddress(covenantAddress)
		// 		holderAddress = testCtx.queryHolderAddress(covenantAddress)
		// 		liquidPoolerAddress = testCtx.queryLiquidPoolerAddress(covenantAddress)
		// 		partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
		// 		partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
		// 		partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
		// 		partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
		// 	})

		// 	t.Run("fund contracts with neutron", func(t *testing.T) {
		// 		addrs := []string{
		// 			partyAIbcForwarderAddress,
		// 			partyBIbcForwarderAddress,
		// 			clockAddress,
		// 			partyARouterAddress,
		// 			partyBRouterAddress,
		// 			holderAddress,
		// 			liquidPoolerAddress,
		// 		}
		// 		testCtx.fundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)

		// 		testCtx.skipBlocks(2)
		// 	})

		// 	t.Run("tick until forwarders create ICA", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
		// 			forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

		// 			if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
		// 				testCtx.skipBlocks(5)
		// 				partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
		// 				partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
		// 		testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, sideBasedHappyCaseOsmoAccount, int64(osmoContributionAmount))
		// 		testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, sideBasedHappyCaseHubAccount, int64(atomContributionAmount))

		// 		testCtx.skipBlocks(3)

		// 		atomBal, _ := cosmosAtom.GetBalance(ctx, partyADepositAddress, nativeAtomDenom)
		// 		require.Equal(t, int64(atomContributionAmount), atomBal)
		// 		osmoBal, _ := cosmosOsmosis.GetBalance(ctx, partyBDepositAddress, nativeOsmoDenom)
		// 		require.Equal(t, int64(osmoContributionAmount), osmoBal)
		// 	})

		// 	t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
		// 		for {
		// 			holderOsmoBal := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
		// 			holderAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
		// 			holderState := testCtx.queryContractState(holderAddress)

		// 			println("holder atom bal: ", holderAtomBal)
		// 			println("holder osmo bal: ", holderOsmoBal)
		// 			println("holder state: ", holderState)

		// 			if holderAtomBal == atomContributionAmount && holderOsmoBal == osmoContributionAmount {
		// 				println("holder/liquidpooler received atom & osmo")
		// 				break
		// 			} else if holderState == "active" {
		// 				println("holderState: ", holderState)
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick until holder sends the funds to LPer and receives LP tokens in return", func(t *testing.T) {
		// 		for {
		// 			holderLpTokenBal := testCtx.queryLpTokenBalance(liquidityTokenAddress, holderAddress)
		// 			println("holder lp token balance: ", holderLpTokenBal)

		// 			if holderLpTokenBal == 0 {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			} else {
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("lockup expires", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			if testCtx.getNeutronHeight() >= uint64(expirationHeight) {
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("party A claims", func(t *testing.T) {
		// 		for {
		// 			routerAtomBalB := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
		// 			routerOsmoBalB := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyBRouterAddress)
		// 			routerAtomBalA := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
		// 			routerOsmoBalA := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyARouterAddress)

		// 			println("routerAtomBalB: ", routerAtomBalB)
		// 			println("routerOsmoBalB: ", routerOsmoBalB)
		// 			println("routerAtomBalA: ", routerAtomBalA)
		// 			println("routerOsmoBalA: ", routerOsmoBalA)

		// 			if routerOsmoBalB != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 				testCtx.holderClaim(holderAddress, osmoNeutronAccount, keyring.BackendTest)
		// 			}
		// 		}

		// 	})

		// 	t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
		// 		for {
		// 			osmoBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, sideBasedHappyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
		// 			)
		// 			atomBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, sideBasedHappyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
		// 			)

		// 			println("party A atom bal: ", atomBalPartyA)
		// 			println("party B osmo bal: ", osmoBalPartyB)

		// 			if atomBalPartyA != 0 && osmoBalPartyB != 0 {
		// 				println("nice")
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})
		// })
	})
}