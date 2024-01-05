package covenant_single_party_pol

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

const gaiaNeutronICSPath = "gn-ics-path"
const gaiaNeutronIBCPath = "gn-ibc-path"
const gaiaStrideIBCPath = "go-ibc-path"
const neutronStrideIBCPath = "no-ibc-path"
const nativeAtomDenom = "uatom"
const nativeStrideAtomDenom = "statom"
const nativeNtrnDenom = "untrn"

var covenantAddress string
var clockAddress string
var liquidPoolerAddress string
var partyDepositAddress string
var holderAddress string
var neutronAtomIbcDenom, neutronStatomIbcDenom, strideAtomIbcDenom string
var atomNeutronICSConnectionId, neutronAtomICSConnectionId string
var neutronStrideIBCConnId, strideNeutronIBCConnId string
var atomNeutronIBCConnId, neutronAtomIBCConnId string
var gaiaStrideIBCConnId, strideGaiaIBCConnId string
var tokenAddress string
var whitelistAddress string
var factoryAddress string
var coinRegistryAddress string
var stableswapAddress string
var liquidityTokenAddress string

// PARTY_A
const atomContributionAmount uint64 = 5_000_000_000 // in uatom

// sets up and tests a single party pol by hub
func TestSinglePartyPol(t *testing.T) {
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
			Name:    "stride",
			Version: "v14.0.0",
			ChainConfig: ibc.ChainConfig{
				Type:    "cosmos",
				Name:    "stride",
				ChainID: "stride-3",
				Denom:   "ustrd",
				ModifyGenesis: utils.SetupStrideGenesis([]string{
					"/cosmos.bank.v1beta1.MsgSend",
					"/cosmos.bank.v1beta1.MsgMultiSend",
					"/cosmos.staking.v1beta1.MsgDelegate",
					"/cosmos.staking.v1beta1.MsgUndelegate",
					"/cosmos.staking.v1beta1.MsgBeginRedelegate",
					"/cosmos.staking.v1beta1.MsgRedeemTokensforShares",
					"/cosmos.staking.v1beta1.MsgTokenizeShares",
					"/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward",
					"/cosmos.distribution.v1beta1.MsgSetWithdrawAddress",
					"/ibc.applications.transfer.v1.MsgTransfer",
				}),
				GasPrices:     "0.0ustrd",
				GasAdjustment: 1.3,
				Images: []ibc.DockerImage{
					{
						Repository: "ghcr.io/strangelove-ventures/heighliner/stride",
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
	atom, neutron, stride := chains[0], chains[1], chains[2]
	cosmosAtom, cosmosNeutron, cosmosStride := atom.(*cosmos.CosmosChain), neutron.(*cosmos.CosmosChain), stride.(*cosmos.CosmosChain)

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
		AddChain(cosmosStride).
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
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  cosmosNeutron,
			Chain2:  cosmosStride,
			Relayer: r,
			Path:    neutronStrideIBCPath,
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  cosmosAtom,
			Chain2:  cosmosStride,
			Relayer: r,
			Path:    gaiaStrideIBCPath,
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
		Stride:                    cosmosStride,
		StrideClients:             []*ibc.ClientOutput{},
		GaiaClients:               []*ibc.ClientOutput{},
		NeutronClients:            []*ibc.ClientOutput{},
		StrideConnections:         []*ibc.ConnectionOutput{},
		GaiaConnections:           []*ibc.ConnectionOutput{},
		NeutronConnections:        []*ibc.ConnectionOutput{},
		NeutronTransferChannelIds: make(map[string]string),
		GaiaTransferChannelIds:    make(map[string]string),
		StrideTransferChannelIds:  make(map[string]string),
		GaiaIcsChannelIds:         make(map[string]string),
		NeutronIcsChannelIds:      make(map[string]string),
		T:                         t,
		Ctx:                       ctx,
	}

	testCtx.SkipBlocks(5)

	t.Run("generate IBC paths", func(t *testing.T) {
		utils.GeneratePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosNeutron.Config().ChainID, gaiaNeutronIBCPath)
		utils.GeneratePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosStride.Config().ChainID, gaiaStrideIBCPath)
		utils.GeneratePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosStride.Config().ChainID, neutronStrideIBCPath)
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
		utils.GenerateClient(t, ctx, testCtx, r, eRep, neutronStrideIBCPath, cosmosNeutron, cosmosStride)
		neutronStrideIBCConnId, strideNeutronIBCConnId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, neutronStrideIBCPath, cosmosNeutron, cosmosStride)
		utils.LinkPath(t, ctx, r, eRep, cosmosNeutron, cosmosStride, neutronStrideIBCPath)

		utils.GenerateClient(t, ctx, testCtx, r, eRep, gaiaStrideIBCPath, cosmosAtom, cosmosStride)
		gaiaStrideIBCConnId, strideGaiaIBCConnId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, gaiaStrideIBCPath, cosmosAtom, cosmosStride)
		utils.LinkPath(t, ctx, r, eRep, cosmosAtom, cosmosStride, gaiaStrideIBCPath)

		utils.GenerateClient(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		atomNeutronIBCConnId, neutronAtomIBCConnId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		utils.LinkPath(t, ctx, r, eRep, cosmosAtom, cosmosNeutron, gaiaNeutronIBCPath)
	})

	// Start the relayer and clean it up when the test ends.
	err = r.StartRelayer(ctx, eRep, gaiaNeutronICSPath, gaiaNeutronIBCPath, gaiaStrideIBCPath, neutronStrideIBCPath)
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
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron, stride)
	gaiaUser, neutronUser, strideUser := users[0], users[1], users[2]
	_, _, _ = gaiaUser, neutronUser, strideUser

	testCtx.SkipBlocks(5)

	t.Run("determine ibc channels", func(t *testing.T) {
		neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)
		strideChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosStride.Config().ChainID)

		// Find all pairwise channels
		utils.GetPairwiseTransferChannelIds(testCtx, strideChannelInfo, neutronChannelInfo, strideNeutronIBCConnId, neutronStrideIBCConnId, stride.Config().Name, neutron.Config().Name)
		utils.GetPairwiseTransferChannelIds(testCtx, strideChannelInfo, gaiaChannelInfo, strideGaiaIBCConnId, gaiaStrideIBCConnId, stride.Config().Name, cosmosAtom.Config().Name)
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
		// 2. statom on neutron
		neutronStatomIbcDenom = testCtx.GetIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosStride.Config().Name],
			nativeStrideAtomDenom,
		)
		// 3. atom on stride
		strideAtomIbcDenom = testCtx.GetIbcDenom(
			testCtx.StrideTransferChannelIds[cosmosAtom.Config().Name],
			nativeAtomDenom,
		)
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
		_, _, _, _, _, _, _ = clockCodeId, interchainRouterCodeId, nativeRouterCodeId, ibcForwarderCodeId, holderCodeId, lperCodeId, covenantCodeId

		t.Run("deploy covenant contracts", func(t *testing.T) {
			covenantCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)

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
				// Add ibc native tokens for statom and uatom to the native coin registry
				// each of these tokens has a precision of 6
				addMessage := fmt.Sprintf(
					`{"add":{"native_coins":[["%s",6],["%s",6]]}}`,
					neutronAtomIbcDenom,
					neutronStatomIbcDenom)
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
				testCtx.CreateAstroportFactoryPair(3, neutronStatomIbcDenom, neutronAtomIbcDenom, factoryAddress, neutronUser, keyring.BackendTest)
			})
		})

		t.Run("fund stride user with atom to liquidstake", func(t *testing.T) {

			autopilotString := `{"autopilot":{"receiver":"` + strideUser.Bech32Address(stride.Config().Bech32Prefix) + `","stakeibc":{"stride_address":"` + strideUser.Bech32Address(stride.Config().Bech32Prefix) + `","action":"LiquidStake"}}}`
			cmd := []string{cosmosAtom.Config().Bin, "tx", "ibc-transfer", "transfer", "transfer",
				testCtx.GaiaTransferChannelIds[cosmosStride.Config().Name], autopilotString,
				"100000000000uatom",
				"--keyring-backend", keyring.BackendTest,
				"--node", cosmosAtom.GetRPCAddress(),
				"--from", gaiaUser.KeyName,
				"--gas", "auto",
				"--home", cosmosAtom.HomeDir(),
				"--chain-id", cosmosAtom.Config().ChainID,
				"-y",
			}
			_, _, err = cosmosAtom.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			testCtx.SkipBlocks(10)

			// ibc transfer statom on stride to neutron user
			transferStAtomNeutron := ibc.WalletAmount{
				Address: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				Denom:   "stuatom",
				Amount:  int64(100000000000),
			}
			_, err = cosmosStride.SendIBCTransfer(ctx, testCtx.StrideTransferChannelIds[cosmosNeutron.Config().Name], strideUser.KeyName, transferStAtomNeutron, ibc.TransferOptions{})
			require.NoError(t, err)

			testCtx.SkipBlocks(10)
		})

		t.Run("add liquidity to the atom-statom stableswap pool", func(t *testing.T) {
			liquidityTokenAddress, stableswapAddress = testCtx.QueryAstroLpTokenAndStableswapAddress(
				factoryAddress, neutronStatomIbcDenom, neutronAtomIbcDenom)
			// set up the pool with 1:10 ratio of atom/statom
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
				neutronAtomIbcDenom, neutronStatomIbcDenom, atomContributionAmount/2, atomContributionAmount/2, neutronUser, stableswapAddress)

			testCtx.SkipBlocks(2)
			neutronUserLPTokenBal := testCtx.QueryLpTokenBalance(liquidityTokenAddress, neutronUser.Bech32Address(neutron.Config().Bech32Prefix))
			println("neutronUser lp token bal: ", neutronUserLPTokenBal)
		})

	})
}
