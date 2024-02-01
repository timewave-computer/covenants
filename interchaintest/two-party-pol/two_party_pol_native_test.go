package covenant_two_party_pol

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
	utils "github.com/timewave-computer/covenants/interchaintest/utils"
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
			ModifyGenesis:       utils.SetupGaiaGenesis(utils.GetDefaultInterchainGenesisMessages()),
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
				ModifyGenesis: utils.SetupNeutronGenesis(
					"0.05",
					[]string{nativeNtrnDenom},
					[]string{nativeAtomDenom},
					utils.GetDefaultNeutronInterchainGenesisMessages(),
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
				ModifyGenesis: utils.SetupOsmoGenesis(
					append(utils.GetDefaultInterchainGenesisMessages(), "/ibc.applications.interchain_accounts.v1.InterchainAccount"),
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

	testCtx := &utils.TestContext{
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
		T:                         t,
		Ctx:                       ctx,
	}

	testCtx.SkipBlocks(5)

	t.Run("generate IBC paths", func(t *testing.T) {
		utils.GeneratePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosNeutron.Config().ChainID, gaiaNeutronIBCPath)
		utils.GeneratePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosAtom.Config().ChainID, gaiaNeutronICSPath)
	})

	t.Run("setup neutron-gaia ICS", func(t *testing.T) {
		utils.GenerateClient(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)
		neutronClients := testCtx.GetChainClients(cosmosNeutron.Config().Name)
		atomClients := testCtx.GetChainClients(cosmosAtom.Config().Name)

		err = r.UpdatePath(ctx, eRep, gaiaNeutronICSPath, ibc.PathUpdateOptions{
			SrcClientID: &neutronClients[0].ClientID,
			DstClientID: &atomClients[0].ClientID,
		})
		require.NoError(t, err)

		atomNeutronICSConnectionId, neutronAtomICSConnectionId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)

		utils.GenerateICSChannel(t, ctx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)

		utils.CreateValidator(t, ctx, r, eRep, atom, neutron)
		testCtx.SkipBlocks(2)
	})

	t.Run("setup IBC interchain clients, connections, and links", func(t *testing.T) {
		utils.GenerateClient(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		atomNeutronIBCConnId, neutronAtomIBCConnId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		utils.LinkPath(t, ctx, r, eRep, cosmosAtom, cosmosNeutron, gaiaNeutronIBCPath)
		testCtx.SkipBlocks(10)
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
	testCtx.SkipBlocks(2)

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron)
	gaiaUser, neutronUser := users[0], users[1]
	_, _ = gaiaUser, neutronUser
	hubNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]

	rqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	sideBasedRqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	happyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	sideBasedHappyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]

	rqCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	sideBasedRqCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	happyCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	sideBasedHappyCaseNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(neutronContributionAmount), neutron)[0]

	testCtx.SkipBlocks(5)

	t.Run("determine ibc channels", func(t *testing.T) {
		neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)

		// Find all pairwise channels
		utils.GetPairwiseTransferChannelIds(testCtx, gaiaChannelInfo, neutronChannelInfo, atomNeutronIBCConnId, neutronAtomIBCConnId, cosmosAtom.Config().Name, neutron.Config().Name)
		utils.GetPairwiseCCVChannelIds(testCtx, gaiaChannelInfo, neutronChannelInfo, atomNeutronICSConnectionId, neutronAtomICSConnectionId, cosmosAtom.Config().Name, cosmosNeutron.Config().Name)
	})

	t.Run("determine ibc denoms", func(t *testing.T) {
		// We can determine the ibc denoms of:
		// 1. ATOM on Neutron
		neutronAtomIbcDenom = testCtx.GetIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
			nativeAtomDenom,
		)
		// 2. neutron => hub
		hubNeutronIbcDenom = testCtx.GetIbcDenom(
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
		var covenantRqCodeId uint64
		var covenantSideBasedRqCodeId uint64

		t.Run("deploy covenant contracts", func(t *testing.T) {
			// something was going wrong with instantiating the same code twice,
			// hence this weird workaround
			covenantCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)
			covenantRqCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)
			covenantSideBasedRqCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)

			// store clock and get code id
			clockCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, clockContractPath)

			// store routers and get code id
			interchainRouterCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, interchainRouterContractPath)
			nativeRouterCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, nativeRouterContractPath)

			// store forwarder and get code id
			ibcForwarderCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, ibcForwarderContractPath)

			// store lper, get code
			lperCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, liquidPoolerPath)

			// store holder and get code id
			holderCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, holderContractPath)

			testCtx.SkipBlocks(5)
		})

		t.Run("deploy astroport contracts", func(t *testing.T) {
			stablePairCodeId := testCtx.StoreContract(cosmosNeutron, neutronUser, "wasms/astroport_pair_stable.wasm")
			factoryCodeId := testCtx.StoreContract(cosmosNeutron, neutronUser, "wasms/astroport_factory.wasm")
			whitelistCodeId := testCtx.StoreContract(cosmosNeutron, neutronUser, "wasms/astroport_whitelist.wasm")
			tokenCodeId := testCtx.StoreContract(cosmosNeutron, neutronUser, "wasms/astroport_token.wasm")
			coinRegistryCodeId := testCtx.StoreContract(cosmosNeutron, neutronUser, "wasms/astroport_native_coin_registry.wasm")

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
				testCtx.SkipBlocks(2)
			})

			t.Run("factory", func(t *testing.T) {
				factoryAddress = testCtx.InstantiateAstroportFactory(
					stablePairCodeId, tokenCodeId, whitelistCodeId, factoryCodeId, coinRegistryAddress, neutronUser)
				println("astroport factory: ", factoryAddress)
				testCtx.SkipBlocks(2)
			})
			t.Run("create pair on factory", func(t *testing.T) {
				testCtx.CreateAstroportFactoryPair(3, cosmosNeutron.Config().Denom, neutronAtomIbcDenom, factoryAddress, neutronUser, keyring.BackendTest)
			})
		})

		t.Run("add liquidity to the atom-neutron stableswap pool", func(t *testing.T) {
			liquidityTokenAddress, stableswapAddress = testCtx.QueryAstroLpTokenAndStableswapAddress(
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

			testCtx.SkipBlocks(2)

			testCtx.ProvideAstroportLiquidity(
				neutronAtomIbcDenom, cosmosNeutron.Config().Denom, atomContributionAmount, neutronContributionAmount, neutronUser, stableswapAddress)

			testCtx.SkipBlocks(2)
			testCtx.QueryLpTokenBalance(liquidityTokenAddress, neutronUser.Bech32Address(neutron.Config().Bech32Prefix))
		})

		t.Run("two party POL happy path", func(t *testing.T) {
			var depositBlock Block
			var lockupBlock Block
			var hubReceiverAddr string
			var neutronReceiverAddr string
			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "10000", // sec
					IbcTransferTimeout: "10000", // sec
				}

				currentHeight := testCtx.GetNeutronHeight()
				depositBlock = Block(currentHeight + 200)
				lockupBlock = Block(currentHeight + 200)

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

				hubReceiverAddr = happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				neutronReceiverAddr = happyCaseNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix)

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
					DenomToPfmMap:             map[string]PacketForwardMiddlewareConfig{},
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

				denomSplits := map[string]SplitConfig{
					neutronAtomIbcDenom: SplitConfig{
						Receivers: map[string]string{
							hubReceiverAddr:     "0.5",
							neutronReceiverAddr: "0.5",
						},
					},
					cosmosNeutron.Config().Denom: SplitConfig{
						Receivers: map[string]string{
							hubReceiverAddr:     "0.5",
							neutronReceiverAddr: "0.5",
						},
					},
				}

				liquidPoolerConfig := LiquidPoolerConfig{
					Astroport: &AstroportLiquidPoolerConfig{
						PairType:    pairType,
						PoolAddress: poolAddress,
						AssetADenom: neutronAtomIbcDenom,
						AssetBDenom: cosmosNeutron.Config().Denom,
						SingleSideLpLimits: SingleSideLpLimits{
							AssetALimit: "100000",
							AssetBLimit: "100000",
						},
					},
				}

				fundingDuration := Duration{
					Time: new(uint64),
				}
				*fundingDuration.Time = 300

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
					RagequitConfig:     &ragequitConfig,
					DepositDeadline:    depositDeadline,
					PartyAShare:        "50",
					PartyBShare:        "50",
					CovenantType:       "share",
					Splits:             denomSplits,
					FallbackSplit:      nil,
					LiquidPoolerConfig: liquidPoolerConfig,
					PoolPriceConfig: PoolPriceConfig{
						ExpectedSpotPrice:     "0.1",
						AcceptablePriceSpread: "0.09",
					},
				}

				covenantAddress = testCtx.ManualInstantiate(covenantCodeId, covenantMsg, neutronUser, keyring.BackendTest)

				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.QueryClockAddress(covenantAddress)
				holderAddress = testCtx.QueryHolderAddress(covenantAddress)
				liquidPoolerAddress = testCtx.QueryLiquidPoolerAddress(covenantAddress)
				partyARouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_a")
				partyBRouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_b")
				partyAIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_a")
				partyBIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_b")
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
				testCtx.FundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				for {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					forwarderAState := testCtx.QueryContractState(partyAIbcForwarderAddress)
					println("forwarderAState: ", forwarderAState)
					if forwarderAState == "ica_created" {
						partyADepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_a")
						partyBDepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_b")
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
				testCtx.FundChainAddrs([]string{partyBDepositAddress}, cosmosNeutron, happyCaseNeutronAccount, int64(neutronContributionAmount))
				testCtx.FundChainAddrs([]string{partyADepositAddress}, cosmosAtom, happyCaseHubAccount, int64(atomContributionAmount))

				testCtx.SkipBlocks(3)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					holderNeutronBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
					holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
					holderState := testCtx.QueryContractState(holderAddress)
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

			t.Run("tick until holder sends funds to LiquidPooler and LPer receives LP tokens", func(t *testing.T) {
				for {
					if testCtx.QueryLpTokenBalance(liquidityTokenAddress, liquidPoolerAddress) == 0 {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("tick until holder expires", func(t *testing.T) {
				for {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					holderState := testCtx.QueryContractState(holderAddress)
					println("holder state: ", holderState)

					if holderState == "expired" {
						break
					}
				}
			})

			t.Run("party A claims and router receives the funds", func(t *testing.T) {
				routerNeutronBalA := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
				routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
				println("routerAtomBalA: ", routerAtomBalA)
				println("routerNeutronBalA: ", routerNeutronBalA)
				routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
				routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
				println("routerAtomBalB: ", routerAtomBalB)
				println("routerNeutronBalB: ", routerNeutronBalB)
				holderNtrnBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
				holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				println("holderNtrnBal: ", holderNtrnBal)
				println("holderAtomBal: ", holderAtomBal)

				testCtx.SkipBlocks(10)
				testCtx.HolderClaim(holderAddress, hubNeutronAccount, keyring.BackendTest)
				testCtx.SkipBlocks(5)

				routerNeutronBalA = testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
				routerAtomBalA = testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
				println("routerAtomBalA: ", routerAtomBalA)
				println("routerNeutronBalA: ", routerNeutronBalA)
				routerNeutronBalB = testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
				routerAtomBalB = testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
				println("routerAtomBalB: ", routerAtomBalB)
				println("routerNeutronBalB: ", routerNeutronBalB)
				holderNtrnBal = testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
				holderAtomBal = testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				println("holderNtrnBal: ", holderNtrnBal)
				println("holderAtomBal: ", holderAtomBal)

				// for {
				// 	routerNeutronBalA := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
				// 	routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
				// 	println("routerAtomBalA: ", routerAtomBalA)
				// 	println("routerNeutronBalA: ", routerNeutronBalA)
				// 	if routerAtomBalA != 0 && routerNeutronBalA != 0 {
				// 		break
				// 	} else {
				// 		testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				// 	}
				// }
			})

			t.Run("tick until party A claim is distributed", func(t *testing.T) {
				println("hub receiver addr: ", hubReceiverAddr)
				for {
					routerNeutronBalA := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
					routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
					println("routerAtomBalA: ", routerAtomBalA)
					println("routerNeutronBalA: ", routerNeutronBalA)
					routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
					routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
					println("routerAtomBalB: ", routerAtomBalB)
					println("routerNeutronBalB: ", routerNeutronBalB)
					holderNtrnBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
					holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
					println("holderNtrnBal: ", holderNtrnBal)
					println("holderAtomBal: ", holderAtomBal)

					// routerNeutronBalA := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
					// routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
					atomBalPartyA := testCtx.QueryHubDenomBalance(cosmosAtom.Config().Denom, hubReceiverAddr)
					neutronBalPartyA := testCtx.QueryHubDenomBalance(hubNeutronIbcDenom, hubReceiverAddr)

					println("party A router atom bal: ", routerAtomBalA)
					println("party A router neutron bal: ", routerNeutronBalA)

					atomBalPartyB := testCtx.QueryNeutronDenomBalance(cosmosAtom.Config().Denom, neutronReceiverAddr)
					neutronBalPartyB := testCtx.QueryNeutronDenomBalance(hubNeutronIbcDenom, neutronReceiverAddr)

					println("party B router atom bal: ", atomBalPartyB)
					println("party B router neutron bal: ", neutronBalPartyB)

					if atomBalPartyA != 0 && neutronBalPartyA != 0 {
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("party B claims and router receives the funds", func(t *testing.T) {
				routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
				routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
				println("routerAtomBalB: ", routerAtomBalB)
				println("routerNeutronBalB: ", routerNeutronBalB)
				testCtx.SkipBlocks(5)
				testCtx.HolderClaim(holderAddress, happyCaseNeutronAccount, keyring.BackendTest)
				testCtx.SkipBlocks(5)
				// for {
				// 	routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
				// 	routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
				// 	println("routerAtomBalB: ", routerAtomBalB)
				// 	println("routerNeutronBalB: ", routerNeutronBalB)
				// 	if routerAtomBalB != 0 && routerNeutronBalB != 0 {
				// 		break
				// 	} else {
				// 		testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				// 	}
				// }
			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					atomBalPartyA := testCtx.QueryHubDenomBalance(cosmosAtom.Config().Denom, hubReceiverAddr)
					neutronBalPartyA := testCtx.QueryHubDenomBalance(hubNeutronIbcDenom, hubReceiverAddr)
					neutronBalPartyB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, neutronReceiverAddr)
					atomBalPartyB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, neutronReceiverAddr)

					println("party A neutron bal: ", neutronBalPartyA)
					println("party A atom bal: ", atomBalPartyA)
					println("party B neutron bal: ", neutronBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					if atomBalPartyB != 0 && neutronBalPartyB != 0 {
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})
		})

		t.Run("two party share based POL ragequit path", func(t *testing.T) {
			var hubReceiverAddr string
			var neutronReceiverAddr string
			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "100", // sec
					IbcTransferTimeout: "100", // sec
				}

				currentHeight := testCtx.GetNeutronHeight()
				depositBlock := Block(currentHeight + 200)
				lockupBlock := Block(currentHeight + 300)

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
				hubReceiverAddr = rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				neutronReceiverAddr = rqCaseNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix)

				partyAConfig := InterchainCovenantParty{
					RemoteChainDenom:          "uatom",
					PartyReceiverAddr:         hubReceiverAddr,
					Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              atomCoin,
					NativeDenom:               neutronAtomIbcDenom,
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
					DenomToPfmMap:             map[string]PacketForwardMiddlewareConfig{},
				}
				partyBConfig := NativeCovenantParty{
					PartyReceiverAddr: neutronReceiverAddr,
					Addr:              neutronReceiverAddr,
					Contribution:      neutronCoin,
					NativeDenom:       cosmosNeutron.Config().Denom,
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

				liquidPoolerConfig := LiquidPoolerConfig{
					Astroport: &AstroportLiquidPoolerConfig{
						PairType:    pairType,
						PoolAddress: poolAddress,
						AssetADenom: neutronAtomIbcDenom,
						AssetBDenom: cosmosNeutron.Config().Denom,
						SingleSideLpLimits: SingleSideLpLimits{
							AssetALimit: "100000",
							AssetBLimit: "100000",
						},
					},
				}

				covenantMsg := CovenantInstantiateMsg{
					Label:           "two-party-pol-covenant-ragequit",
					Timeouts:        timeouts,
					PresetIbcFee:    presetIbcFee,
					ContractCodeIds: codeIds,
					LockupConfig:    lockupConfig,
					PartyAConfig:    CovenantPartyConfig{Interchain: &partyAConfig},
					PartyBConfig:    CovenantPartyConfig{Native: &partyBConfig},
					RagequitConfig:  &ragequitConfig,
					DepositDeadline: depositDeadline,
					CovenantType:    "share",
					PartyAShare:     "50",
					PartyBShare:     "50",
					PoolPriceConfig: PoolPriceConfig{
						ExpectedSpotPrice:     "0.1",
						AcceptablePriceSpread: "0.09",
					},
					Splits: map[string]SplitConfig{
						neutronAtomIbcDenom:          SplitConfig{Receivers: map[string]string{hubReceiverAddr: "0.5", neutronReceiverAddr: "0.5"}},
						cosmosNeutron.Config().Denom: SplitConfig{Receivers: map[string]string{hubReceiverAddr: "0.5", neutronReceiverAddr: "0.5"}},
					},
					FallbackSplit:      nil,
					EmergencyCommittee: neutronUser.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					LiquidPoolerConfig: liquidPoolerConfig,
				}

				covenantAddress = testCtx.ManualInstantiate(covenantRqCodeId, covenantMsg, neutronUser, keyring.BackendTest)
				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.QueryClockAddress(covenantAddress)
				holderAddress = testCtx.QueryHolderAddress(covenantAddress)
				liquidPoolerAddress = testCtx.QueryLiquidPoolerAddress(covenantAddress)
				partyARouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_a")
				partyBRouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_b")
				partyAIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_a")
				partyBIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_b")
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
				println("funding addresses with 5000000000untrn")
				testCtx.FundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				testCtx.SkipBlocks(5)
				for {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					forwarderAState := testCtx.QueryContractState(partyAIbcForwarderAddress)

					if forwarderAState == "ica_created" {
						testCtx.SkipBlocks(3)
						partyADepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_a")
						partyBDepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_b")
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
				testCtx.FundChainAddrs([]string{partyBDepositAddress}, cosmosNeutron, rqCaseNeutronAccount, int64(neutronContributionAmount))
				testCtx.FundChainAddrs([]string{partyADepositAddress}, cosmosAtom, rqCaseHubAccount, int64(atomContributionAmount))

				testCtx.SkipBlocks(3)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					holderNeutronBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
					holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
					holderState := testCtx.QueryContractState(holderAddress)

					println("holder atom bal: ", holderAtomBal)
					println("holder neutron bal: ", holderNeutronBal)
					println("holder state: ", holderState)

					if holderAtomBal == atomContributionAmount && holderNeutronBal == neutronContributionAmount {
						println("holder received atom & neutron")
						break
					} else if holderState == "active" {
						println("holder is active")
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder sends funds to LPer and receives LP tokens in return", func(t *testing.T) {
				for {
					holderLpTokenBal := testCtx.QueryLpTokenBalance(liquidityTokenAddress, liquidPoolerAddress)

					if holderLpTokenBal == 0 {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("party A ragequits", func(t *testing.T) {
				testCtx.SkipBlocks(10)
				testCtx.HolderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
				testCtx.SkipBlocks(5)
				for {
					routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
					routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)

					println("routerAtomBalA: ", routerAtomBalA)
					println("routerNeutronBalB: ", routerNeutronBalB)

					if routerAtomBalA != 0 {
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until party A ragequit is distributed", func(t *testing.T) {
				for {

					neutronBalPartyA := testCtx.QueryHubDenomBalance(hubNeutronIbcDenom, hubReceiverAddr)
					neutronBalPartyB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, neutronReceiverAddr)
					atomBalPartyA := testCtx.QueryHubDenomBalance(cosmosAtom.Config().Denom, hubReceiverAddr)
					atomBalPartyB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, neutronReceiverAddr)

					println("party A osmo bal: ", neutronBalPartyA)
					println("party A atom bal: ", atomBalPartyA)
					println("party B osmo bal: ", neutronBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					if atomBalPartyA != 0 && neutronBalPartyA != 0 {
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("party B claims and router receives the funds", func(t *testing.T) {
				testCtx.SkipBlocks(5)
				testCtx.HolderClaim(holderAddress, rqCaseNeutronAccount, keyring.BackendTest)
				testCtx.SkipBlocks(5)

				for {
					routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
					routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)

					println("routerAtomBalB: ", routerAtomBalB)
					println("routerNeutronBalB: ", routerNeutronBalB)

					if routerNeutronBalB != 0 {
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					neutronBalPartyB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, neutronReceiverAddr)
					atomBalPartyA := testCtx.QueryHubDenomBalance(cosmosAtom.Config().Denom, hubReceiverAddr)
					atomBalPartyB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, neutronReceiverAddr)

					println("party A atom bal: ", atomBalPartyA)
					println("party B neutron bal: ", neutronBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					if atomBalPartyA != 0 && neutronBalPartyB != 0 && atomBalPartyB != 0 {
						println("nice")
						break
					}
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			})
		})

		t.Run("two party POL side-based ragequit path", func(t *testing.T) {
			var hubReceiverAddr string
			var neutronReceiverAddr string

			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "100", // sec
					IbcTransferTimeout: "100", // sec
				}

				currentHeight := testCtx.GetNeutronHeight()
				depositBlock := Block(currentHeight + 200)
				lockupBlock := Block(currentHeight + 300)

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
				hubReceiverAddr = sideBasedRqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				neutronReceiverAddr = sideBasedRqCaseNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix)
				partyAConfig := InterchainCovenantParty{
					RemoteChainDenom:          "uatom",
					PartyReceiverAddr:         hubReceiverAddr,
					Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              atomCoin,
					NativeDenom:               neutronAtomIbcDenom,
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
					DenomToPfmMap:             map[string]PacketForwardMiddlewareConfig{},
				}
				partyBConfig := NativeCovenantParty{
					PartyReceiverAddr: neutronReceiverAddr,
					Addr:              neutronReceiverAddr,
					Contribution:      neutronCoin,
					NativeDenom:       cosmosNeutron.Config().Denom,
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

				liquidPoolerConfig := LiquidPoolerConfig{
					Astroport: &AstroportLiquidPoolerConfig{
						PairType:    pairType,
						PoolAddress: poolAddress,
						AssetADenom: neutronAtomIbcDenom,
						AssetBDenom: cosmosNeutron.Config().Denom,
						SingleSideLpLimits: SingleSideLpLimits{
							AssetALimit: "100000",
							AssetBLimit: "100000",
						},
					},
				}

				covenantMsg := CovenantInstantiateMsg{
					Label:           "two-party-pol-covenant-side-ragequit",
					Timeouts:        timeouts,
					PresetIbcFee:    presetIbcFee,
					ContractCodeIds: codeIds,
					LockupConfig:    lockupConfig,
					PartyAConfig:    CovenantPartyConfig{Interchain: &partyAConfig},
					PartyBConfig:    CovenantPartyConfig{Native: &partyBConfig},
					RagequitConfig:  &ragequitConfig,
					DepositDeadline: depositDeadline,
					PartyAShare:     "50",
					PartyBShare:     "50",
					PoolPriceConfig: PoolPriceConfig{
						ExpectedSpotPrice:     "0.1",
						AcceptablePriceSpread: "0.09",
					},
					CovenantType: "side",
					Splits: map[string]SplitConfig{
						neutronAtomIbcDenom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:     "1.0",
								neutronReceiverAddr: "0.0",
							},
						},
						cosmosNeutron.Config().Denom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:     "0.0",
								neutronReceiverAddr: "1.0",
							},
						},
					},
					FallbackSplit:      nil,
					LiquidPoolerConfig: liquidPoolerConfig,
				}

				covenantAddress = testCtx.ManualInstantiate(covenantSideBasedRqCodeId, covenantMsg, neutronUser, keyring.BackendTest)
				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.QueryClockAddress(covenantAddress)
				holderAddress = testCtx.QueryHolderAddress(covenantAddress)
				liquidPoolerAddress = testCtx.QueryLiquidPoolerAddress(covenantAddress)
				partyARouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_a")
				partyBRouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_b")
				partyAIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_a")
				partyBIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_b")
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
				testCtx.FundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)

				testCtx.SkipBlocks(2)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				for {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					forwarderAState := testCtx.QueryContractState(partyAIbcForwarderAddress)

					if forwarderAState == "ica_created" {
						testCtx.SkipBlocks(5)
						partyADepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_a")
						partyBDepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_b")
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
				testCtx.FundChainAddrs([]string{partyBDepositAddress}, cosmosNeutron, sideBasedRqCaseNeutronAccount, int64(neutronContributionAmount))
				testCtx.FundChainAddrs([]string{partyADepositAddress}, cosmosAtom, sideBasedRqCaseHubAccount, int64(atomContributionAmount))

				testCtx.SkipBlocks(3)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					holderNeutronBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
					holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
					holderState := testCtx.QueryContractState(holderAddress)

					println("holder atom bal: ", holderAtomBal)
					println("holder neutron bal: ", holderNeutronBal)

					if holderAtomBal == atomContributionAmount && holderNeutronBal == neutronContributionAmount {
						println("holder received atom & neutron")
						break
					} else if holderState == "active" {
						println("holder state: ", holderState)
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder sends the funds to LPer and receives LP tokens in return", func(t *testing.T) {
				for {
					holderLpTokenBal := testCtx.QueryLpTokenBalance(liquidityTokenAddress, liquidPoolerAddress)
					println("holder lp token balance: ", holderLpTokenBal)

					if holderLpTokenBal == 0 {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("party A ragequits", func(t *testing.T) {
				testCtx.SkipBlocks(10)
				testCtx.HolderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
				testCtx.SkipBlocks(5)
				for {
					routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
					routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
					routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
					routerNeutronBalA := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)

					println("routerAtomBalA: ", routerAtomBalA)
					println("routerAtomBalB: ", routerAtomBalB)
					println("routerNeutronBalB: ", routerNeutronBalB)
					println("routerNeutronBalA: ", routerNeutronBalA)

					if routerAtomBalA != 0 {
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
					routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
					routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
					routerNeutronBalA := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
					neutronBalPartyB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, neutronReceiverAddr)
					neutronBalPartyA := testCtx.QueryHubDenomBalance(hubNeutronIbcDenom, hubReceiverAddr)
					atomBalPartyA := testCtx.QueryHubDenomBalance(cosmosAtom.Config().Denom, hubReceiverAddr)
					atomBalPartyB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, neutronReceiverAddr)

					println("routerAtomBalA: ", routerAtomBalA)
					println("routerAtomBalB: ", routerAtomBalB)
					println("routerNeutronBalB: ", routerNeutronBalB)
					println("routerNeutronBalA: ", routerNeutronBalA)
					println("party A atom bal: ", atomBalPartyA)
					println("party A neutron bal: ", neutronBalPartyA)
					println("party B neutron bal: ", neutronBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					println("\n")

					if atomBalPartyA != 0 && neutronBalPartyB != 0 && atomBalPartyB != 0 {
						println("nice")
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})
		})

		t.Run("two party POL side-based happy path", func(t *testing.T) {

			var hubReceiverAddr string
			var neutronReceiverAddr string
			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "100", // sec
					IbcTransferTimeout: "100", // sec
				}

				currentHeight := testCtx.GetNeutronHeight()
				depositBlock := Block(currentHeight + 200)
				lockupBlock := Block(currentHeight + 200)
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
				hubReceiverAddr = sideBasedHappyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				neutronReceiverAddr = sideBasedHappyCaseNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix)
				partyAConfig := InterchainCovenantParty{
					RemoteChainDenom:          "uatom",
					PartyReceiverAddr:         hubReceiverAddr,
					Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              atomCoin,
					NativeDenom:               neutronAtomIbcDenom,
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
					DenomToPfmMap:             map[string]PacketForwardMiddlewareConfig{},
				}
				partyBConfig := NativeCovenantParty{
					PartyReceiverAddr: neutronReceiverAddr,
					Addr:              neutronReceiverAddr,
					Contribution:      neutronCoin,
					NativeDenom:       cosmosNeutron.Config().Denom,
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

				liquidPoolerConfig := LiquidPoolerConfig{
					Astroport: &AstroportLiquidPoolerConfig{
						PairType:    pairType,
						PoolAddress: poolAddress,
						AssetADenom: neutronAtomIbcDenom,
						AssetBDenom: cosmosNeutron.Config().Denom,
						SingleSideLpLimits: SingleSideLpLimits{
							AssetALimit: "100000",
							AssetBLimit: "100000",
						},
					},
				}

				covenantMsg := CovenantInstantiateMsg{
					Label:           "two-party-pol-covenant-side-happy",
					Timeouts:        timeouts,
					PresetIbcFee:    presetIbcFee,
					ContractCodeIds: codeIds,
					LockupConfig:    lockupConfig,
					PartyAConfig:    CovenantPartyConfig{Interchain: &partyAConfig},
					PartyBConfig:    CovenantPartyConfig{Native: &partyBConfig},
					RagequitConfig:  &ragequitConfig,
					DepositDeadline: depositDeadline,
					PartyAShare:     "50",
					PartyBShare:     "50",
					PoolPriceConfig: PoolPriceConfig{
						ExpectedSpotPrice:     "0.1",
						AcceptablePriceSpread: "0.09",
					},
					CovenantType: "side",
					Splits: map[string]SplitConfig{
						neutronAtomIbcDenom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:     "1.0",
								neutronReceiverAddr: "0.0",
							},
						},
						cosmosNeutron.Config().Denom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:     "0.0",
								neutronReceiverAddr: "1.0",
							},
						},
					},
					FallbackSplit:      nil,
					LiquidPoolerConfig: liquidPoolerConfig,
				}

				covenantAddress = testCtx.ManualInstantiate(covenantSideBasedRqCodeId, covenantMsg, neutronUser, keyring.BackendTest)
				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.QueryClockAddress(covenantAddress)
				holderAddress = testCtx.QueryHolderAddress(covenantAddress)
				liquidPoolerAddress = testCtx.QueryLiquidPoolerAddress(covenantAddress)
				partyARouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_a")
				partyBRouterAddress = testCtx.QueryInterchainRouterAddress(covenantAddress, "party_b")
				partyAIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_a")
				partyBIbcForwarderAddress = testCtx.QueryIbcForwarderAddress(covenantAddress, "party_b")
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

				testCtx.FundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)

				testCtx.SkipBlocks(2)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				for {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					forwarderAState := testCtx.QueryContractState(partyAIbcForwarderAddress)

					if forwarderAState == "ica_created" {
						testCtx.SkipBlocks(5)
						partyADepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_a")
						partyBDepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_b")
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
				testCtx.FundChainAddrs([]string{partyBDepositAddress}, cosmosNeutron, sideBasedHappyCaseNeutronAccount, int64(neutronContributionAmount))
				testCtx.FundChainAddrs([]string{partyADepositAddress}, cosmosAtom, sideBasedHappyCaseHubAccount, int64(atomContributionAmount))

				testCtx.SkipBlocks(3)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					holderNeutronBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
					holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
					holderState := testCtx.QueryContractState(holderAddress)

					println("holder atom bal: ", holderAtomBal)
					println("holder neutron bal: ", holderNeutronBal)
					println("holder state: ", holderState)

					if holderAtomBal == atomContributionAmount && holderNeutronBal == neutronContributionAmount {
						println("holder/liquidpooler received atom & neutron")
						break
					} else if holderState == "active" {
						println("holderState: ", holderState)
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder sends the funds to LPer and receives LP tokens in return", func(t *testing.T) {
				for {
					holderLpTokenBal := testCtx.QueryLpTokenBalance(liquidityTokenAddress, liquidPoolerAddress)
					println("holder lp token balance: ", holderLpTokenBal)

					if holderLpTokenBal == 0 {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("lockup expires", func(t *testing.T) {
				for {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					holderState := testCtx.QueryContractState(holderAddress)
					println("holder state: ", holderState)
					if holderState == "expired" {
						break
					}
				}
			})

			t.Run("party A claims", func(t *testing.T) {
				testCtx.SkipBlocks(5)
				testCtx.HolderClaim(holderAddress, sideBasedHappyCaseNeutronAccount, keyring.BackendTest)
				testCtx.SkipBlocks(5)

				for {
					routerAtomBalB := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
					routerNeutronBalB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyBRouterAddress)
					routerAtomBalA := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
					routerNeutronBalA := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)

					println("routerAtomBalB: ", routerAtomBalB)
					println("routerNeutronBalB: ", routerNeutronBalB)
					println("routerAtomBalA: ", routerAtomBalA)
					println("routerNeutronBalA: ", routerNeutronBalA)

					if routerNeutronBalB != 0 {
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}

			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					neutronBalPartyB := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, neutronReceiverAddr)
					atomBalPartyA := testCtx.QueryHubDenomBalance(cosmosAtom.Config().Denom, hubReceiverAddr)

					println("party A atom bal: ", atomBalPartyA)
					println("party B neutron bal: ", neutronBalPartyB)

					if atomBalPartyA != 0 && neutronBalPartyB != 0 {
						println("nice")
						break
					} else {
						testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})
		})
	})
}
