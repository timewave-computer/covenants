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
	transfertypes "github.com/cosmos/ibc-go/v4/modules/apps/transfer/types"
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

// sets up and tests a tokenswap between hub and stargaze facilitated by neutron
func TestTokenSwap(t *testing.T) {
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
			GasAdjustment: 1.3,
			GasPrices:     "0.0atom",
			ModifyGenesis: setupGaiaGenesis([]string{
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
				Denom:          "untrn",
				GasPrices:      "0.0untrn,0.0uatom",
				GasAdjustment:  1.3,
				TrustingPeriod: "1197504s",
				NoHostMount:    false,
				ModifyGenesis: setupNeutronGenesis(
					"0.05",
					[]string{"untrn"},
					[]string{"uatom"},
					[]string{
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
		{
			Name:    "osmosis",
			Version: "v14.0.0",
			ChainConfig: ibc.ChainConfig{
				Type:         "cosmos",
				Bin:          "osmosisd",
				Bech32Prefix: "osmo",
				Denom:        "uosmo",
				ModifyGenesis: setupOsmoGenesis([]string{
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
					"/ibc.applications.interchain_accounts.v1.InterchainAccount",
				}),
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
		zaptest.NewLogger(t),
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
	err = ic.Build(ctx, eRep, ibctest.InterchainBuildOptions{
		TestName:          t.Name(),
		Client:            client,
		NetworkID:         network,
		BlockDatabaseFile: ibctest.DefaultBlockDatabaseFilepath(),
		SkipPathCreation:  true,
	})
	require.NoError(t, err, "failed to build interchain")

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis)
	require.NoError(t, err, "failed to wait for blocks")

	testCtx := &TestContext{
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
	}

	// generate paths
	generatePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosNeutron.Config().ChainID, gaiaNeutronIBCPath)
	generatePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosOsmosis.Config().ChainID, gaiaOsmosisIBCPath)
	generatePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosOsmosis.Config().ChainID, neutronOsmosisIBCPath)
	generatePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosAtom.Config().ChainID, gaiaNeutronICSPath)

	// create clients
	generateClient(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)
	neutronClients := testCtx.getChainClients(cosmosNeutron.Config().Name)
	atomClients := testCtx.getChainClients(cosmosAtom.Config().Name)

	err = r.UpdatePath(ctx, eRep, gaiaNeutronICSPath, ibc.PathUpdateOptions{
		SrcClientID: &neutronClients[0].ClientID,
		DstClientID: &atomClients[0].ClientID,
	})
	require.NoError(t, err)

	atomNeutronICSConnectionId, neutronAtomICSConnectionId := generateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)

	generateICSChannel(t, ctx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)

	// create connections and link everything up
	generateClient(t, ctx, testCtx, r, eRep, neutronOsmosisIBCPath, cosmosNeutron, cosmosOsmosis)
	neutronOsmosisIBCConnId, osmosisNeutronIBCConnId := generateConnections(t, ctx, testCtx, r, eRep, neutronOsmosisIBCPath, cosmosNeutron, cosmosOsmosis)
	linkPath(t, ctx, r, eRep, cosmosNeutron, cosmosOsmosis, neutronOsmosisIBCPath)

	generateClient(t, ctx, testCtx, r, eRep, gaiaOsmosisIBCPath, cosmosAtom, cosmosOsmosis)
	gaiaOsmosisIBCConnId, osmosisGaiaIBCConnId := generateConnections(t, ctx, testCtx, r, eRep, gaiaOsmosisIBCPath, cosmosAtom, cosmosOsmosis)
	linkPath(t, ctx, r, eRep, cosmosAtom, cosmosOsmosis, gaiaOsmosisIBCPath)

	generateClient(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
	atomNeutronIBCConnId, neutronAtomIBCConnId := generateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
	linkPath(t, ctx, r, eRep, cosmosAtom, cosmosNeutron, gaiaNeutronIBCPath)

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

	createValidator(t, ctx, r, eRep, atom, neutron)
	err = testutil.WaitForBlocks(ctx, 2, atom, neutron, osmosis)
	require.NoError(t, err, "failed to wait for blocks")

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron, osmosis)
	gaiaUser, neutronUser, osmoUser := users[0], users[1], users[2]
	_, _, _ = gaiaUser, neutronUser, osmoUser

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis)
	require.NoError(t, err, "failed to wait for blocks")

	neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
	gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)
	osmoChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosOsmosis.Config().ChainID)

	// Find all pairwise channels
	getPairwiseTransferChannelIds(testCtx, osmoChannelInfo, neutronChannelInfo, osmosisNeutronIBCConnId, neutronOsmosisIBCConnId, osmosis.Config().Name, neutron.Config().Name)
	getPairwiseTransferChannelIds(testCtx, osmoChannelInfo, gaiaChannelInfo, osmosisGaiaIBCConnId, gaiaOsmosisIBCConnId, osmosis.Config().Name, cosmosAtom.Config().Name)
	getPairwiseTransferChannelIds(testCtx, gaiaChannelInfo, neutronChannelInfo, atomNeutronIBCConnId, neutronAtomIBCConnId, cosmosAtom.Config().Name, neutron.Config().Name)
	getPairwiseCCVChannelIds(testCtx, gaiaChannelInfo, neutronChannelInfo, atomNeutronICSConnectionId, neutronAtomICSConnectionId, cosmosAtom.Config().Name, cosmosNeutron.Config().Name)

	println("neutron channels:")
	for key, value := range testCtx.NeutronTransferChannelIds {
		fmt.Printf("Key: %s, Value: %s\n", key, value)
	}
	print("\n osmo channels: ")
	for key, value := range testCtx.OsmoTransferChannelIds {
		fmt.Printf("Key: %s, Value: %s\n", key, value)
	}
	println("gaia channels:")
	for key, value := range testCtx.GaiaTransferChannelIds {
		fmt.Printf("Key: %s, Value: %s\n", key, value)
	}
	println("gaia ics channels:")
	for key, value := range testCtx.GaiaIcsChannelIds {
		fmt.Printf("Key: %s, Value: %s\n", key, value)
	}
	println("neutron ics channels:")
	for key, value := range testCtx.NeutronIcsChannelIds {
		fmt.Printf("Key: %s, Value: %s\n", key, value)
	}

	// We can determine the ibc denoms of:
	// 1. ATOM on Neutron
	neutronAtomIbcDenom := testCtx.getIbcDenom(testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name], cosmosAtom.Config().Denom)
	// 2. Osmo on neutron
	neutronOsmoIbcDenom := testCtx.getIbcDenom(
		testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
		cosmosOsmosis.Config().Denom,
	)

	// 3. hub atom => neutron => osmosis
	// transfer/channel-0/transfer/channel-3

	neutronAtomPrefix := fmt.Sprintf("%s/%s", "transfer", testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name])
	neutronOsmoPrefix := fmt.Sprintf("%s/%s", "transfer", testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name])
	gaiaNeutronPrefix := fmt.Sprintf("%s/%s", "transfer", testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name])
	osmoNeutronPrefix := fmt.Sprintf("%s/%s", "transfer", testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name])

	println("neutronAtomPrefix: ", neutronAtomPrefix)
	println("neutronOsmoPrefix: ", neutronOsmoPrefix)
	println("gaiaNeutronPrefix: ", gaiaNeutronPrefix)
	println("osmoNeutronPrefix: ", osmoNeutronPrefix)

	osmoNeutronAtomPrefixedDenom := transfertypes.GetPrefixedDenom(
		osmoNeutronPrefix,
		neutronAtomPrefix,
		cosmosAtom.Config().Denom,
	)
	println("osmoNeutronAtomPrefixedDenom: ", osmoNeutronAtomPrefixedDenom)

	gaiaNeutronOsmoPrefixedDenom := transfertypes.GetPrefixedDenom(
		gaiaNeutronPrefix,
		neutronOsmoPrefix,
		cosmosOsmosis.Config().Denom,
	)
	println("gaiaNeutronOsmoPrefixedDenom: ", gaiaNeutronOsmoPrefixedDenom)

	gaiaNeutronOsmoIbcDenom := transfertypes.ParseDenomTrace(gaiaNeutronOsmoPrefixedDenom).IBCDenom()
	osmoNeutronAtomIbcDenom := transfertypes.ParseDenomTrace(osmoNeutronAtomPrefixedDenom).IBCDenom()

	println("neutronAtomIbcDenom: ", neutronAtomIbcDenom)
	println("neutronOsmoIbcDenom: ", neutronOsmoIbcDenom)
	println("osmoNeutronAtomIbcDenom: ", osmoNeutronAtomIbcDenom)
	println("gaiaNeutronOsmoIbcDenom: ", gaiaNeutronOsmoIbcDenom)

	// 2CB1

	var covenantAddress string
	var clockAddress string
	var splitterAddress string
	var partyARouterAddress string
	var partyBRouterAddress string
	var partyAIbcForwarderAddress string
	var partyBIbcForwarderAddress string
	var holderAddress string

	var partyADepositAddress, partyBDepositAddress string

	t.Run("tokenswap setup", func(t *testing.T) {
		//----------------------------------------------//
		// Testing parameters
		//----------------------------------------------//

		// PARTY_A
		const osmoContributionAmount uint64 = 100_000_000_000 // in uosmo

		// PARTY_B
		const atomContributionAmount uint64 = 5_000_000_000 // in uatom

		//----------------------------------------------//
		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_swap.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const routerContractPath = "wasms/covenant_interchain_router.wasm"
		const splitterContractPath = "wasms/covenant_interchain_splitter.wasm"
		const ibcForwarderContractPath = "wasms/covenant_ibc_forwarder.wasm"
		const swapHolderContractPath = "wasms/covenant_swap_holder.wasm"

		// After storing on Neutron, we will receive a code id
		// We parse all the subcontracts into uint64
		// The will be required when we instantiate the covenant.
		var clockCodeId uint64
		var routerCodeId uint64
		var splitterCodeId uint64
		var ibcForwarderCodeId uint64
		var swapHolderCodeId uint64
		var covenantCodeIdStr string
		var covenantCodeId uint64
		_ = covenantCodeId

		t.Run("deploy covenant contracts", func(t *testing.T) {
			// store covenant and get code id
			covenantCodeIdStr, err = cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, covenantContractPath)
			require.NoError(t, err, "failed to store stride covenant contract")
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

			// store clock and get code id
			splitterCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, splitterContractPath)
			require.NoError(t, err, "failed to store splitter contract")
			splitterCodeId, err = strconv.ParseUint(splitterCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store clock and get code id
			ibcForwarderCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, ibcForwarderContractPath)
			require.NoError(t, err, "failed to store ibc forwarder contract")
			ibcForwarderCodeId, err = strconv.ParseUint(ibcForwarderCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store clock and get code id
			swapHolderCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, swapHolderContractPath)
			require.NoError(t, err, "failed to store swap holder contract")
			swapHolderCodeId, err = strconv.ParseUint(swapHolderCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

		})
		println(covenantCodeIdStr, clockCodeId, routerCodeId, splitterCodeId, ibcForwarderCodeId, swapHolderCodeId)
		require.NoError(t, testutil.WaitForBlocks(ctx, 10, cosmosNeutron, cosmosAtom, cosmosOsmosis))

		t.Run("instantiate covenant", func(t *testing.T) {

			// Clock instantiation message
			clockMsg := PresetClockFields{
				ClockCode: clockCodeId,
				Label:     "covenant-clock",
				Whitelist: []string{},
			}

			timeouts := Timeouts{
				IcaTimeout:         "100", // sec
				IbcTransferTimeout: "100", // sec
			}

			swapCovenantTerms := SwapCovenantTerms{
				PartyAAmount: strconv.FormatUint(atomContributionAmount, 10),
				PartyBAmount: strconv.FormatUint(osmoContributionAmount, 10),
			}

			covenantPartiesConfig := CovenantPartiesConfig{
				PartyA: CovenantParty{
					Addr:     gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					IbcDenom: neutronAtomIbcDenom,
					ReceiverConfig: ReceiverConfig{
						Native: gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					},
				},
				PartyB: CovenantParty{
					Addr:     osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix),
					IbcDenom: neutronOsmoIbcDenom,
					ReceiverConfig: ReceiverConfig{
						Native: osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix),
					},
				},
			}
			timestamp := Timestamp("1981539923")

			lockupConfig := LockupConfig{
				Time: &timestamp,
			}
			covenantTerms := CovenantTerms{
				TokenSwap: swapCovenantTerms,
			}
			presetIbcFee := PresetIbcFee{
				AckFee:     "10000",
				TimeoutFee: "10000",
			}

			presetSwapHolder := PresetSwapHolderFields{
				LockupConfig:          lockupConfig,
				CovenantPartiesConfig: covenantPartiesConfig,
				CovenantTerms:         covenantTerms,
				CodeId:                swapHolderCodeId,
				Label:                 "swap-holder",
			}

			swapCovenantParties := SwapCovenantParties{
				PartyA: SwapPartyConfig{
					Addr:                      gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					NativeDenom:               "uatom",
					IbcDenom:                  neutronAtomIbcDenom,
					PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyReceiverAddr:         gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					PartyChainConnectionId:    neutronAtomIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				},
				PartyB: SwapPartyConfig{
					Addr:                      osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix),
					NativeDenom:               "uosmo",
					IbcDenom:                  neutronOsmoIbcDenom,
					PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
					HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
					PartyReceiverAddr:         osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix),
					PartyChainConnectionId:    neutronOsmosisIBCConnId,
					IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				},
			}

			presetSplitterFields := PresetSplitterFields{
				Splits: []DenomSplit{
					{
						Denom: neutronOsmoIbcDenom,
						Type: SplitType{
							Custom: SplitConfig{
								Receivers: []Receiver{
									Receiver{
										Address: gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
										Share:   "100",
									},
								},
							},
						},
					},
					{
						Denom: neutronAtomIbcDenom,
						Type: SplitType{
							Custom: SplitConfig{
								Receivers: []Receiver{
									Receiver{
										Address: osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix),
										Share:   "100",
									},
								},
							},
						},
					},
				},
				FallbackSplit: nil,
				Label:         "interchain-splitter",
			}

			covenantMsg := CovenantInstantiateMsg{
				Label:                  "swap-covenant",
				Timeouts:               timeouts,
				PresetIbcFee:           presetIbcFee,
				IbcForwarderCode:       ibcForwarderCodeId,
				InterchainRouterCode:   routerCodeId,
				InterchainSplitterCode: splitterCodeId,
				PresetClock:            clockMsg,
				PresetSwapHolder:       presetSwapHolder,
				SwapCovenantParties:    swapCovenantParties,
				PresetSplitterFields:   presetSplitterFields,
			}

			str, err := json.Marshal(covenantMsg)
			require.NoError(t, err, "Failed to marshall CovenantInstantiateMsg")
			println("covenant instantiation msg: ", string(str))
			instantiateMsg := string(str)

			cmd := []string{"neutrond", "tx", "wasm", "instantiate", covenantCodeIdStr,
				instantiateMsg,
				"--label", "swap-covenant",
				"--no-admin",
				"--from", neutronUser.KeyName,
				"--output", "json",
				"--home", neutron.HomeDir(),
				"--node", neutron.GetRPCAddress(),
				"--chain-id", neutron.Config().ChainID,
				"--gas", "90009000",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			resp, _, err := neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			println("instantiated, skipping 10 blocks...")

			println("instantiate response: ", string(resp), "\n")
			require.NoError(t, testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis))

			queryCmd := []string{"neutrond", "query", "wasm",
				"list-contract-by-code", covenantCodeIdStr,
				"--output", "json",
				"--home", neutron.HomeDir(),
				"--node", neutron.GetRPCAddress(),
				"--chain-id", neutron.Config().ChainID,
			}

			queryResp, _, err := neutron.Exec(ctx, queryCmd, nil)
			require.NoError(t, err, "failed to query")

			println("query response: ", string(queryResp))
			type QueryContractResponse struct {
				Contracts  []string `json:"contracts"`
				Pagination any      `json:"pagination"`
			}

			contactsRes := QueryContractResponse{}
			require.NoError(t, json.Unmarshal(queryResp, &contactsRes), "failed to unmarshal contract response")

			covenantAddress = contactsRes.Contracts[len(contactsRes.Contracts)-1]

			println("covenant address: ", covenantAddress)
		})

		t.Run("query covenant contracts", func(t *testing.T) {
			routerQueryPartyA := InterchainRouterQuery{
				Party: Party{
					Party: "party_a",
				},
			}
			routerQueryPartyB := InterchainRouterQuery{
				Party: Party{
					Party: "party_b",
				},
			}
			forwarderQueryPartyA := IbcForwarderQuery{
				Party: Party{
					Party: "party_a",
				},
			}
			forwarderQueryPartyB := IbcForwarderQuery{
				Party: Party{
					Party: "party_b",
				},
			}
			var response CovenantAddressQueryResponse

			err = cosmosNeutron.QueryContract(ctx, covenantAddress, ClockAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated clock address")
			clockAddress = response.Data
			println("clock addr: ", clockAddress)

			err = cosmosNeutron.QueryContract(ctx, covenantAddress, HolderAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated holder address")
			holderAddress = response.Data
			println("holder addr: ", holderAddress)

			err = cosmosNeutron.QueryContract(ctx, covenantAddress, SplitterAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated splitter address")
			splitterAddress = response.Data
			println("splitter addr: ", splitterAddress)

			err = cosmosNeutron.QueryContract(ctx, covenantAddress, routerQueryPartyA, &response)
			require.NoError(t, err, "failed to query instantiated party a router address")
			partyARouterAddress = response.Data
			println("partyARouterAddress: ", partyARouterAddress)

			err = cosmosNeutron.QueryContract(ctx, covenantAddress, routerQueryPartyB, &response)
			require.NoError(t, err, "failed to query instantiated party b router address")
			partyBRouterAddress = response.Data
			println("partyBRouterAddress: ", partyBRouterAddress)

			err = cosmosNeutron.QueryContract(ctx, covenantAddress, forwarderQueryPartyA, &response)
			require.NoError(t, err, "failed to query instantiated party a forwarder address")
			partyAIbcForwarderAddress = response.Data
			println("partyAIbcForwarderAddress: ", partyAIbcForwarderAddress)

			err = cosmosNeutron.QueryContract(ctx, covenantAddress, forwarderQueryPartyB, &response)
			require.NoError(t, err, "failed to query instantiated party b forwarder address")
			partyBIbcForwarderAddress = response.Data
			println("partyBIbcForwarderAddress: ", partyBIbcForwarderAddress)
		})

		t.Run("fund contracts with neutron", func(t *testing.T) {
			err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: partyAIbcForwarderAddress,
				Amount:  5000001,
				Denom:   neutron.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from neutron user to partyAIbcForwarder contract")

			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: partyBIbcForwarderAddress,
				Amount:  5000001,
				Denom:   neutron.Config().Denom,
			})
			require.NoError(t, err, "failed to send funds from neutron user to partyBIbcForwarder contract")

			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: clockAddress,
				Amount:  5000001,
				Denom:   neutron.Config().Denom,
			})
			require.NoError(t, err, "failed to send funds from neutron user to clock contract")
			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: partyARouterAddress,
				Amount:  15000001,
				Denom:   neutron.Config().Denom,
			})
			require.NoError(t, err, "failed to send funds from neutron user to party a router")
			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: partyBRouterAddress,
				Amount:  15000001,
				Denom:   neutron.Config().Denom,
			})
			require.NoError(t, err, "failed to send funds from neutron user to party b router")

			err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			bal, err := neutron.GetBalance(ctx, partyAIbcForwarderAddress, neutron.Config().Denom)
			require.NoError(t, err)
			require.Equal(t, int64(5000001), bal)
			bal, err = neutron.GetBalance(ctx, partyBIbcForwarderAddress, neutron.Config().Denom)
			require.NoError(t, err)
			require.Equal(t, int64(5000001), bal)
			bal, err = neutron.GetBalance(ctx, clockAddress, neutron.Config().Denom)
			require.NoError(t, err)
			require.Equal(t, int64(5000001), bal)
			bal, err = neutron.GetBalance(ctx, partyARouterAddress, neutron.Config().Denom)
			require.NoError(t, err)
			require.Equal(t, int64(15000001), bal)
			bal, err = neutron.GetBalance(ctx, partyBRouterAddress, neutron.Config().Denom)
			require.NoError(t, err)
			require.Equal(t, int64(15000001), bal)
		})

		tickClock := func() {
			cmd := []string{"neutrond", "tx", "wasm", "execute", clockAddress,
				`{"tick":{}}`,
				"--from", neutronUser.KeyName,
				"--gas-prices", "0.0untrn",
				"--gas-adjustment", `1.8`,
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

			println("tick cmd: ", strings.Join(cmd, " "))
			stdout, _, err := cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			println("clock tick response: ", string(stdout))
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis)
			require.NoError(t, err, "failed to wait for blocks")
		}

		t.Run("tick until forwarders create ICA", func(t *testing.T) {
			const maxTicks = 10
			tick := 1
			var response CovenantAddressQueryResponse
			for tick <= maxTicks {
				println("Ticking clock ", tick, " of ", maxTicks)
				tickClock()
				type DepositAddress struct{}
				type DepositAddressQuery struct {
					DepositAddress DepositAddress `json:"deposit_address"`
				}
				depositAddressQuery := DepositAddressQuery{
					DepositAddress: DepositAddress{},
				}

				type ContractState struct{}
				type ContractStateQuery struct {
					ContractState ContractState `json:"contract_state"`
				}
				contractStateQuery := ContractStateQuery{
					ContractState: ContractState{},
				}

				err := cosmosNeutron.QueryContract(ctx, partyAIbcForwarderAddress, depositAddressQuery, &response)
				require.NoError(t, err, "failed to query party a forwarder deposit address")
				partyADepositAddr := response.Data
				println("partyADepositAddress: ", partyADepositAddress)

				err = cosmosNeutron.QueryContract(ctx, partyBIbcForwarderAddress, depositAddressQuery, &response)
				require.NoError(t, err, "failed to query party b forwarder deposit address")
				partyBDepositAddr := response.Data
				println("partyBDepositAddress: ", partyBDepositAddress)

				err = cosmosNeutron.QueryContract(ctx, partyAIbcForwarderAddress, contractStateQuery, &response)
				require.NoError(t, err, "failed to query forwarder A state")
				forwarderAState := response.Data
				println("forwarderAState: ", forwarderAState)
				err = cosmosNeutron.QueryContract(ctx, partyBIbcForwarderAddress, contractStateQuery, &response)
				require.NoError(t, err, "failed to query forwarder B state")
				forwarderBState := response.Data
				println("forwarderBState: ", forwarderBState)

				if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
					partyADepositAddress = partyADepositAddr
					partyBDepositAddress = partyBDepositAddr
					break
				}
				tick += 1
			}
		})

		t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
			err := cosmosOsmosis.SendFunds(ctx, osmoUser.KeyName, ibc.WalletAmount{
				Address: partyBDepositAddress,
				Denom:   cosmosOsmosis.Config().Denom,
				Amount:  int64(osmoContributionAmount + 1000),
			})
			require.NoError(t, err, "failed to fund osmo forwarder")
			err = cosmosAtom.SendFunds(ctx, gaiaUser.KeyName, ibc.WalletAmount{
				Address: partyADepositAddress,
				Denom:   cosmosAtom.Config().Denom,
				Amount:  int64(atomContributionAmount + 1000),
			})
			require.NoError(t, err, "failed to fund gaia forwarder")

			err = testutil.WaitForBlocks(ctx, 5, atom, neutron, osmosis)
			require.NoError(t, err, "failed to wait for blocks")

			bal, err := cosmosAtom.GetBalance(ctx, partyADepositAddress, cosmosAtom.Config().Denom)
			require.NoError(t, err, "failed to query bal")
			require.Equal(t, int64(atomContributionAmount+1000), bal)
			bal, err = cosmosOsmosis.GetBalance(ctx, partyBDepositAddress, cosmosOsmosis.Config().Denom)
			require.NoError(t, err, "failed to query bal")
			require.Equal(t, int64(osmoContributionAmount+1000), bal)
		})

		t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {

			const maxTicks = 20
			tick := 1
			var response CovenantAddressQueryResponse
			for tick <= maxTicks {
				println("Ticking clock ", tick, " of ", maxTicks)
				tickClock()

				type ContractState struct{}
				type ContractStateQuery struct {
					ContractState ContractState `json:"contract_state"`
				}
				contractStateQuery := ContractStateQuery{
					ContractState: ContractState{},
				}

				err = cosmosNeutron.QueryContract(ctx, partyAIbcForwarderAddress, contractStateQuery, &response)
				require.NoError(t, err, "failed to query forwarder A state")
				forwarderAState := response.Data
				println("forwarderAState: ", forwarderAState)
				err = cosmosNeutron.QueryContract(ctx, partyBIbcForwarderAddress, contractStateQuery, &response)
				require.NoError(t, err, "failed to query forwarder B state")
				forwarderBState := response.Data
				println("forwarderBState: ", forwarderBState)
				err = cosmosNeutron.QueryContract(ctx, holderAddress, contractStateQuery, &response)
				require.NoError(t, err, "failed to query holder state")
				holderState := response.Data
				println("holderState: ", holderState)

				holderOsmoBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query holder osmo bal")
				println("holder osmo bal: ", holderOsmoBal)
				holderAtomBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query holder atom bal")
				println("holder atom bal: ", holderAtomBal)

				if holderAtomBal != 0 && holderOsmoBal != 0 {
					break
				}
				tick += 1
			}
		})

		t.Run("tick until holder sends the funds to splitter", func(t *testing.T) {
			const maxTicks = 20
			tick := 1
			for tick <= maxTicks {
				holderOsmoBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query holder osmo bal")
				println("holder osmo bal: ", holderOsmoBal)
				holderAtomBal, err := cosmosNeutron.GetBalance(ctx, holderAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query holder atom bal")
				println("holder atom bal: ", holderAtomBal)

				println("Ticking clock ", tick, " of ", maxTicks)
				tickClock()

				splitterOsmoBal, err := cosmosNeutron.GetBalance(ctx, splitterAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query splitterOsmoBal")
				println("splitterOsmoBal: ", splitterOsmoBal)
				splitterAtomBal, err := cosmosNeutron.GetBalance(ctx, splitterAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query splitterAtomBal")
				println("splitterAtomBal: ", splitterAtomBal)

				if splitterAtomBal != 0 && splitterOsmoBal != 0 {
					break
				}
			}
		})

		t.Run("tick until splitter sends the funds to routers", func(t *testing.T) {
			const maxTicks = 20
			tick := 1
			for tick <= maxTicks {
				splitterOsmoBal, err := cosmosNeutron.GetBalance(ctx, splitterAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query splitterOsmoBal")
				println("splitterOsmoBal: ", splitterOsmoBal)
				splitterAtomBal, err := cosmosNeutron.GetBalance(ctx, splitterAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query splitterAtomBal")
				println("splitterAtomBal: ", splitterAtomBal)

				println("Ticking clock ", tick, " of ", maxTicks)
				tickClock()

				partyARouterAtomBal, err := cosmosNeutron.GetBalance(ctx, partyARouterAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query partyARouterBal")
				println("partyARouter atom bal: ", partyARouterAtomBal)
				partyARouterOsmoBal, err := cosmosNeutron.GetBalance(ctx, partyARouterAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query partyARouterOsmoBal")
				println("partyARouter osmo bal: ", partyARouterOsmoBal)
				partyBRouterOsmoBal, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query partyBRouterOsmoBal")
				println("partyBRouterOsmoBal: ", partyBRouterOsmoBal)
				partyBRouterAtomBal, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query partyBRouterAtomBal")
				println("partyBRouterAtomBal: ", partyBRouterAtomBal)

				if partyARouterOsmoBal != 0 && partyBRouterAtomBal != 0 {
					break
				}
			}
		})

		t.Run("tick until routers route the funds to final receivers", func(t *testing.T) {

			const maxTicks = 50
			tick := 1
			for tick <= maxTicks {
				partyARouterAtomBal, err := cosmosNeutron.GetBalance(ctx, partyARouterAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query partyARouterBal")
				println("partyARouter atom bal: ", partyARouterAtomBal)
				partyARouterOsmoBal, err := cosmosNeutron.GetBalance(ctx, partyARouterAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query partyARouterOsmoBal")
				println("partyARouter osmo bal: ", partyARouterOsmoBal)
				partyBRouterOsmoBal, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query partyBRouterOsmoBal")
				println("partyBRouterOsmoBal: ", partyBRouterOsmoBal)
				partyBRouterAtomBal, err := cosmosNeutron.GetBalance(ctx, partyBRouterAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query partyBRouterAtomBal")
				println("partyBRouterAtomBal: ", partyBRouterAtomBal)

				println("Ticking clock ", tick, " of ", maxTicks)
				tickClock()

				osmoBal, err := cosmosOsmosis.GetBalance(ctx, osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom)
				require.NoError(t, err, "failed to query osmoBal")
				println("osmo user atom bal: ", osmoBal)
				gaiaBal, err := cosmosAtom.GetBalance(ctx, gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query gaiaBal")
				println("gaia user osmo bal: ", gaiaBal)

				if osmoBal != 0 && gaiaBal != 0 {
					break
				}
			}
		})
	})
}
