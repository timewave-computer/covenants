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
				Bin:                 "neutrond",
				Bech32Prefix:        "neutron",
				Denom:               "untrn",
				GasPrices:           "0.0untrn,0.0uatom",
				GasAdjustment:       1.3,
				TrustingPeriod:      "1197504s",
				NoHostMount:         false,
				ModifyGenesis:       setupNeutronGenesis("0.05", []string{"untrn"}, []string{"uatom"}),
				ConfigFileOverrides: configFileOverrides,
			},
		},
		{
			Name:    "osmosis",
			Version: "v11.0.0",
			ChainConfig: ibc.ChainConfig{
				Type:          "cosmos",
				Bin:           "osmosisd",
				Bech32Prefix:  "osmo",
				Denom:         "uosmo",
				GasPrices:     "0.0uosmo",
				GasAdjustment: 1.3,
				Images: []ibc.DockerImage{
					{
						Repository: "ghcr.io/strangelove-ventures/heighliner/osmosis",
						Version:    "v11.0.0",
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
	neutronOsmoIbcDenom := testCtx.getIbcDenom(testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name], cosmosOsmosis.Config().Denom)

	print("\nneutronAtomIbcDenom: ", neutronAtomIbcDenom)
	print("\nneutronOsmoIbcDenom: ", neutronOsmoIbcDenom)

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

			presetIbcFee := PresetIbcFee{
				AckFee:     "1000",
				TimeoutFee: "1000",
			}

			timeouts := Timeouts{
				IcaTimeout:         "10", // sec
				IbcTransferTimeout: "5",  // sec
			}

			swapCovenantTerms := SwapCovenantTerms{
				PartyAAmount: strconv.FormatUint(atomContributionAmount, 10),
				PartyBAmount: strconv.FormatUint(osmoContributionAmount, 10),
			}

			covenantPartiesConfig := CovenantPartiesConfig{
				PartyA: CovenantParty{
					Addr:          gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					ProvidedDenom: "uatom",
					ReceiverConfig: ReceiverConfig{
						Native: gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					},
				},
				PartyB: CovenantParty{
					Addr:          neutronUser.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					ProvidedDenom: "untrn",
					ReceiverConfig: ReceiverConfig{
						Native: neutronUser.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
					},
				},
			}
			timestamp := Timestamp("1000000")

			lockupConfig := LockupConfig{
				Time: &timestamp,
			}
			covenantTerms := CovenantTerms{
				TokenSwap: swapCovenantTerms,
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
					Addr:                   gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					ProvidedDenom:          "uatom",
					PartyChainChannelId:    testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
					PartyReceiverAddr:      gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix),
					PartyChainConnectionId: neutronAtomIBCConnId,
					IbcTransferTimeout:     timeouts.IbcTransferTimeout,
				},
				PartyB: SwapPartyConfig{
					Addr:                   osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix),
					ProvidedDenom:          "uosmo",
					PartyChainChannelId:    testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
					PartyReceiverAddr:      osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix),
					PartyChainConnectionId: neutronOsmosisIBCConnId,
					IbcTransferTimeout:     timeouts.IbcTransferTimeout,
				},
			}
			covenantMsg := CovenantInstantiateMsg{
				Label:                  "swap-covenant",
				PresetIbcFee:           presetIbcFee,
				Timeouts:               timeouts,
				IbcForwarderCode:       ibcForwarderCodeId,
				InterchainRouterCode:   routerCodeId,
				InterchainSplitterCode: splitterCodeId,
				PresetClock:            clockMsg,
				PresetSwapHolder:       presetSwapHolder,
				SwapCovenantParties:    swapCovenantParties,
			}

			str, err := json.Marshal(covenantMsg)
			require.NoError(t, err, "Failed to marshall CovenantInstantiateMsg")
			println("covenant instantiation msg: ", string(str))
			instantiateMsg := string(str)

			// covenantContractAddress, err := cosmosNeutron.InstantiateContract(
			// 	ctx,
			// 	neutronUser.KeyName,
			// 	covenantCodeIdStr,
			// 	instantiateCmd,
			// 	true,
			// )
			// if err != nil {
			// 	println("error: ", err)
			// } else {
			// 	println("no error: ", covenantContractAddress)
			// }
			// require.NoError(t, testutil.WaitForBlocks(ctx, 100, atom, neutron, osmosis))

			cmd := []string{"neutrond", "tx", "wasm", "instantiate", covenantCodeIdStr,
				instantiateMsg,
				"--label", "swap-covenant",
				"--no-admin",
				"--from", neutronUser.KeyName,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--chain-id", neutron.Config().ChainID,
				"--gas", "9000000",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			resp, _, err := neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			println("instantiated, skipping 10 blocks...")
			require.NoError(t, testutil.WaitForBlocks(ctx, 10, atom, neutron, osmosis))

			println("instantiate response: ", string(resp), "\n")

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

			contractAddress := contactsRes.Contracts[len(contactsRes.Contracts)-1]

			println("covenant address: ", contractAddress)

		})
	})
}
