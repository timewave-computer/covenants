package ibc_test

import (
	"context"
	"encoding/json"
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/cosmos/cosmos-sdk/crypto/keyring"
	ibctest "github.com/strangelove-ventures/interchaintest/v4"
	"github.com/strangelove-ventures/interchaintest/v4/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v4/ibc"
	"github.com/strangelove-ventures/interchaintest/v4/relayer"
	"github.com/strangelove-ventures/interchaintest/v4/relayer/rly"
	"github.com/strangelove-ventures/interchaintest/v4/testreporter"
	"github.com/strangelove-ventures/interchaintest/v4/testutil"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap"
	"go.uber.org/zap/zaptest"
)

const gaiaNeutronICSPath = "gn-ics-path"
const gaiaNeutronIBCPath = "gn-ibc-path"
const gaiaStrideIBCPath = "gs-ibc-path"
const neutronStrideIBCPath = "ns-ibc-path"
const nativeAtomDenom = "uatom"
const nativeStrideDenom = "ustrd"
const nativeNtrnDenom = "untrn"
const nativeStatomDenom = "statom"

var covenantAddress string
var clockAddress string
var partyARouterAddress, partyBRouterAddress string
var liquidPoolerAddress string
var partyAIbcForwarderAddress, partyBIbcForwarderAddress string
var partyADepositAddress, partyBDepositAddress string
var holderAddress string
var liquidStakerAddress string
var remoteChainSplitterAddress string
var neutronAtomIbcDenom, neutronStAtomIbcDenom, strdNeutronAtomIbcDenom, strideAtomIbcDenom, gaiaNeutronStrdIbcDenom string
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

const atomContributionAmount uint64 = 5_000_000_000 // in uatom

// sets up and tests a single party pol by hub on neutron
func TestTwoPartyPol(t *testing.T) {
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
						Version:    "v1.0.2",
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
					getDefaultInterchainGenesisMessages(),
				),
				ConfigFileOverrides: configFileOverrides,
			},
		},
		{
			ChainConfig: ibc.ChainConfig{
				Type:    "cosmos",
				Name:    "stride",
				ChainID: "stride-3",
				Images: []ibc.DockerImage{
					{
						Repository: "stride",
						Version:    "v9.2.1",
						UidGid:     "1025:1025",
					},
				},
				Bin:            "strided",
				Bech32Prefix:   "stride",
				Denom:          "ustrd",
				GasPrices:      "0.00ustrd",
				GasAdjustment:  1.3,
				TrustingPeriod: "330h",
				NoHostMount:    false,
				ModifyGenesis: setupStrideGenesis([]string{
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
		relayer.CustomDockerImage("ghcr.io/cosmos/relayer", "v2.3.1", rly.RlyDefaultUidGid),
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

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")

	testCtx := &TestContext{
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
	}

	t.Run("generate IBC paths", func(t *testing.T) {
		generatePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosNeutron.Config().ChainID, gaiaNeutronIBCPath)
		generatePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosStride.Config().ChainID, gaiaStrideIBCPath)
		generatePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosStride.Config().ChainID, neutronStrideIBCPath)
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
		err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
		require.NoError(t, err, "failed to wait for blocks")
	})

	t.Run("setup IBC interchain clients, connections, and links", func(t *testing.T) {
		generateClient(t, ctx, testCtx, r, eRep, neutronStrideIBCPath, cosmosNeutron, cosmosStride)
		neutronStrideIBCConnId, strideNeutronIBCConnId = generateConnections(t, ctx, testCtx, r, eRep, neutronStrideIBCPath, cosmosNeutron, cosmosStride)
		linkPath(t, ctx, r, eRep, cosmosNeutron, cosmosStride, neutronStrideIBCPath)

		generateClient(t, ctx, testCtx, r, eRep, gaiaStrideIBCPath, cosmosAtom, cosmosStride)
		gaiaStrideIBCConnId, strideGaiaIBCConnId = generateConnections(t, ctx, testCtx, r, eRep, gaiaStrideIBCPath, cosmosAtom, cosmosStride)
		linkPath(t, ctx, r, eRep, cosmosAtom, cosmosStride, gaiaStrideIBCPath)

		generateClient(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		atomNeutronIBCConnId, neutronAtomIBCConnId = generateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		linkPath(t, ctx, r, eRep, cosmosAtom, cosmosNeutron, gaiaNeutronIBCPath)
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

	err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron, stride)
	gaiaUser, neutronUser, strideUser := users[0], users[1], users[2]
	_, _ = gaiaUser, neutronUser

	strideAdminMnemonic := "tone cause tribe this switch near host damage idle fragile antique tail soda alien depth write wool they rapid unfold body scan pledge soft"
	strideAdmin, _ := ibctest.GetAndFundTestUserWithMnemonic(ctx, "default", strideAdminMnemonic, (100_000_000), cosmosStride)

	cosmosStride.SendFunds(ctx, strideUser.KeyName, ibc.WalletAmount{
		Address: strideAdmin.Bech32Address(stride.Config().Bech32Prefix),
		Denom:   "ustrd",
		Amount:  10000000,
	})

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")
	// hubNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]
	// strideNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")

	t.Run("determine ibc channels", func(t *testing.T) {
		neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)
		strideChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosStride.Config().ChainID)

		// Find all pairwise channels
		getPairwiseTransferChannelIds(testCtx, strideChannelInfo, neutronChannelInfo, strideNeutronIBCConnId, neutronStrideIBCConnId, stride.Config().Name, neutron.Config().Name)
		getPairwiseTransferChannelIds(testCtx, strideChannelInfo, gaiaChannelInfo, strideGaiaIBCConnId, gaiaStrideIBCConnId, stride.Config().Name, cosmosAtom.Config().Name)
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
		// 2. Stride on neutron
		neutronStAtomIbcDenom = testCtx.getIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosStride.Config().Name],
			nativeStatomDenom,
		)
		// 3. hub atom => neutron => stride
		strdNeutronAtomIbcDenom = testCtx.getMultihopIbcDenom(
			[]string{
				testCtx.StrideTransferChannelIds[cosmosNeutron.Config().Name],
				testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
			},
			nativeAtomDenom,
		)
		// 4. stride strd => neutron => hub
		gaiaNeutronStrdIbcDenom = testCtx.getMultihopIbcDenom(
			[]string{
				testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				testCtx.NeutronTransferChannelIds[cosmosStride.Config().Name],
			},
			nativeStrideDenom,
		)

		strideAtomIbcDenom = testCtx.getIbcDenom(
			testCtx.StrideTransferChannelIds[cosmosAtom.Config().Name],
			atom.Config().Denom,
		)
	})

	t.Run("single party pol covenant setup", func(t *testing.T) {
		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_single_party_pol.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const routerContractPath = "wasms/covenant_interchain_router.wasm"
		const ibcForwarderContractPath = "wasms/covenant_ibc_forwarder.wasm"
		const holderContractPath = "wasms/covenant_single_party_pol_holder.wasm"
		const liquidPoolerPath = "wasms/covenant_astroport_liquid_pooler.wasm"
		const remoteChainSplitterPath = "wasms/covenant_remote_chain_splitter.wasm"
		const liquidStakerPath = "wasms/covenant_liquid_staker.wasm"

		// After storing on Neutron, we will receive a code id
		// We parse all the subcontracts into uint64
		// The will be required when we instantiate the covenant.
		var clockCodeId uint64
		var routerCodeId uint64
		var ibcForwarderCodeId uint64
		var holderCodeId uint64
		var lperCodeId uint64
		var covenantCodeIdStr string
		var covenantCodeId uint64
		var liquidStakerCodeId uint64
		var remoteChainSplitterCodeId uint64

		_, _, _, _ = remoteChainSplitterCodeId, liquidStakerCodeId, covenantCodeId, lperCodeId
		_, _, _, _ = holderCodeId, ibcForwarderCodeId, routerCodeId, clockCodeId

		t.Run("deploy covenant contracts", func(t *testing.T) {
			// store covenant and get code id
			covenantCodeIdStr, err = cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, covenantContractPath)
			require.NoError(t, err, "failed to store single party pol covenant contract")
			covenantCodeId, err = strconv.ParseUint(covenantCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store clock and get code id
			clockCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, clockContractPath)
			require.NoError(t, err, "failed to store clock contract")
			clockCodeId, err = strconv.ParseUint(clockCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store router and get code id
			routerCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, routerContractPath)
			require.NoError(t, err, "failed to store router contract")
			routerCodeId, err = strconv.ParseUint(routerCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store forwarder and get code id
			ibcForwarderCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, ibcForwarderContractPath)
			require.NoError(t, err, "failed to store ibc forwarder contract")
			ibcForwarderCodeId, err = strconv.ParseUint(ibcForwarderCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store lper, get code
			lperCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, liquidPoolerPath)
			require.NoError(t, err, "failed to store liquid pooler contract")
			lperCodeId, err = strconv.ParseUint(lperCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store clock and get code id
			holderCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, holderContractPath)
			require.NoError(t, err, "failed to store single party pol holder contract")
			holderCodeId, err = strconv.ParseUint(holderCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store clock and get code id
			liquidStakerCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, liquidStakerPath)
			require.NoError(t, err, "failed to store liquid staker contract")
			liquidStakerCodeId, err = strconv.ParseUint(liquidStakerCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			require.NoError(t, testutil.WaitForBlocks(ctx, 5, cosmosNeutron, cosmosAtom, cosmosStride))
		})

		t.Run("deploy astroport contracts", func(t *testing.T) {

			stablePairCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_pair_stable.wasm")
			require.NoError(t, err, "failed to store astroport stableswap contract")
			stablePairCodeId, err := strconv.ParseUint(stablePairCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			factoryCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_factory.wasm")
			require.NoError(t, err, "failed to store astroport factory contract")

			whitelistCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_whitelist.wasm")
			require.NoError(t, err, "failed to store astroport whitelist contract")
			whitelistCodeId, err := strconv.ParseUint(whitelistCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			tokenCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_token.wasm")
			require.NoError(t, err, "failed to store astroport token contract")
			tokenCodeId, err := strconv.ParseUint(tokenCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			t.Run("astroport token", func(t *testing.T) {

				msg := NativeTokenInstantiateMsg{
					Name:            "nativetoken",
					Symbol:          "ntk",
					Decimals:        5,
					InitialBalances: []Cw20Coin{},
					Mint:            nil,
					Marketing:       nil,
				}

				str, err := json.Marshal(msg)
				require.NoError(t, err, "Failed to marshall NativeTokenInstantiateMsg")

				tokenAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, tokenCodeIdStr, string(str), true)
				require.NoError(t, err, "Failed to instantiate nativetoken")
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("whitelist", func(t *testing.T) {

				admins := []string{neutronUser.Bech32Address(neutron.Config().Bech32Prefix)}

				msg := WhitelistInstantiateMsg{
					Admins:  admins,
					Mutable: false,
				}

				str, err := json.Marshal(msg)
				require.NoError(t, err, "Failed to marshall WhitelistInstantiateMsg")

				whitelistAddress, err = cosmosNeutron.InstantiateContract(
					ctx, neutronUser.KeyName, whitelistCodeIdStr, string(str), true)
				require.NoError(t, err, "Failed to instantiate Whitelist")
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("native coins registry", func(t *testing.T) {
				coinRegistryCodeId, err := cosmosNeutron.StoreContract(
					ctx, neutronUser.KeyName, "wasms/astroport_native_coin_registry.wasm")
				require.NoError(t, err, "failed to store astroport native coin registry contract")

				msg := NativeCoinRegistryInstantiateMsg{
					Owner: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				}
				str, err := json.Marshal(msg)
				require.NoError(t, err, "Failed to marshall NativeCoinRegistryInstantiateMsg")

				nativeCoinRegistryAddress, err := cosmosNeutron.InstantiateContract(
					ctx, neutronUser.KeyName, coinRegistryCodeId, string(str), true)
				require.NoError(t, err, "Failed to instantiate NativeCoinRegistry")
				coinRegistryAddress = nativeCoinRegistryAddress
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("add coins to registry", func(t *testing.T) {
				// Add ibc native tokens for uatom and statom to the native coin registry
				// each of these tokens has a precision of 6
				addMessage := `{"add":{"native_coins":[["` + neutronAtomIbcDenom + `",6],["` + neutronStAtomIbcDenom + `",6]]}}`
				addCmd := []string{"neutrond", "tx", "wasm", "execute",
					coinRegistryAddress,
					addMessage,
					"--from", neutronUser.KeyName,
					"--gas-prices", "0.0untrn",
					"--gas-adjustment", `1.5`,
					"--output", "json",
					"--home", "/var/cosmos-chain/neutron-2",
					"--node", neutron.GetRPCAddress(),
					"--home", neutron.HomeDir(),
					"--chain-id", neutron.Config().ChainID,
					"--from", neutronUser.KeyName,
					"--gas", "auto",
					"--keyring-backend", keyring.BackendTest,
					"-y",
				}
				_, _, err = cosmosNeutron.Exec(ctx, addCmd, nil)
				require.NoError(t, err, err)
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("factory", func(t *testing.T) {
				pairConfigs := []PairConfig{
					PairConfig{
						CodeId: stablePairCodeId,
						PairType: PairType{
							Stable: struct{}{},
						},
						TotalFeeBps:         0,
						MakerFeeBps:         0,
						IsDisabled:          false,
						IsGeneratorDisabled: true,
					},
				}

				msg := FactoryInstantiateMsg{
					PairConfigs:         pairConfigs,
					TokenCodeId:         tokenCodeId,
					FeeAddress:          nil,
					GeneratorAddress:    nil,
					Owner:               neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
					WhitelistCodeId:     whitelistCodeId,
					CoinRegistryAddress: coinRegistryAddress,
				}

				str, err := json.Marshal(msg)
				require.NoError(t, err, "Failed to marshall FactoryInstantiateMsg")

				factoryAddr, err := cosmosNeutron.InstantiateContract(
					ctx, neutronUser.KeyName, factoryCodeIdStr, string(str), true)
				require.NoError(t, err, "Failed to instantiate Factory")
				factoryAddress = factoryAddr
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("create pair on factory", func(t *testing.T) {

				initParams := StablePoolParams{
					Amp: 3,
				}
				binaryData, err := json.Marshal(initParams)
				require.NoError(t, err, "error encoding stable pool params to binary")

				statomNativeToken := NativeToken{
					Denom: neutronStAtomIbcDenom,
				}
				atomNativeToken := NativeToken{
					Denom: neutronAtomIbcDenom,
				}
				assetInfos := []AssetInfo{
					{
						NativeToken: &statomNativeToken,
					},
					{
						NativeToken: &atomNativeToken,
					},
				}

				initPairMsg := CreatePair{
					PairType: PairType{
						Stable: struct{}{},
					},
					AssetInfos: assetInfos,
					InitParams: binaryData,
				}

				createPairMsg := CreatePairMsg{
					CreatePair: initPairMsg,
				}

				str, err := json.Marshal(createPairMsg)
				require.NoError(t, err, "Failed to marshall CreatePair message")

				createCmd := []string{"neutrond", "tx", "wasm", "execute",
					factoryAddress,
					string(str),
					"--from", neutronUser.KeyName,
					"--gas-prices", "0.0untrn",
					"--gas-adjustment", `1.5`,
					"--output", "json",
					"--home", "/var/cosmos-chain/neutron-2",
					"--node", neutron.GetRPCAddress(),
					"--home", neutron.HomeDir(),
					"--chain-id", neutron.Config().ChainID,
					"--from", neutronUser.KeyName,
					"--gas", "auto",
					"--keyring-backend", keyring.BackendTest,
					"-y",
				}

				_, _, err = cosmosNeutron.Exec(ctx, createCmd, nil)
				require.NoError(t, err, err)
				err = testutil.WaitForBlocks(ctx, 20, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
			})
		})

		t.Run("register stride host zone", func(t *testing.T) {

			cmd := []string{"strided", "tx", "stakeibc", "register-host-zone",
				strideGaiaIBCConnId,
				cosmosAtom.Config().Denom,
				cosmosAtom.Config().Bech32Prefix,
				strideAtomIbcDenom,
				testCtx.StrideTransferChannelIds[cosmosAtom.Config().Name],
				"1",
				"--from", strideAdmin.KeyName,
				"--gas", "auto",
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--chain-id", cosmosStride.Config().ChainID,
				"--node", cosmosStride.GetRPCAddress(),
				"--home", cosmosStride.HomeDir(),
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err = cosmosStride.Exec(ctx, cmd, nil)
			require.NoError(t, err, "failed to register host zone on stride")

			err = testutil.WaitForBlocks(ctx, 5, stride)
			require.NoError(t, err, "failed to wait for blocks")
		})

		// Stride needs validators that it can stake ATOM with to issue us stATOM
		t.Run("register gaia validators on stride", func(t *testing.T) {

			type Validator struct {
				Name    string `json:"name"`
				Address string `json:"address"`
				Weight  int    `json:"weight"`
			}

			type Data struct {
				BlockHeight string      `json:"block_height"`
				Total       string      `json:"total"`
				Validators  []Validator `json:"validators"`
			}

			valcmd := []string{"gaiad", "query", "tendermint-validator-set",
				"50",
				"--chain-id", cosmosAtom.Config().ChainID,
				"--node", cosmosAtom.GetRPCAddress(),
				"--home", cosmosAtom.HomeDir(),
			}
			resp, _, err := cosmosAtom.Exec(ctx, valcmd, nil)
			require.NoError(t, err, "Failed to query valset")
			err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			var addresses []string
			var votingPowers []string

			lines := strings.Split(string(resp), "\n")

			for _, line := range lines {
				if strings.HasPrefix(line, "- address: ") {
					address := strings.TrimPrefix(line, "- address: ")
					addresses = append(addresses, address)
				} else if strings.HasPrefix(line, "  voting_power: ") {
					votingPower := strings.TrimPrefix(line, "  voting_power: ")
					votingPowers = append(votingPowers, votingPower)
				}
			}

			// Create validators slice
			var validators []Validator

			for i := 1; i <= len(addresses); i++ {
				votingPowStr := strings.ReplaceAll(votingPowers[i-1], "\"", "")
				valWeight, err := strconv.Atoi(votingPowStr)
				require.NoError(t, err, "failed to parse voting power")

				validator := Validator{
					Name:    fmt.Sprintf("val%d", i),
					Address: addresses[i-1],
					Weight:  valWeight,
				}
				validators = append(validators, validator)
			}

			// Create JSON object
			data := map[string][]Validator{
				"validators": validators,
			}

			// Convert to JSON
			jsonData, err := json.Marshal(data)
			require.NoError(t, err, "failed to marshall data")

			fullPath := filepath.Join(cosmosStride.HomeDir(), "vals.json")
			bashCommand := "echo '" + string(jsonData) + "' > " + fullPath
			fullPathCmd := []string{"/bin/sh", "-c", bashCommand}

			_, _, err = cosmosStride.Exec(ctx, fullPathCmd, nil)
			require.NoError(t, err, "failed to create json with gaia LS validator set on stride")

			err = testutil.WaitForBlocks(ctx, 5, neutron, atom, stride)
			require.NoError(t, err, "failed to wait for blocks")

			cmd := []string{"strided", "tx", "stakeibc", "add-validators",
				cosmosAtom.Config().ChainID,
				fullPath,
				"--from", strideAdmin.KeyName,
				"--gas", "auto",
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--chain-id", cosmosStride.Config().ChainID,
				"--node", cosmosStride.GetRPCAddress(),
				"--home", cosmosStride.HomeDir(),
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err = cosmosStride.Exec(ctx, cmd, nil)
			require.NoError(t, err, "failed to register host zone on stride")

			err = testutil.WaitForBlocks(ctx, 5, stride)
			require.NoError(t, err, "failed to wait for blocks")

			queryCmd := []string{"strided", "query", "stakeibc",
				"show-validators",
				cosmosAtom.Config().ChainID,
				"--chain-id", cosmosStride.Config().ChainID,
				"--node", cosmosStride.GetRPCAddress(),
				"--home", cosmosStride.HomeDir(),
			}

			_, _, err = cosmosStride.Exec(ctx, queryCmd, nil)
			require.NoError(t, err, "failed to query host validators")
		})

		t.Run("add liquidity to the atom-statom stableswap pool", func(t *testing.T) {
			// query neutronUser balance of lp tokens

			stAtom := NativeToken{
				Denom: neutronStAtomIbcDenom,
			}
			nativeAtom := NativeToken{
				Denom: neutronAtomIbcDenom,
			}
			assetInfos := []AssetInfo{
				{
					NativeToken: &stAtom,
				},
				{
					NativeToken: &nativeAtom,
				},
			}
			pair := Pair{
				AssetInfos: assetInfos,
			}
			pairQueryMsg := PairQuery{
				Pair: pair,
			}
			queryJson, _ := json.Marshal(pairQueryMsg)

			queryCmd := []string{"neutrond", "query", "wasm", "contract-state", "smart",
				factoryAddress, string(queryJson),
			}

			print("\n factory query cmd: ", string(strings.Join(queryCmd, " ")), "\n")

			factoryQueryRespBytes, _, _ := neutron.Exec(ctx, queryCmd, nil)
			print(string(factoryQueryRespBytes))

			var response FactoryPairResponse
			err = cosmosNeutron.QueryContract(ctx, factoryAddress, pairQueryMsg, &response)
			stableswapAddress = response.Data.ContractAddr
			print("\n stableswap address: ", stableswapAddress, "\n")
			liquidityTokenAddress = response.Data.LiquidityToken
			print("\n liquidity token: ", liquidityTokenAddress, "\n")

			require.NoError(t, err, "failed to query pair info")
			jsonResp, _ := json.Marshal(response)
			print("\npair info: ", string(jsonResp), "\n")

			// lets set up the pool with 100K each of atom/statom
			// ibc transfer 100K atom to neutron user
			transferNeutron := ibc.WalletAmount{
				Address: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				Denom:   atom.Config().Denom,
				Amount:  int64(100_000_000_000),
			}
			_, err := atom.SendIBCTransfer(ctx, testCtx.GaiaTransferChannelIds[neutron.Config().Name], gaiaUser.KeyName, transferNeutron, ibc.TransferOptions{})
			require.NoError(t, err)

			testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)

			// send 100K atom to stride which we can liquid stake
			autopilotString := `{"autopilot":{"receiver":"` + strideUser.Bech32Address(stride.Config().Bech32Prefix) + `","stakeibc":{"stride_address":"` + strideUser.Bech32Address(stride.Config().Bech32Prefix) + `","action":"LiquidStake"}}}`
			cmd := []string{atom.Config().Bin, "tx", "ibc-transfer", "transfer", "transfer", testCtx.GaiaTransferChannelIds[stride.Config().Name], autopilotString,
				"100000000000uatom",
				"--keyring-backend", keyring.BackendTest,
				"--node", atom.GetRPCAddress(),
				"--from", gaiaUser.KeyName,
				"--gas", "auto",
				"--home", atom.HomeDir(),
				"--chain-id", atom.Config().ChainID,
				"-y",
			}
			_, _, err = atom.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)

			// ibc transfer statom on stride to neutron user
			transferStAtomNeutron := ibc.WalletAmount{
				Address: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				Denom:   "stuatom",
				Amount:  int64(100000000000),
			}
			_, err = stride.SendIBCTransfer(ctx, testCtx.StrideTransferChannelIds[neutron.Config().Name], strideUser.KeyName, transferStAtomNeutron, ibc.TransferOptions{})
			require.NoError(t, err)

			testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)

			// join pool
			assets := []AstroportAsset{
				AstroportAsset{
					Info: AssetInfo{
						NativeToken: &NativeToken{
							Denom: neutronAtomIbcDenom,
						},
					},
					Amount: "100000000000",
				},
				AstroportAsset{
					Info: AssetInfo{
						NativeToken: &NativeToken{
							Denom: neutronStAtomIbcDenom,
						},
					},
					Amount: "100000000000",
				},
			}

			msg := ProvideLiqudityMsg{
				ProvideLiquidity: ProvideLiquidityStruct{
					Assets:            assets,
					SlippageTolerance: "0.01",
					AutoStake:         false,
					Receiver:          neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				},
			}

			str, err := json.Marshal(msg)
			require.NoError(t, err, "Failed to marshall provide liquidity msg")
			amountStr := "100000000000" + neutronAtomIbcDenom + "," + "100000000000" + neutronStAtomIbcDenom

			cmd = []string{"neutrond", "tx", "wasm", "execute", stableswapAddress,
				string(str),
				"--from", neutronUser.KeyName,
				"--amount", amountStr,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--chain-id", neutron.Config().ChainID,
				"--gas", "900000",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}
			resp, _, err := cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			jsonResp, _ = json.Marshal(resp)
			print("\nprovide liquidity response: ", string(jsonResp), "\n")

			testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)

		})

	})

}
