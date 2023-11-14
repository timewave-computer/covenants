package ibc_test

import (
	"context"
	"encoding/json"
	"fmt"
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
const gaiaOsmosisIBCPath = "go-ibc-path"
const neutronOsmosisIBCPath = "no-ibc-path"
const nativeAtomDenom = "uatom"
const nativeOsmoDenom = "uosmo"
const nativeNtrnDenom = "untrn"

var covenantAddress string
var clockAddress string
var partyARouterAddress, partyBRouterAddress string
var liquidPoolerAddress string
var partyAIbcForwarderAddress, partyBIbcForwarderAddress string
var partyADepositAddress, partyBDepositAddress string
var holderAddress string
var neutronAtomIbcDenom, neutronOsmoIbcDenom, osmoNeutronAtomIbcDenom, gaiaNeutronOsmoIbcDenom string
var atomNeutronICSConnectionId, neutronAtomICSConnectionId string
var neutronOsmosisIBCConnId, osmosisNeutronIBCConnId string
var atomNeutronIBCConnId, neutronAtomIBCConnId string
var gaiaOsmosisIBCConnId, osmosisGaiaIBCConnId string
var tokenAddress string
var whitelistAddress string
var factoryAddress string
var coinRegistryAddress string
var stableswapAddress string
var liquidityTokenAddress string

// PARTY_A
const atomContributionAmount uint64 = 5_000_000_000 // in uatom

// PARTY_B
const osmoContributionAmount uint64 = 50_000_000_000 // in uosmo

// sets up and tests a two party pol between hub and osmo facilitated by neutron
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
		relayer.CustomDockerImage("ghcr.io/cosmos/relayer", "v2.3.1", rly.RlyDefaultUidGid),
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
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  cosmosNeutron,
			Chain2:  cosmosOsmosis,
			Relayer: r,
			Path:    neutronOsmosisIBCPath,
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  cosmosAtom,
			Chain2:  cosmosOsmosis,
			Relayer: r,
			Path:    gaiaOsmosisIBCPath,
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

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis)
	require.NoError(t, err, "failed to wait for blocks")

	testCtx := &TestContext{
		Neutron:                   cosmosNeutron,
		Hub:                       cosmosAtom,
		Osmosis:                   cosmosOsmosis,
		OsmoClients:               []*ibc.ClientOutput{},
		GaiaClients:               []*ibc.ClientOutput{},
		NeutronClients:            []*ibc.ClientOutput{},
		OsmoConnections:           []*ibc.ConnectionOutput{},
		GaiaConnections:           []*ibc.ConnectionOutput{},
		NeutronConnections:        []*ibc.ConnectionOutput{},
		NeutronTransferChannelIds: make(map[string]string),
		GaiaTransferChannelIds:    make(map[string]string),
		OsmoTransferChannelIds:    make(map[string]string),
		GaiaIcsChannelIds:         make(map[string]string),
		NeutronIcsChannelIds:      make(map[string]string),
		t:                         t,
		ctx:                       ctx,
	}

	t.Run("generate IBC paths", func(t *testing.T) {
		generatePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosNeutron.Config().ChainID, gaiaNeutronIBCPath)
		generatePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosOsmosis.Config().ChainID, gaiaOsmosisIBCPath)
		generatePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosOsmosis.Config().ChainID, neutronOsmosisIBCPath)
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
		err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
		require.NoError(t, err, "failed to wait for blocks")
	})

	t.Run("setup IBC interchain clients, connections, and links", func(t *testing.T) {
		generateClient(t, ctx, testCtx, r, eRep, neutronOsmosisIBCPath, cosmosNeutron, cosmosOsmosis)
		neutronOsmosisIBCConnId, osmosisNeutronIBCConnId = generateConnections(t, ctx, testCtx, r, eRep, neutronOsmosisIBCPath, cosmosNeutron, cosmosOsmosis)
		linkPath(t, ctx, r, eRep, cosmosNeutron, cosmosOsmosis, neutronOsmosisIBCPath)

		generateClient(t, ctx, testCtx, r, eRep, gaiaOsmosisIBCPath, cosmosAtom, cosmosOsmosis)
		gaiaOsmosisIBCConnId, osmosisGaiaIBCConnId = generateConnections(t, ctx, testCtx, r, eRep, gaiaOsmosisIBCPath, cosmosAtom, cosmosOsmosis)
		linkPath(t, ctx, r, eRep, cosmosAtom, cosmosOsmosis, gaiaOsmosisIBCPath)

		generateClient(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		atomNeutronIBCConnId, neutronAtomIBCConnId = generateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		linkPath(t, ctx, r, eRep, cosmosAtom, cosmosNeutron, gaiaNeutronIBCPath)
	})

	// Start the relayer and clean it up when the test ends.
	err = r.StartRelayer(ctx, eRep, gaiaNeutronICSPath, gaiaNeutronIBCPath, gaiaOsmosisIBCPath, neutronOsmosisIBCPath)
	require.NoError(t, err, "failed to start relayer with given paths")
	t.Cleanup(func() {
		err = r.StopRelayer(ctx, eRep)
		if err != nil {
			t.Logf("failed to stop relayer: %s", err)
		}
	})

	err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
	require.NoError(t, err, "failed to wait for blocks")

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron, osmosis)
	gaiaUser, neutronUser, osmoUser := users[0], users[1], users[2]
	_, _, _ = gaiaUser, neutronUser, osmoUser
	hubNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]
	osmoNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]

	rqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount+1), atom)[0]
	rqCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(osmoContributionAmount+1), osmosis)[0]

	sideBasedRqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount+1), atom)[0]
	sideBasedRqCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(osmoContributionAmount+1), osmosis)[0]

	happyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount+1), atom)[0]
	happyCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(osmoContributionAmount+1), osmosis)[0]

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis)
	require.NoError(t, err, "failed to wait for blocks")

	t.Run("determine ibc channels", func(t *testing.T) {
		neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)
		osmoChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosOsmosis.Config().ChainID)

		// Find all pairwise channels
		getPairwiseTransferChannelIds(testCtx, osmoChannelInfo, neutronChannelInfo, osmosisNeutronIBCConnId, neutronOsmosisIBCConnId, osmosis.Config().Name, neutron.Config().Name)
		getPairwiseTransferChannelIds(testCtx, osmoChannelInfo, gaiaChannelInfo, osmosisGaiaIBCConnId, gaiaOsmosisIBCConnId, osmosis.Config().Name, cosmosAtom.Config().Name)
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
		// 2. Osmo on neutron
		neutronOsmoIbcDenom = testCtx.getIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
			nativeOsmoDenom,
		)
		// 3. hub atom => neutron => osmosis
		osmoNeutronAtomIbcDenom = testCtx.getMultihopIbcDenom(
			[]string{
				testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
				testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
			},
			nativeAtomDenom,
		)
		// 4. osmosis osmo => neutron => hub
		gaiaNeutronOsmoIbcDenom = testCtx.getMultihopIbcDenom(
			[]string{
				testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
			},
			nativeOsmoDenom,
		)
	})

	t.Run("two party pol covenant setup", func(t *testing.T) {
		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_two_party_pol.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const routerContractPath = "wasms/covenant_interchain_router.wasm"
		const ibcForwarderContractPath = "wasms/covenant_ibc_forwarder.wasm"
		const holderContractPath = "wasms/covenant_two_party_pol_holder.wasm"
		const liquidPoolerPath = "wasms/covenant_astroport_liquid_pooler.wasm"

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

		var covenantRqCodeIdStr string
		var covenantRqCodeId uint64
		var covenantSideBasedRqCodeIdStr string
		var covenantSideBasedRqCodeId uint64
		_ = covenantCodeId
		_ = covenantRqCodeId
		_ = covenantSideBasedRqCodeId

		queryLpTokenBalance := func(token string, addr string) string {
			bal := Balance{
				Address: addr,
			}

			balanceQueryMsg := Cw20QueryMsg{
				Balance: bal,
			}
			var response Cw20BalanceResponse
			err = cosmosNeutron.QueryContract(ctx, token, balanceQueryMsg, &response)
			require.NoError(t, err, "failed to query lp token balance")
			jsonResp, _ := json.Marshal(response)
			print("\n balance response: ", string(jsonResp), "\n")
			return response.Data.Balance
		}

		t.Run("deploy covenant contracts", func(t *testing.T) {
			// store covenant and get code id
			covenantCodeIdStr, err = cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, covenantContractPath)
			require.NoError(t, err, "failed to store two party pol covenant contract")
			covenantCodeId, err = strconv.ParseUint(covenantCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			covenantRqCodeIdStr, err = cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, covenantContractPath)
			require.NoError(t, err, "failed to store two party pol covenant contract")
			covenantRqCodeId, err = strconv.ParseUint(covenantRqCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			covenantSideBasedRqCodeIdStr, err = cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, covenantContractPath)
			require.NoError(t, err, "failed to store two party pol covenant contract")
			covenantSideBasedRqCodeId, err = strconv.ParseUint(covenantSideBasedRqCodeIdStr, 10, 64)
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
			require.NoError(t, err, "failed to store two party pol holder contract")
			holderCodeId, err = strconv.ParseUint(holderCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			require.NoError(t, testutil.WaitForBlocks(ctx, 5, cosmosNeutron, cosmosAtom, cosmosOsmosis))
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("add coins to registry", func(t *testing.T) {
				// Add ibc native tokens for uosmo and uatom to the native coin registry
				// each of these tokens has a precision of 6
				addMessage := `{"add":{"native_coins":[["` + neutronAtomIbcDenom + `",6],["` + neutronOsmoIbcDenom + `",6]]}}`
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("create pair on factory", func(t *testing.T) {

				initParams := StablePoolParams{
					Amp: 3,
				}
				binaryData, err := json.Marshal(initParams)
				require.NoError(t, err, "error encoding stable pool params to binary")

				osmoNativeToken := NativeToken{
					Denom: neutronOsmoIbcDenom,
				}
				atomNativeToken := NativeToken{
					Denom: neutronAtomIbcDenom,
				}
				assetInfos := []AssetInfo{
					{
						NativeToken: &osmoNativeToken,
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
				err = testutil.WaitForBlocks(ctx, 20, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")
			})
		})

		t.Run("add liquidity to the atom-osmo stableswap pool", func(t *testing.T) {
			osmoNativeToken := NativeToken{
				Denom: neutronOsmoIbcDenom,
			}
			atomNativeToken := NativeToken{
				Denom: neutronAtomIbcDenom,
			}
			assetInfos := []AssetInfo{
				{
					NativeToken: &osmoNativeToken,
				},
				{
					NativeToken: &atomNativeToken,
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

			// set up the pool with 1:10 ratio of atom/osmo
			transferAtom := ibc.WalletAmount{
				Address: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				Denom:   cosmosAtom.Config().Denom,
				Amount:  int64(atomContributionAmount),
			}
			_, err := atom.SendIBCTransfer(ctx,
				testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				gaiaUser.KeyName,
				transferAtom,
				ibc.TransferOptions{})
			require.NoError(t, err)

			transferOsmo := ibc.WalletAmount{
				Address: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
				Denom:   osmosis.Config().Denom,
				Amount:  int64(osmoContributionAmount),
			}

			_, err = osmosis.SendIBCTransfer(ctx,
				testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
				osmoUser.KeyName,
				transferOsmo,
				ibc.TransferOptions{})
			require.NoError(t, err)

			testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis)

			// join pool
			assets := []AstroportAsset{
				AstroportAsset{
					Info: AssetInfo{
						NativeToken: &NativeToken{
							Denom: neutronAtomIbcDenom,
						},
					},
					Amount: strconv.FormatUint(atomContributionAmount, 10),
				},
				AstroportAsset{
					Info: AssetInfo{
						NativeToken: &NativeToken{
							Denom: neutronOsmoIbcDenom,
						},
					},
					Amount: strconv.FormatUint(osmoContributionAmount, 10),
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
			amountStr := strconv.FormatUint(atomContributionAmount, 10) + neutronAtomIbcDenom + "," + strconv.FormatUint(osmoContributionAmount, 10) + neutronOsmoIbcDenom

			cmd := []string{"neutrond", "tx", "wasm", "execute", stableswapAddress,
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
			println("liq provision msg: \n ", strings.Join(cmd, " "), "\n")

			resp, _, err := cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			jsonResp, _ = json.Marshal(resp)
			print("\nprovide liquidity response: ", string(jsonResp), "\n")

			testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis)
			neutronUserLPTokenBal := queryLpTokenBalance(liquidityTokenAddress, neutronUser.Bech32Address(neutron.Config().Bech32Prefix))
			println("neutronUser lp token bal: ", neutronUserLPTokenBal)
		})

		t.Run("two party POL happy path", func(t *testing.T) {

			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "100", // sec
					IbcTransferTimeout: "100", // sec
				}

				depositBlock := Block(500)
				lockupBlock := Block(500)

				lockupConfig := Expiration{
					AtHeight: &lockupBlock,
				}
				depositDeadline := Expiration{
					AtHeight: &depositBlock,
				}
				presetIbcFee := PresetIbcFee{
					AckFee:     "10000",
					TimeoutFee: "10000",
				}

				atomCoin := Coin{
					Denom:  cosmosAtom.Config().Denom,
					Amount: strconv.FormatUint(atomContributionAmount, 10),
				}

				osmoCoin := Coin{
					Denom:  cosmosOsmosis.Config().Denom,
					Amount: strconv.FormatUint(osmoContributionAmount, 10),
				}

				hubReceiverAddr := happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				osmoReceiverAddr := happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)

				partyAConfig := CovenantPartyConfig{
					ControllerAddr:            hubReceiverAddr,
					HostAddr:                  hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              atomCoin,
					IbcDenom:                  neutronAtomIbcDenom,
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyReceiverAddr:         hubReceiverAddr,
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				}
				partyBConfig := CovenantPartyConfig{
					ControllerAddr:            osmoReceiverAddr,
					HostAddr:                  osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              osmoCoin,
					IbcDenom:                  neutronOsmoIbcDenom,
					PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
					PartyReceiverAddr:         osmoReceiverAddr,
					PartyChainConnectionId:    neutronOsmosisIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				}
				codeIds := ContractCodeIds{
					IbcForwarderCode:     ibcForwarderCodeId,
					InterchainRouterCode: routerCodeId,
					ClockCode:            clockCodeId,
					HolderCode:           holderCodeId,
					LiquidPoolerCode:     lperCodeId,
				}

				ragequitTerms := RagequitTerms{
					Penalty:      "0.1",
					RagequitType: "share",
				}

				ragequitConfig := RagequitConfig{
					Enabled: &ragequitTerms,
				}

				poolAddress := stableswapAddress
				pairType := PairType{
					Stable: struct{}{},
				}

				covenantMsg := CovenantInstantiateMsg{
					Label:                    "two-party-pol-covenant-happy",
					Timeouts:                 timeouts,
					PresetIbcFee:             presetIbcFee,
					ContractCodeIds:          codeIds,
					LockupConfig:             lockupConfig,
					PartyAConfig:             partyAConfig,
					PartyBConfig:             partyBConfig,
					PoolAddress:              poolAddress,
					RagequitConfig:           &ragequitConfig,
					DepositDeadline:          depositDeadline,
					PartyAShare:              "50",
					PartyBShare:              "50",
					ExpectedPoolRatio:        "0.1",
					AcceptablePoolRatioDelta: "0.09",
					PairType:                 pairType,
					Splits: []DenomSplit{
						{
							Denom: neutronAtomIbcDenom,
							Type: SplitType{
								Custom: SplitConfig{
									Receivers: []Receiver{
										{
											Address: hubReceiverAddr,
											Share:   "1.0",
										},
									},
								},
							},
						},
						{
							Denom: neutronOsmoIbcDenom,
							Type: SplitType{
								Custom: SplitConfig{
									Receivers: []Receiver{
										{
											Address: osmoReceiverAddr,
											Share:   "1.0",
										},
									},
								},
							},
						},
					},
					FallbackSplit: nil,
				}
				str, err := json.Marshal(covenantMsg)
				require.NoError(t, err, "Failed to marshall CovenantInstantiateMsg")
				instantiateMsg := string(str)

				println("instantiation message: ", instantiateMsg)

				covenantAddress = testCtx.manualInstantiate(covenantCodeIdStr, instantiateMsg, neutronUser, keyring.BackendTest)

				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.queryClockAddress(covenantAddress)
				println("clock addr: ", clockAddress)

				holderAddress = testCtx.queryHolderAddress(covenantAddress)
				println("holder addr: ", holderAddress)

				liquidPoolerAddress = testCtx.queryLiquidPoolerAddress(covenantAddress)
				println("liquid pooler addr: ", liquidPoolerAddress)

				partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
				println("partyARouterAddress: ", partyARouterAddress)

				partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
				println("partyBRouterAddress: ", partyBRouterAddress)

				partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
				println("partyAIbcForwarderAddress: ", partyAIbcForwarderAddress)

				partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
				println("partyBIbcForwarderAddress: ", partyBIbcForwarderAddress)
			})

			t.Run("fund contracts with neutron", func(t *testing.T) {
				err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyAIbcForwarderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to partyAIbcForwarder contract")

				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyBIbcForwarderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to partyBIbcForwarder contract")

				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: clockAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to clock contract")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyARouterAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to party a router")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyBRouterAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to party b router")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: holderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to holder")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: liquidPoolerAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to holder")

				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				bal, err := neutron.GetBalance(ctx, partyAIbcForwarderAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyBIbcForwarderAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, clockAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyARouterAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyBRouterAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				require.NoError(t, testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis), "failed to wait for blocks")
				for {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
					forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

					if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
						require.NoError(t, testutil.WaitForBlocks(ctx, 3, atom, neutron, osmosis), "failed to wait for blocks")
						partyADepositAddress = testCtx.queryDepositAddress(partyAIbcForwarderAddress)
						partyBDepositAddress = testCtx.queryDepositAddress(partyBIbcForwarderAddress)
						println("both parties icas created: ", partyADepositAddress, " , ", partyBDepositAddress)
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {

				err := cosmosOsmosis.SendFunds(ctx, happyCaseOsmoAccount.KeyName, ibc.WalletAmount{
					Address: partyBDepositAddress,
					Denom:   nativeOsmoDenom,
					Amount:  int64(osmoContributionAmount),
				})
				require.NoError(t, err, "failed to fund osmo forwarder")
				err = cosmosAtom.SendFunds(ctx, happyCaseHubAccount.KeyName, ibc.WalletAmount{
					Address: partyADepositAddress,
					Denom:   nativeAtomDenom,
					Amount:  int64(atomContributionAmount),
				})
				require.NoError(t, err, "failed to fund gaia forwarder")

				err = testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				bal, err := cosmosAtom.GetBalance(ctx, partyADepositAddress, nativeAtomDenom)
				require.NoError(t, err, "failed to query bal")
				require.Equal(t, int64(atomContributionAmount), bal)
				bal, err = cosmosOsmosis.GetBalance(ctx, partyBDepositAddress, nativeOsmoDenom)
				require.NoError(t, err, "failed to query bal")
				require.Equal(t, int64(osmoContributionAmount), bal)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					holderOsmoBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronOsmoIbcDenom)
					require.NoError(t, err, "failed to query holder osmo bal")
					holderAtomBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronAtomIbcDenom)
					require.NoError(t, err, "failed to query holder atom bal")
					println("holder atom bal: ", holderAtomBal)
					println("holder osmo bal: ", holderOsmoBal)

					holderState := testCtx.queryContractState(holderAddress)
					println("holder state: ", holderState)

					if holderAtomBal == int64(atomContributionAmount) && holderOsmoBal == int64(osmoContributionAmount) || holderState == "active" {
						println("\nholder/liquidpooler received atom & osmo\n")
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder sends the funds to LPer", func(t *testing.T) {
				for {
					liquidPoolerOsmoBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronOsmoIbcDenom)
					require.NoError(t, err, "failed to query liquidPooler osmo bal")
					liquidPoolerAtomBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronAtomIbcDenom)
					require.NoError(t, err, "failed to query liquidPooler atom bal")
					holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderAddress)

					println("liquid pooler atom bal: ", liquidPoolerAtomBal)
					println("liquid pooler osmo bal: ", liquidPoolerOsmoBal)
					println("holder lp token balance: ", holderLpTokenBal)

					if liquidPoolerOsmoBal == int64(osmoContributionAmount) && liquidPoolerAtomBal == int64(atomContributionAmount) {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder receives LP tokens", func(t *testing.T) {
				for {
					holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderAddress)
					println("holder lp token balance: ", holderLpTokenBal)
					holderLpBal, err := strconv.ParseUint(holderLpTokenBal, 10, 64)
					if err != nil {
						panic(err)
					}

					if holderLpBal == 0 {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("tick until holder expires", func(t *testing.T) {
				for {
					neutronHeight, err := cosmosNeutron.Height(ctx)
					require.NoError(t, err)

					if neutronHeight >= 515 {
						println("neutron height: ", neutronHeight)
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("party A claims and router receives the funds", func(t *testing.T) {
				for {
					routerAtomBalA, err := cosmosNeutron.GetBalance(ctx, partyARouterAddress, neutronAtomIbcDenom)
					require.NoError(t, err)

					routerOsmoBalB, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronOsmoIbcDenom)
					require.NoError(t, err)

					println("routerAtomBalA: ", routerAtomBalA)
					println("routerOsmoBalB: ", routerOsmoBalB)

					if routerAtomBalA != 0 && routerOsmoBalB != 0 {
						break
					} else {

						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
						testCtx.holderClaim(holderAddress, hubNeutronAccount, keyring.BackendTest)

						err = testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis)
						require.NoError(t, err, "failed to wait for blocks")
					}
				}
			})

			t.Run("tick x10", func(t *testing.T) {
				i := 10
				for {

					if i == 0 {
						osmoBalPartyA, err := cosmosAtom.GetBalance(
							ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom,
						)
						require.NoError(t, err)

						osmoBalPartyB, err := cosmosOsmosis.GetBalance(
							ctx, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
						)
						require.NoError(t, err)

						atomBalPartyA, err := cosmosAtom.GetBalance(
							ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
						)
						require.NoError(t, err)

						atomBalPartyB, err := cosmosOsmosis.GetBalance(
							ctx, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom,
						)
						require.NoError(t, err)

						println("party A osmo bal: ", osmoBalPartyA)
						println("party A atom bal: ", atomBalPartyA)
						println("party B osmo bal: ", osmoBalPartyB)
						println("party B atom bal: ", atomBalPartyB)
						break
					} else {
						i = i - 1
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("party B claims and router receives the funds", func(t *testing.T) {

				testCtx.holderClaim(holderAddress, osmoNeutronAccount, keyring.BackendTest)

				err = testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				for {
					routerAtomBalB, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronAtomIbcDenom)
					require.NoError(t, err)

					routerOsmoBalB, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronOsmoIbcDenom)
					require.NoError(t, err)

					println("routerAtomBalB: ", routerAtomBalB)
					println("routerOsmoBalB: ", routerOsmoBalB)

					if routerAtomBalB != 0 || routerOsmoBalB != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					osmoBalPartyA, err := cosmosAtom.GetBalance(
						ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom,
					)
					require.NoError(t, err)

					osmoBalPartyB, err := cosmosOsmosis.GetBalance(
						ctx, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
					)
					require.NoError(t, err)

					atomBalPartyA, err := cosmosAtom.GetBalance(
						ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
					)
					require.NoError(t, err)

					atomBalPartyB, err := cosmosOsmosis.GetBalance(
						ctx, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom,
					)
					require.NoError(t, err)

					println("party A osmo bal: ", osmoBalPartyA)
					println("party A atom bal: ", atomBalPartyA)
					println("party B osmo bal: ", osmoBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					if atomBalPartyA != 0 && osmoBalPartyB != 0 {
						break
					}

					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			})
		})

		t.Run("two party POL ragequit path", func(t *testing.T) {

			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "100", // sec
					IbcTransferTimeout: "100", // sec
				}

				currentHeight, err := cosmosNeutron.Height(ctx)
				require.NoError(t, err, "failed to get neutron height")
				depositBlock := Block(currentHeight + 200)
				lockupBlock := Block(currentHeight + 300)

				lockupConfig := Expiration{
					AtHeight: &lockupBlock,
				}
				depositDeadline := Expiration{
					AtHeight: &depositBlock,
				}
				presetIbcFee := PresetIbcFee{
					AckFee:     "10000",
					TimeoutFee: "10000",
				}

				atomCoin := Coin{
					Denom:  cosmosAtom.Config().Denom,
					Amount: strconv.FormatUint(atomContributionAmount, 10),
				}

				osmoCoin := Coin{
					Denom:  cosmosOsmosis.Config().Denom,
					Amount: strconv.FormatUint(osmoContributionAmount, 10),
				}
				hubReceiverAddr := rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				osmoReceiverAddr := rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)
				partyAConfig := CovenantPartyConfig{
					ControllerAddr:            hubReceiverAddr,
					HostAddr:                  hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              atomCoin,
					IbcDenom:                  neutronAtomIbcDenom,
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyReceiverAddr:         hubReceiverAddr,
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				}
				partyBConfig := CovenantPartyConfig{
					ControllerAddr:            osmoReceiverAddr,
					HostAddr:                  osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              osmoCoin,
					IbcDenom:                  neutronOsmoIbcDenom,
					PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
					PartyReceiverAddr:         osmoReceiverAddr,
					PartyChainConnectionId:    neutronOsmosisIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				}
				codeIds := ContractCodeIds{
					IbcForwarderCode:     ibcForwarderCodeId,
					InterchainRouterCode: routerCodeId,
					ClockCode:            clockCodeId,
					HolderCode:           holderCodeId,
					LiquidPoolerCode:     lperCodeId,
				}

				ragequitTerms := RagequitTerms{
					Penalty:      "0.1",
					RagequitType: "share",
				}

				ragequitConfig := RagequitConfig{
					Enabled: &ragequitTerms,
				}

				poolAddress := stableswapAddress
				pairType := PairType{
					Stable: struct{}{},
				}

				covenantMsg := CovenantInstantiateMsg{
					Label:                    "two-party-pol-covenant-ragequit",
					Timeouts:                 timeouts,
					PresetIbcFee:             presetIbcFee,
					ContractCodeIds:          codeIds,
					LockupConfig:             lockupConfig,
					PartyAConfig:             partyAConfig,
					PartyBConfig:             partyBConfig,
					PoolAddress:              poolAddress,
					RagequitConfig:           &ragequitConfig,
					DepositDeadline:          depositDeadline,
					PartyAShare:              "50",
					PartyBShare:              "50",
					ExpectedPoolRatio:        "0.1",
					AcceptablePoolRatioDelta: "0.09",
					PairType:                 pairType,
					Splits: []DenomSplit{
						{
							Denom: neutronAtomIbcDenom,
							Type: SplitType{
								Custom: SplitConfig{
									Receivers: []Receiver{
										{
											Address: hubReceiverAddr,
											Share:   "1.0",
										},
									},
								},
							},
						},
						{
							Denom: neutronOsmoIbcDenom,
							Type: SplitType{
								Custom: SplitConfig{
									Receivers: []Receiver{
										{
											Address: osmoReceiverAddr,
											Share:   "1.0",
										},
									},
								},
							},
						},
					},
					FallbackSplit: nil,
				}
				str, err := json.Marshal(covenantMsg)
				require.NoError(t, err, "Failed to marshall CovenantInstantiateMsg")
				instantiateMsg := string(str)

				covenantAddress = testCtx.manualInstantiate(covenantRqCodeIdStr, instantiateMsg, neutronUser, keyring.BackendTest)
				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.queryClockAddress(covenantAddress)
				println("clock addr: ", clockAddress)

				holderAddress = testCtx.queryHolderAddress(covenantAddress)
				println("holder addr: ", holderAddress)

				liquidPoolerAddress = testCtx.queryLiquidPoolerAddress(covenantAddress)
				println("liquid pooler addr: ", liquidPoolerAddress)

				partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
				println("partyARouterAddress: ", partyARouterAddress)

				partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
				println("partyBRouterAddress: ", partyBRouterAddress)

				partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
				println("partyAIbcForwarderAddress: ", partyAIbcForwarderAddress)

				partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
				println("partyBIbcForwarderAddress: ", partyBIbcForwarderAddress)
			})

			t.Run("fund contracts with neutron", func(t *testing.T) {
				err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyAIbcForwarderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to partyAIbcForwarder contract")

				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyBIbcForwarderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to partyBIbcForwarder contract")

				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: clockAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to clock contract")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyARouterAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to party a router")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyBRouterAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to party b router")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: holderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to holder")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: liquidPoolerAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to holder")

				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				bal, err := neutron.GetBalance(ctx, partyAIbcForwarderAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyBIbcForwarderAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, clockAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyARouterAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyBRouterAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				require.NoError(t, testutil.WaitForBlocks(ctx, 15, atom, neutron, osmosis), "failed to wait for blocks")
				for {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
					forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

					if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
						require.NoError(t, testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis), "failed to wait for blocks")

						partyADepositAddress = testCtx.queryDepositAddress(partyAIbcForwarderAddress)
						partyBDepositAddress = testCtx.queryDepositAddress(partyBIbcForwarderAddress)
						println("both parties icas created: ", partyADepositAddress, " , ", partyBDepositAddress)
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {

				err := cosmosOsmosis.SendFunds(ctx, rqCaseOsmoAccount.KeyName, ibc.WalletAmount{
					Address: partyBDepositAddress,
					Denom:   nativeOsmoDenom,
					Amount:  int64(osmoContributionAmount),
				})
				require.NoError(t, err, "failed to fund osmo forwarder")
				err = cosmosAtom.SendFunds(ctx, rqCaseHubAccount.KeyName, ibc.WalletAmount{
					Address: partyADepositAddress,
					Denom:   nativeAtomDenom,
					Amount:  int64(atomContributionAmount),
				})
				require.NoError(t, err, "failed to fund gaia forwarder")

				err = testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				bal, err := cosmosAtom.GetBalance(ctx, partyADepositAddress, nativeAtomDenom)
				require.NoError(t, err, "failed to query bal")
				require.Equal(t, int64(atomContributionAmount), bal)
				bal, err = cosmosOsmosis.GetBalance(ctx, partyBDepositAddress, nativeOsmoDenom)
				require.NoError(t, err, "failed to query bal")
				require.Equal(t, int64(osmoContributionAmount), bal)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					holderOsmoBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronOsmoIbcDenom)
					require.NoError(t, err, "failed to query holder osmo bal")
					holderAtomBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronAtomIbcDenom)
					require.NoError(t, err, "failed to query holder atom bal")
					// liquidPoolerOsmoBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronOsmoIbcDenom)
					// require.NoError(t, err, "failed to query liquidPooler osmo bal")
					// liquidPoolerAtomBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronAtomIbcDenom)
					// require.NoError(t, err, "failed to query liquidPooler atom bal")
					println("holder atom bal: ", holderAtomBal)
					println("holder osmo bal: ", holderOsmoBal)

					holderState := testCtx.queryContractState(holderAddress)
					println("holder state: ", holderState)

					if holderAtomBal == int64(atomContributionAmount) && holderOsmoBal == int64(osmoContributionAmount) || holderState == "active" {
						println("\nholder/liquidpooler received atom & osmo\n")
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder sends the funds to LPer", func(t *testing.T) {
				for {
					liquidPoolerOsmoBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronOsmoIbcDenom)
					require.NoError(t, err, "failed to query liquidPooler osmo bal")
					liquidPoolerAtomBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronAtomIbcDenom)
					require.NoError(t, err, "failed to query liquidPooler atom bal")
					holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderAddress)

					println("liquid pooler atom bal: ", liquidPoolerAtomBal)
					println("liquid pooler osmo bal: ", liquidPoolerOsmoBal)
					println("holder lp token balance: ", holderLpTokenBal)

					if liquidPoolerOsmoBal == int64(osmoContributionAmount) && liquidPoolerAtomBal == int64(atomContributionAmount) {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder receives LP tokens", func(t *testing.T) {
				for {
					holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderAddress)
					println("holder lp token balance: ", holderLpTokenBal)
					holderLpBal, err := strconv.ParseUint(holderLpTokenBal, 10, 64)
					if err != nil {
						panic(err)
					}

					if holderLpBal == 0 {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("party A ragequits", func(t *testing.T) {
				for {
					routerAtomBalA, err := cosmosNeutron.GetBalance(ctx, partyARouterAddress, neutronAtomIbcDenom)
					require.NoError(t, err)

					routerOsmoBalB, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronOsmoIbcDenom)
					require.NoError(t, err)

					println("routerAtomBalA: ", routerAtomBalA)
					println("routerOsmoBalB: ", routerOsmoBalB)

					if routerAtomBalA != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
						testCtx.holderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
					}
				}
			})

			t.Run("tick x10", func(t *testing.T) {
				i := 10
				for {

					if i == 0 {
						osmoBalPartyA, err := cosmosAtom.GetBalance(
							ctx, rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom,
						)
						require.NoError(t, err)

						osmoBalPartyB, err := cosmosOsmosis.GetBalance(
							ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
						)
						require.NoError(t, err)

						atomBalPartyA, err := cosmosAtom.GetBalance(
							ctx, rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
						)
						require.NoError(t, err)

						atomBalPartyB, err := cosmosOsmosis.GetBalance(
							ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom,
						)
						require.NoError(t, err)

						println("party A osmo bal: ", osmoBalPartyA)
						println("party A atom bal: ", atomBalPartyA)
						println("party B osmo bal: ", osmoBalPartyB)
						println("party B atom bal: ", atomBalPartyB)
						break
					} else {
						i = i - 1
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("party B claims and router receives the funds", func(t *testing.T) {

				testCtx.holderClaim(holderAddress, osmoNeutronAccount, keyring.BackendTest)

				err = testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				for {
					routerAtomBalB, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronAtomIbcDenom)
					require.NoError(t, err)

					routerOsmoBalB, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronOsmoIbcDenom)
					require.NoError(t, err)

					println("routerAtomBalB: ", routerAtomBalB)
					println("routerOsmoBalB: ", routerOsmoBalB)

					if routerOsmoBalB != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					osmoBalPartyB, err := cosmosOsmosis.GetBalance(
						ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
					)
					require.NoError(t, err)

					atomBalPartyA, err := cosmosAtom.GetBalance(
						ctx, rqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
					)
					require.NoError(t, err)

					atomBalPartyB, err := cosmosOsmosis.GetBalance(
						ctx, rqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom,
					)
					require.NoError(t, err)

					println("party A atom bal: ", atomBalPartyA)
					println("party B osmo bal: ", osmoBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					if atomBalPartyA != 0 && osmoBalPartyB != 0 && atomBalPartyB != 0 {
						println("nice")
						break
					}

					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			})
		})

		t.Run("two party POL side-based ragequit path", func(t *testing.T) {

			t.Run("instantiate covenant", func(t *testing.T) {
				timeouts := Timeouts{
					IcaTimeout:         "100", // sec
					IbcTransferTimeout: "100", // sec
				}

				currentHeight, err := cosmosNeutron.Height(ctx)
				require.NoError(t, err, "failed to get neutron height")
				depositBlock := Block(currentHeight + 200)
				lockupBlock := Block(currentHeight + 300)

				lockupConfig := Expiration{
					AtHeight: &lockupBlock,
				}
				depositDeadline := Expiration{
					AtHeight: &depositBlock,
				}
				presetIbcFee := PresetIbcFee{
					AckFee:     "10000",
					TimeoutFee: "10000",
				}

				atomCoin := Coin{
					Denom:  cosmosAtom.Config().Denom,
					Amount: strconv.FormatUint(atomContributionAmount, 10),
				}

				osmoCoin := Coin{
					Denom:  cosmosOsmosis.Config().Denom,
					Amount: strconv.FormatUint(osmoContributionAmount, 10),
				}
				hubReceiverAddr := sideBasedRqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
				osmoReceiverAddr := sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)
				partyAConfig := CovenantPartyConfig{
					ControllerAddr:            hubReceiverAddr,
					HostAddr:                  hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              atomCoin,
					IbcDenom:                  neutronAtomIbcDenom,
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyReceiverAddr:         hubReceiverAddr,
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				}
				partyBConfig := CovenantPartyConfig{
					ControllerAddr:            osmoReceiverAddr,
					HostAddr:                  osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					Contribution:              osmoCoin,
					IbcDenom:                  neutronOsmoIbcDenom,
					PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
					PartyReceiverAddr:         osmoReceiverAddr,
					PartyChainConnectionId:    neutronOsmosisIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				}
				codeIds := ContractCodeIds{
					IbcForwarderCode:     ibcForwarderCodeId,
					InterchainRouterCode: routerCodeId,
					ClockCode:            clockCodeId,
					HolderCode:           holderCodeId,
					LiquidPoolerCode:     lperCodeId,
				}

				ragequitTerms := RagequitTerms{
					Penalty:      "0.1",
					RagequitType: "side",
				}

				ragequitConfig := RagequitConfig{
					Enabled: &ragequitTerms,
				}

				poolAddress := stableswapAddress
				pairType := PairType{
					Stable: struct{}{},
				}

				covenantMsg := CovenantInstantiateMsg{
					Label:                    "two-party-pol-covenant-side-ragequit",
					Timeouts:                 timeouts,
					PresetIbcFee:             presetIbcFee,
					ContractCodeIds:          codeIds,
					LockupConfig:             lockupConfig,
					PartyAConfig:             partyAConfig,
					PartyBConfig:             partyBConfig,
					PoolAddress:              poolAddress,
					RagequitConfig:           &ragequitConfig,
					DepositDeadline:          depositDeadline,
					PartyAShare:              "50",
					PartyBShare:              "50",
					ExpectedPoolRatio:        "0.1",
					AcceptablePoolRatioDelta: "0.09",
					PairType:                 pairType,
					Splits: []DenomSplit{
						{
							Denom: neutronAtomIbcDenom,
							Type: SplitType{
								Custom: SplitConfig{
									Receivers: []Receiver{
										{
											Address: hubReceiverAddr,
											Share:   "1.0",
										},
									},
								},
							},
						},
						{
							Denom: neutronOsmoIbcDenom,
							Type: SplitType{
								Custom: SplitConfig{
									Receivers: []Receiver{
										{
											Address: osmoReceiverAddr,
											Share:   "1.0",
										},
									},
								},
							},
						},
					},
					FallbackSplit: nil,
				}
				str, err := json.Marshal(covenantMsg)
				require.NoError(t, err, "Failed to marshall CovenantInstantiateMsg")
				instantiateMsg := string(str)

				covenantAddress = testCtx.manualInstantiate(covenantSideBasedRqCodeIdStr, instantiateMsg, neutronUser, keyring.BackendTest)
				println("covenant address: ", covenantAddress)
			})

			t.Run("query covenant contracts", func(t *testing.T) {
				clockAddress = testCtx.queryClockAddress(covenantAddress)
				println("clock addr: ", clockAddress)

				holderAddress = testCtx.queryHolderAddress(covenantAddress)
				println("holder addr: ", holderAddress)

				liquidPoolerAddress = testCtx.queryLiquidPoolerAddress(covenantAddress)
				println("liquid pooler addr: ", liquidPoolerAddress)

				partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
				println("partyARouterAddress: ", partyARouterAddress)

				partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
				println("partyBRouterAddress: ", partyBRouterAddress)

				partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
				println("partyAIbcForwarderAddress: ", partyAIbcForwarderAddress)

				partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
				println("partyBIbcForwarderAddress: ", partyBIbcForwarderAddress)
			})

			t.Run("fund contracts with neutron", func(t *testing.T) {
				err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyAIbcForwarderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to partyAIbcForwarder contract")

				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyBIbcForwarderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to partyBIbcForwarder contract")

				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: clockAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to clock contract")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyARouterAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to party a router")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyBRouterAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to party b router")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: holderAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to holder")
				err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: liquidPoolerAddress,
					Amount:  5000000001,
					Denom:   nativeNtrnDenom,
				})
				require.NoError(t, err, "failed to send funds from neutron user to holder")

				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				bal, err := neutron.GetBalance(ctx, partyAIbcForwarderAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyBIbcForwarderAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, clockAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyARouterAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
				bal, err = neutron.GetBalance(ctx, partyBRouterAddress, nativeNtrnDenom)
				require.NoError(t, err)
				require.Equal(t, int64(5000000001), bal)
			})

			t.Run("tick until forwarders create ICA", func(t *testing.T) {
				require.NoError(t, testutil.WaitForBlocks(ctx, 15, atom, neutron, osmosis), "failed to wait for blocks")
				for {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

					forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
					forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

					if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
						require.NoError(t, testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis), "failed to wait for blocks")

						partyADepositAddress = testCtx.queryDepositAddress(partyAIbcForwarderAddress)
						partyBDepositAddress = testCtx.queryDepositAddress(partyBIbcForwarderAddress)

						println("both parties icas created: ", partyADepositAddress, " , ", partyBDepositAddress)
						break
					}
				}
			})

			t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {

				err := cosmosOsmosis.SendFunds(ctx, sideBasedRqCaseOsmoAccount.KeyName, ibc.WalletAmount{
					Address: partyBDepositAddress,
					Denom:   nativeOsmoDenom,
					Amount:  int64(osmoContributionAmount),
				})
				require.NoError(t, err, "failed to fund osmo forwarder")
				err = cosmosAtom.SendFunds(ctx, sideBasedRqCaseHubAccount.KeyName, ibc.WalletAmount{
					Address: partyADepositAddress,
					Denom:   nativeAtomDenom,
					Amount:  int64(atomContributionAmount),
				})
				require.NoError(t, err, "failed to fund gaia forwarder")

				err = testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis)
				require.NoError(t, err, "failed to wait for blocks")

				bal, err := cosmosAtom.GetBalance(ctx, partyADepositAddress, nativeAtomDenom)
				require.NoError(t, err, "failed to query bal")
				require.Equal(t, int64(atomContributionAmount), bal)
				bal, err = cosmosOsmosis.GetBalance(ctx, partyBDepositAddress, nativeOsmoDenom)
				require.NoError(t, err, "failed to query bal")
				require.Equal(t, int64(osmoContributionAmount), bal)
			})

			t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
				for {
					holderOsmoBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronOsmoIbcDenom)
					require.NoError(t, err, "failed to query holder osmo bal")
					holderAtomBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronAtomIbcDenom)
					require.NoError(t, err, "failed to query holder atom bal")

					println("holder atom bal: ", holderAtomBal)
					println("holder osmo bal: ", holderOsmoBal)

					holderState := testCtx.queryContractState(holderAddress)
					println("holder state: ", holderState)

					if holderAtomBal == int64(atomContributionAmount) && holderOsmoBal == int64(osmoContributionAmount) || holderState == "active" {
						println("\nholder/liquidpooler received atom & osmo\n")
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder sends the funds to LPer", func(t *testing.T) {
				for {
					liquidPoolerOsmoBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronOsmoIbcDenom)
					require.NoError(t, err, "failed to query liquidPooler osmo bal")
					liquidPoolerAtomBal, err := cosmosNeutron.GetBalance(ctx, liquidPoolerAddress, neutronAtomIbcDenom)
					require.NoError(t, err, "failed to query liquidPooler atom bal")
					holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderAddress)

					println("liquid pooler atom bal: ", liquidPoolerAtomBal)
					println("liquid pooler osmo bal: ", liquidPoolerOsmoBal)
					println("holder lp token balance: ", holderLpTokenBal)

					if liquidPoolerOsmoBal == int64(osmoContributionAmount) && liquidPoolerAtomBal == int64(atomContributionAmount) {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick until holder receives LP tokens", func(t *testing.T) {
				for {
					holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderAddress)
					println("holder lp token balance: ", holderLpTokenBal)
					holderLpBal, err := strconv.ParseUint(holderLpTokenBal, 10, 64)
					if err != nil {
						panic(err)
					}

					if holderLpBal == 0 {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					} else {
						break
					}
				}
			})

			t.Run("party A ragequits", func(t *testing.T) {

				for {
					routerAtomBalA, err := cosmosNeutron.GetBalance(ctx, partyARouterAddress, neutronAtomIbcDenom)
					require.NoError(t, err)

					routerOsmoBalB, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronOsmoIbcDenom)
					require.NoError(t, err)

					println("routerAtomBalA: ", routerAtomBalA)
					println("routerOsmoBalB: ", routerOsmoBalB)

					if routerAtomBalA != 0 {
						break
					} else {
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
						testCtx.holderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
					}
				}
			})

			t.Run("tick x10", func(t *testing.T) {
				i := 10
				for {

					if i == 0 {
						osmoBalPartyA, err := cosmosAtom.GetBalance(
							ctx, sideBasedRqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom,
						)
						require.NoError(t, err)

						osmoBalPartyB, err := cosmosOsmosis.GetBalance(
							ctx, sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
						)
						require.NoError(t, err)

						atomBalPartyA, err := cosmosAtom.GetBalance(
							ctx, sideBasedRqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
						)
						require.NoError(t, err)

						atomBalPartyB, err := cosmosOsmosis.GetBalance(
							ctx, sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom,
						)
						require.NoError(t, err)

						println("party A osmo bal: ", osmoBalPartyA)
						println("party A atom bal: ", atomBalPartyA)
						println("party B osmo bal: ", osmoBalPartyB)
						println("party B atom bal: ", atomBalPartyB)
						break
					} else {
						i = i - 1
						testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					}
				}
			})

			t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
				for {
					osmoBalPartyB, err := cosmosOsmosis.GetBalance(
						ctx, sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom,
					)
					require.NoError(t, err)

					atomBalPartyA, err := cosmosAtom.GetBalance(
						ctx, sideBasedRqCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom,
					)
					require.NoError(t, err)

					atomBalPartyB, err := cosmosOsmosis.GetBalance(
						ctx, sideBasedRqCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom,
					)
					require.NoError(t, err)

					println("party A atom bal: ", atomBalPartyA)
					println("party B osmo bal: ", osmoBalPartyB)
					println("party B atom bal: ", atomBalPartyB)

					if atomBalPartyA != 0 && osmoBalPartyB != 0 && atomBalPartyB != 0 {
						println("nice")
						break
					}

					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			})
		})
	})
}
