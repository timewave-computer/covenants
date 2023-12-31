package covenant_two_party_pol

import (
	"context"
	"encoding/json"
	"fmt"
	"path/filepath"
	"strconv"
	"testing"
	"time"

	cw "github.com/CosmWasm/wasmvm/types"
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

// sets up and tests a two party pol between hub and osmo facilitated by neutron
func TestTwoPartyOsmoPol(t *testing.T) {
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
				GasAdjustment:  2.0,
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
			Version: "v17.0.0",
			ChainConfig: ibc.ChainConfig{
				Type:         "cosmos",
				Bin:          "osmosisd",
				Bech32Prefix: "osmo",
				Denom:        nativeOsmoDenom,
				ModifyGenesis: utils.SetupOsmoGenesis(
					append(utils.GetDefaultInterchainGenesisMessages(), "/ibc.applications.interchain_accounts.v1.InterchainAccount"),
				),
				GasPrices:           "0.005uosmo",
				GasAdjustment:       2.0,
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

	testCtx := &utils.TestContext{
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
		T:                         t,
		Ctx:                       ctx,
	}

	var noteAddress string
	var voiceAddress string
	var proxyAddress string
	var osmoLiquidPoolerAddress string

	testCtx.SkipBlocks(5)

	t.Run("generate IBC paths", func(t *testing.T) {
		utils.GeneratePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosNeutron.Config().ChainID, gaiaNeutronIBCPath)
		utils.GeneratePath(t, ctx, r, eRep, cosmosAtom.Config().ChainID, cosmosOsmosis.Config().ChainID, gaiaOsmosisIBCPath)
		utils.GeneratePath(t, ctx, r, eRep, cosmosNeutron.Config().ChainID, cosmosOsmosis.Config().ChainID, neutronOsmosisIBCPath)
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
		utils.GenerateClient(t, ctx, testCtx, r, eRep, neutronOsmosisIBCPath, cosmosNeutron, cosmosOsmosis)
		neutronOsmosisIBCConnId, osmosisNeutronIBCConnId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, neutronOsmosisIBCPath, cosmosNeutron, cosmosOsmosis)
		utils.LinkPath(t, ctx, r, eRep, cosmosNeutron, cosmosOsmosis, neutronOsmosisIBCPath)

		utils.GenerateClient(t, ctx, testCtx, r, eRep, gaiaOsmosisIBCPath, cosmosAtom, cosmosOsmosis)
		gaiaOsmosisIBCConnId, osmosisGaiaIBCConnId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, gaiaOsmosisIBCPath, cosmosAtom, cosmosOsmosis)
		utils.LinkPath(t, ctx, r, eRep, cosmosAtom, cosmosOsmosis, gaiaOsmosisIBCPath)

		utils.GenerateClient(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		atomNeutronIBCConnId, neutronAtomIBCConnId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronIBCPath, cosmosAtom, cosmosNeutron)
		utils.LinkPath(t, ctx, r, eRep, cosmosAtom, cosmosNeutron, gaiaNeutronIBCPath)
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
	testCtx.SkipBlocks(2)

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron, osmosis)
	gaiaUser, neutronUser, osmoUser := users[0], users[1], users[2]

	// initialPoolOsmoAmount := int64(600_000_000_000)
	initialPoolAtomAmount := int64(60_000_000_000)
	osmoHelperAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(999_000_000_000), osmosis)[0]
	// hubNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]
	// osmoNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]

	// rqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]
	// rqCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(osmoContributionAmount), osmosis)[0]

	// sideBasedRqCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]
	// sideBasedRqCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(osmoContributionAmount), osmosis)[0]

	// happyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]
	// happyCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(osmoContributionAmount), osmosis)[0]

	// sideBasedHappyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]
	// sideBasedHappyCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(osmoContributionAmount), osmosis)[0]

	testCtx.SkipBlocks(5)

	t.Run("determine ibc channels", func(t *testing.T) {
		neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)
		osmoChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosOsmosis.Config().ChainID)

		// Find all pairwise channels
		utils.GetPairwiseTransferChannelIds(testCtx, osmoChannelInfo, neutronChannelInfo, osmosisNeutronIBCConnId, neutronOsmosisIBCConnId, osmosis.Config().Name, neutron.Config().Name)
		utils.GetPairwiseTransferChannelIds(testCtx, osmoChannelInfo, gaiaChannelInfo, osmosisGaiaIBCConnId, gaiaOsmosisIBCConnId, osmosis.Config().Name, cosmosAtom.Config().Name)
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
		// 2. Osmo on neutron
		neutronOsmoIbcDenom = testCtx.GetIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
			nativeOsmoDenom,
		)
		// 3. hub atom => neutron => osmosis
		osmoNeutronAtomIbcDenom = testCtx.GetMultihopIbcDenom(
			[]string{
				testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
				testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
			},
			nativeAtomDenom,
		)
		// 4. osmosis osmo => neutron => hub
		gaiaNeutronOsmoIbcDenom = testCtx.GetMultihopIbcDenom(
			[]string{
				testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
			},
			nativeOsmoDenom,
		)
		// 5. hub atom => osmosis
		osmosisAtomIbcDenom = testCtx.GetIbcDenom(
			testCtx.OsmoTransferChannelIds[cosmosAtom.Config().Name],
			nativeAtomDenom,
		)
	})

	t.Run("two party pol covenant setup", func(t *testing.T) {
		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_two_party_pol.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const routerContractPath = "wasms/covenant_interchain_router.wasm"
		const ibcForwarderContractPath = "wasms/covenant_ibc_forwarder.wasm"
		const holderContractPath = "wasms/covenant_two_party_pol_holder.wasm"
		const liquidPoolerPath = "wasms/covenant_osmo_liquid_pooler.wasm"

		// After storing on Neutron, we will receive a code id
		// We parse all the subcontracts into uint64
		// The will be required when we instantiate the covenant.
		var clockCodeId uint64
		var routerCodeId uint64
		var ibcForwarderCodeId uint64
		var holderCodeId uint64
		var lperCodeId uint64
		var covenantCodeId uint64
		var covenantRqCodeId uint64
		var covenantSideBasedRqCodeId uint64
		var noteCodeId uint64
		var voiceCodeId uint64
		var proxyCodeId uint64

		_, _, _, _, _ = clockCodeId, routerCodeId, ibcForwarderCodeId, holderCodeId, lperCodeId
		_, _, _ = covenantCodeId, covenantRqCodeId, covenantSideBasedRqCodeId

		t.Run("deploy covenant contracts", func(t *testing.T) {
			// something was going wrong with instantiating the same code twice,
			// hence this weird workaround
			covenantCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)
			covenantRqCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)
			covenantSideBasedRqCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)

			// store clock and get code id
			clockCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, clockContractPath)

			// store router and get code id
			routerCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, routerContractPath)

			// store forwarder and get code id
			ibcForwarderCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, ibcForwarderContractPath)

			// store lper, get code
			lperCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, liquidPoolerPath)

			// store holder and get code id
			holderCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, holderContractPath)

			testCtx.SkipBlocks(5)
		})

		t.Run("store polytone", func(t *testing.T) {
			const polytoneNotePath = "wasms/polytone_note.wasm"
			const polytoneVoicePath = "wasms/polytone_voice.wasm"
			const polytoneProxyPath = "wasms/polytone_proxy.wasm"

			noteCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, polytoneNotePath)
			voiceCodeId = testCtx.StoreContract(cosmosOsmosis, osmoUser, polytoneVoicePath)
			proxyCodeId = testCtx.StoreContract(cosmosOsmosis, osmoUser, polytoneProxyPath)

			println("noteCodeId: ", noteCodeId)
			println("voiceCodeId: ", voiceCodeId)
			println("proxyCodeId: ", proxyCodeId)
		})

		t.Run("add liquidity to osmo-atom pool", func(t *testing.T) {

			// fund an address on osmosis that will provide liquidity
			// at 1:10 ratio of atom/osmo
			_, err := testCtx.Hub.SendIBCTransfer(
				testCtx.Ctx,
				testCtx.GaiaTransferChannelIds[cosmosOsmosis.Config().Name],
				gaiaUser.KeyName,
				ibc.WalletAmount{
					Address: osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix),
					Denom:   testCtx.Hub.Config().Denom,
					Amount:  initialPoolAtomAmount,
				},
				ibc.TransferOptions{})
			require.NoError(testCtx.T, err, err)

			testCtx.SkipBlocks(10)

			osmoBal, _ := testCtx.Osmosis.GetBalance(
				testCtx.Ctx,
				osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix),
				"uosmo",
			)
			atomBal, _ := testCtx.Osmosis.GetBalance(
				testCtx.Ctx,
				osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix),
				osmosisAtomIbcDenom,
			)
			println("osmo helper account atom balance: ", atomBal)
			println("osmo helper account osmo balance: ", osmoBal)

			osmosisPoolInitConfig := cosmos.OsmosisPoolParams{
				Weights:        fmt.Sprintf("10%s,1%s", osmosisAtomIbcDenom, osmosis.Config().Denom),
				InitialDeposit: fmt.Sprintf("50000000000%s,500000000000%s", osmosisAtomIbcDenom, osmosis.Config().Denom),
				SwapFee:        "0.003",
				ExitFee:        "0.00",
				FutureGovernor: "",
			}

			// this fails because of wrong gas being set in interchaintest
			// underlying `ExecTx` call. we call this just to write the
			// config file to the node.
			_, err = cosmos.OsmosisCreatePool(
				cosmosOsmosis,
				ctx,
				osmoHelperAccount.KeyName,
				osmosisPoolInitConfig,
			)
			require.NoError(t, err, err)
			testCtx.SkipBlocks(10)

			manualPoolCreationCmd := []string{
				"osmosisd", "tx", "gamm", "create-pool",
				"--pool-file", filepath.Join(cosmosOsmosis.HomeDir(), "pool.json"),
				"--from", osmoHelperAccount.KeyName,
				"--gas", "3502650",
				"--keyring-backend", keyring.BackendTest,
				"--output", "json",
				"--chain-id", cosmosOsmosis.Config().ChainID,
				"--node", cosmosOsmosis.GetRPCAddress(),
				"--home", cosmosOsmosis.HomeDir(),
				"--fees", "500000uosmo",
				"-y",
			}
			_, _, err = cosmosOsmosis.Exec(ctx, manualPoolCreationCmd, nil)
			require.NoError(testCtx.T, err, err)
			testCtx.SkipBlocks(5)

			queryPoolCmd := []string{"osmosisd", "q", "gamm", "num-pools",
				"--node", cosmosOsmosis.GetRPCAddress(),
				"--home", cosmosOsmosis.HomeDir(),
				"--output", "json",
				"--chain-id", cosmosOsmosis.Config().ChainID,
			}
			numPoolsQueryStdout, _, err := cosmosOsmosis.Exec(testCtx.Ctx, queryPoolCmd, nil)
			require.NoError(testCtx.T, err, err)
			var res map[string]string
			err = json.Unmarshal(numPoolsQueryStdout, &res)
			require.NoError(testCtx.T, err, err)
			poolId := res["num_pools"]
			println("pool id: ", poolId)
			newOsmoBal, _ := testCtx.Osmosis.GetBalance(
				testCtx.Ctx,
				osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix),
				"uosmo",
			)
			newAtomBal, _ := testCtx.Osmosis.GetBalance(
				testCtx.Ctx,
				osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix),
				osmosisAtomIbcDenom,
			)

			println("deposited osmo: ", osmoBal-newOsmoBal)
			println("deposited atom: ", atomBal-newAtomBal)
			testCtx.SkipBlocks(5)
		})

		t.Run("instantiate polytone note and listener on neutron", func(t *testing.T) {
			noteInstantiateMsg := NoteInstantiate{
				BlockMaxGas: "3010000",
			}

			noteAddress = testCtx.InstantiateCmdExecNeutron(noteCodeId, "note", noteInstantiateMsg, neutronUser, keyring.BackendTest)
			println("note address: ", noteAddress)
		})

		t.Run("instantiate polytone voice and tester on osmosis", func(t *testing.T) {

			voiceInstantiateMsg := VoiceInstantiate{
				ProxyCodeId: proxyCodeId,
				BlockMaxGas: 3010000,
			}

			voiceAddress = testCtx.InstantiateCmdExecOsmo(voiceCodeId, "voice", voiceInstantiateMsg, osmoUser, keyring.BackendTest)
			println("voice address: ", voiceAddress)
		})

		t.Run("create polytone channel", func(t *testing.T) {
			err = r.CreateChannel(
				ctx,
				eRep,
				neutronOsmosisIBCPath,
				ibc.CreateChannelOptions{
					SourcePortName: fmt.Sprintf("wasm.%s", noteAddress),
					DestPortName:   fmt.Sprintf("wasm.%s", voiceAddress),
					Order:          ibc.Unordered,
					Version:        "polytone-1",
					Override:       true,
				},
			)
			require.NoError(t, err, err)
			testCtx.SkipBlocks(10)
		})

		t.Run("create osmo liquid pooler", func(t *testing.T) {

			instantiateMsg := OsmoLiquidPoolerInstantiateMsg{
				PoolAddress:   noteAddress,
				ClockAddress:  noteAddress,
				HolderAddress: noteAddress,
				NoteAddress:   noteAddress,
				Coin1: cw.Coin{
					Denom:  osmosisAtomIbcDenom,
					Amount: strconv.FormatUint(atomContributionAmount, 10),
				},
				Coin2: cw.Coin{
					Denom:  testCtx.Osmosis.Config().Denom,
					Amount: strconv.FormatUint(osmoContributionAmount, 10),
				},
				PoolId: "1",
			}

			osmoLiquidPoolerAddress = testCtx.ManualInstantiate(lperCodeId, instantiateMsg, neutronUser, keyring.BackendTest)
			println("liquid pooler address: ", osmoLiquidPoolerAddress)
		})

		t.Run("tick liquid pooler until proxy is created", func(t *testing.T) {
			for {
				lperState := testCtx.QueryContractState(osmoLiquidPoolerAddress)
				println("osmo liquid pooler state: ", lperState)
				if lperState == "proxy_created" {
					proxyAddress = testCtx.QueryProxyAddress(osmoLiquidPoolerAddress)
					println("proxy address: ", proxyAddress)
					break
				} else {
					testCtx.Tick(osmoLiquidPoolerAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("fund proxy address with neutron and osmo tokens", func(t *testing.T) {
			err := cosmosOsmosis.SendFunds(
				testCtx.Ctx,
				osmoHelperAccount.KeyName,
				ibc.WalletAmount{
					Address: proxyAddress,
					Denom:   testCtx.Osmosis.Config().Denom,
					Amount:  int64(osmoContributionAmount),
				},
			)
			require.NoError(t, err, err)
			testCtx.SkipBlocks(5)

			err = cosmosOsmosis.SendFunds(
				testCtx.Ctx,
				osmoHelperAccount.KeyName,
				ibc.WalletAmount{
					Address: proxyAddress,
					Denom:   osmosisAtomIbcDenom,
					Amount:  int64(atomContributionAmount),
				},
			)
			require.NoError(t, err, err)
			testCtx.SkipBlocks(5)

			atomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
			require.Equal(t, atomContributionAmount, atomBal)
			osmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
			require.Equal(t, osmoContributionAmount, osmoBal)
		})

		t.Run("tick until pool is queried", func(t *testing.T) {
			for {
				testCtx.Tick(osmoLiquidPoolerAddress, keyring.BackendTest, neutronUser.KeyName)

				liquidPoolerState := testCtx.QueryContractState(osmoLiquidPoolerAddress)
				println("liquid pooler state: ", liquidPoolerState)
				if liquidPoolerState == "proxy_funded" {
					initAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
					initOsmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
					initGammBal := testCtx.QueryOsmoDenomBalance("gamm/pool/1", proxyAddress)
					println("initial proxy atom bal: ", initAtomBal)
					println("initial proxy osmo bal: ", initOsmoBal)
					println("initial proxy gamm bal: ", initGammBal)
					break
				}
			}

		})

		t.Run("tick liquid pooler until proxy LPs the funds", func(t *testing.T) {
			for {
				testCtx.Tick(osmoLiquidPoolerAddress, keyring.BackendTest, neutronUser.KeyName)
				atomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
				osmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
				gammBal := testCtx.QueryOsmoDenomBalance("gamm/pool/1", proxyAddress)
				println("proxy atom bal: ", atomBal)
				println("proxy osmo bal: ", osmoBal)
				println("proxy gamm bal: ", gammBal)

				if gammBal != 0 {
					break
				}
			}

			testCtx.SkipBlocks(200)
		})

		// t.Run("enter LP pool via proxy", func(t *testing.T) {

		// 	msgJoinPool := MsgJoinPool{
		// 		Sender:         proxyAddress,
		// 		PoolId:         1,
		// 		ShareOutAmount: "1",
		// 		// this should be v1beta1 Coin instead of cw
		// 		TokenInMaxs: []cw.Coin{
		// 			{
		// 				Denom:  testCtx.Osmosis.Config().Denom,
		// 				Amount: strconv.FormatUint(osmoContributionAmount, 10),
		// 			},
		// 			{
		// 				Denom:  osmosisAtomIbcDenom,
		// 				Amount: strconv.FormatUint(atomContributionAmount, 10),
		// 			},
		// 		},
		// 	}
		// 	marshalled, err := json.Marshal(msgJoinPool)
		// 	require.NoError(t, err, err)

		// 	osmoJoinPoolMsg := cw.CosmosMsg{
		// 		Stargate: &cw.StargateMsg{
		// 			TypeURL: "osmosis.gamm.v1beta1.MsgJoinPool",
		// 			Value:   marshalled,
		// 		},
		// 	}

		// 	noteMessage := NoteExecuteMsg{
		// 		Msgs:           []cw.CosmosMsg{osmoJoinPoolMsg},
		// 		TimeoutSeconds: 200,
		// 		Callback: &CallbackRequest{
		// 			Receiver: listenerAddress,
		// 			Msg:      "YWxsZ29vZA", // allgood
		// 		},
		// 	}

		// 	noteExecute := NoteExecute{
		// 		Execute: &noteMessage,
		// 	}

		// 	marshalled, err = json.Marshal(noteExecute)
		// 	require.NoError(t, err, err)

		// 	testCtx.ManualExecNeutron(
		// 		noteAddress,
		// 		string(marshalled),
		// 		neutronUser,
		// 		keyring.BackendTest,
		// 	)
		// 	osmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
		// 	atomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
		// 	println("proxy osmo bal: ", osmoBal)
		// 	println("proxy atom bal: ", atomBal)

		// 	testCtx.SkipBlocks(10)

		// 	marshalledStargateMsg, _ := json.Marshal(osmoJoinPoolMsg)

		// 	wasmWrappedStargateMsg := cw.CosmosMsg{
		// 		Wasm: &cw.WasmMsg{
		// 			Execute: &cw.ExecuteMsg{
		// 				ContractAddr: proxyAddress,
		// 				Msg:          []byte(marshalledStargateMsg),
		// 				Funds:        []cw.Coin{},
		// 			},
		// 		},
		// 	}

		// 	noteMessage = NoteExecuteMsg{
		// 		Msgs:           []cw.CosmosMsg{wasmWrappedStargateMsg},
		// 		TimeoutSeconds: 200,
		// 		Callback: &CallbackRequest{
		// 			Receiver: listenerAddress,
		// 			Msg:      "YWxsZ29vZA", // allgood
		// 		},
		// 	}

		// 	noteExecute = NoteExecute{
		// 		Execute: &noteMessage,
		// 	}

		// 	marshalled, err = json.Marshal(noteExecute)
		// 	require.NoError(t, err, err)

		// 	testCtx.ManualExecNeutron(
		// 		noteAddress,
		// 		string(marshalled),
		// 		neutronUser,
		// 		keyring.BackendTest,
		// 	)
		// 	osmoBal = testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
		// 	atomBal = testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
		// 	println("proxy osmo bal: ", osmoBal)
		// 	println("proxy atom bal: ", atomBal)

		// 	testCtx.SkipBlocks(200)

		// })

		// t.Run("query proxy address", func(t *testing.T) {
		// 	// query the remote address
		// 	remoteAddrQuery := NoteQueryMsg{
		// 		RemoteAddressQuery: RemoteAddress{
		// 			LocalAddress: neutronUser.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 		},
		// 	}

		// 	type QueryResponse struct {
		// 		Data string `json:"data"`
		// 	}
		// 	var queryResponse QueryResponse
		// 	err := testCtx.Neutron.QueryContract(testCtx.Ctx, noteAddress, remoteAddrQuery, &queryResponse)
		// 	require.NoError(t, err, err)
		// 	require.Equal(t, "", queryResponse.Data, "no proxy should exist before first execute msg")

		// 	testCtx.SkipBlocks(3)

		// 	noteCreateAccountMessage := NoteExecuteMsg{
		// 		Msgs:           []cw.CosmosMsg{},
		// 		TimeoutSeconds: 100,
		// 		Callback: &CallbackRequest{
		// 			Receiver: listenerAddress,
		// 			Msg:      "aGVsbG8K",
		// 		},
		// 	}
		// 	noteExecute := NoteExecute{
		// 		Execute: &noteCreateAccountMessage,
		// 	}

		// 	testCtx.ManualExecNeutron(noteAddress, noteExecute, neutronUser, keyring.BackendTest)

		// 	testCtx.SkipBlocks(15)

		// 	err = testCtx.Neutron.QueryContract(testCtx.Ctx, noteAddress, remoteAddrQuery, &queryResponse)
		// 	require.NoError(testCtx.T, err, err)
		// 	require.NotEmpty(testCtx.T, queryResponse.Data, "proxy account failed to be created")
		// 	proxyAddress = queryResponse.Data
		// 	println("proxy address: ", proxyAddress)
		// })

		// t.Run("two party POL happy path", func(t *testing.T) {
		// 	var depositBlock Block
		// 	var lockupBlock Block

		// 	t.Run("instantiate covenant", func(t *testing.T) {
		// 		timeouts := Timeouts{
		// 			IcaTimeout:         "100", // sec
		// 			IbcTransferTimeout: "100", // sec
		// 		}

		// 		currentHeight := testCtx.getNeutronHeight()
		// 		depositBlock = Block(currentHeight + 200)
		// 		lockupBlock = Block(currentHeight + 200)

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

		// 		hubReceiverAddr := happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
		// 		osmoReceiverAddr := happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)

		// 		partyAConfig := InterchainCovenantParty{
		// 			Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			NativeDenom:               neutronAtomIbcDenom,
		// 			RemoteChainDenom:          "uatom",
		// 			PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
		// 			PartyReceiverAddr:         hubReceiverAddr,
		// 			PartyChainConnectionId:    neutronAtomIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 			Contribution:              atomCoin,
		// 		}
		// 		partyBConfig := InterchainCovenantParty{
		// 			Addr:                      osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
		// 			NativeDenom:               neutronOsmoIbcDenom,
		// 			RemoteChainDenom:          "uosmo",
		// 			PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
		// 			HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
		// 			PartyReceiverAddr:         osmoReceiverAddr,
		// 			PartyChainConnectionId:    neutronOsmosisIBCConnId,
		// 			IbcTransferTimeout:        timeouts.IbcTransferTimeout,
		// 			Contribution:              osmoCoin,
		// 		}
		// 		codeIds := ContractCodeIds{
		// 			IbcForwarderCode:     ibcForwarderCodeId,
		// 			InterchainRouterCode: routerCodeId,
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

		// 		denomSplits := []DenomSplit{
		// 			{
		// 				Denom: neutronAtomIbcDenom,
		// 				Type: SplitType{
		// 					Custom: SplitConfig{
		// 						Receivers: map[string]string{
		// 							hubReceiverAddr:  "0.5",
		// 							osmoReceiverAddr: "0.5",
		// 						},
		// 					},
		// 				},
		// 			},
		// 			{
		// 				Denom: neutronOsmoIbcDenom,
		// 				Type: SplitType{
		// 					Custom: SplitConfig{
		// 						Receivers: map[string]string{
		// 							hubReceiverAddr:  "0.5",
		// 							osmoReceiverAddr: "0.5",
		// 						},
		// 					},
		// 				},
		// 			},
		// 		}

		// 		covenantMsg := CovenantInstantiateMsg{
		// 			Label:           "two-party-pol-covenant-happy",
		// 			Timeouts:        timeouts,
		// 			PresetIbcFee:    presetIbcFee,
		// 			ContractCodeIds: codeIds,
		// 			LockupConfig:    lockupConfig,
		// 			PartyAConfig: CovenantPartyConfig{
		// 				Interchain: &partyAConfig,
		// 			},
		// 			PartyBConfig: CovenantPartyConfig{
		// 				Interchain: &partyBConfig,
		// 			},
		// 			PoolAddress:              poolAddress,
		// 			RagequitConfig:           &ragequitConfig,
		// 			DepositDeadline:          depositDeadline,
		// 			PartyAShare:              "50",
		// 			PartyBShare:              "50",
		// 			ExpectedPoolRatio:        "0.1",
		// 			AcceptablePoolRatioDelta: "0.09",
		// 			CovenantType:             "share",
		// 			PairType:                 pairType,
		// 			Splits:                   denomSplits,
		// 			FallbackSplit:            nil,
		// 		}

		// 		covenantAddress = testCtx.manualInstantiate(covenantCodeId, covenantMsg, neutronUser, keyring.BackendTest)

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
		// 	})

		// 	t.Run("tick until forwarders create ICA", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
		// 			forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

		// 			if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
		// 				partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
		// 				partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
		// 		testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, happyCaseOsmoAccount, int64(osmoContributionAmount))
		// 		testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, happyCaseHubAccount, int64(atomContributionAmount))

		// 		testCtx.SkipBlocks(3)
		// 	})

		// 	t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			holderOsmoBal := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
		// 			holderAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
		// 			holderState := testCtx.queryContractState(holderAddress)
		// 			println("holder ibc atom balance: ", holderAtomBal)
		// 			println("holder ibc osmo balance: ", holderOsmoBal)
		// 			println("holder state: ", holderState)

		// 			if holderAtomBal == atomContributionAmount && holderOsmoBal == osmoContributionAmount {
		// 				println("holder received atom & osmo")
		// 				break
		// 			} else if holderState == "active" {
		// 				println("holder: active")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick until holder sends funds to LiquidPooler and receives LP tokens in return", func(t *testing.T) {
		// 		for {
		// 			if testCtx.queryLpTokenBalance(liquidityTokenAddress, holderAddress) == 0 {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			} else {
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick until holder expires", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			holderState := testCtx.queryContractState(holderAddress)
		// 			println("holder state: ", holderState)

		// 			if holderState == "expired" {
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("party A claims and router receives the funds", func(t *testing.T) {
		// 		testCtx.SkipBlocks(10)
		// 		testCtx.holderClaim(holderAddress, hubNeutronAccount, keyring.BackendTest)
		// 		testCtx.SkipBlocks(5)
		// 		for {
		// 			routerOsmoBalA := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyARouterAddress)
		// 			routerAtomBalA := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyARouterAddress)
		// 			println("routerAtomBalA: ", routerAtomBalA)
		// 			println("routerOsmoBalA: ", routerOsmoBalA)
		// 			if routerAtomBalA != 0 && routerOsmoBalA != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick until party A claim is distributed", func(t *testing.T) {
		// 		for {
		// 			atomBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom)
		// 			osmoBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom)

		// 			println("party A atom bal: ", atomBalPartyA)
		// 			println("party A osmo bal: ", osmoBalPartyA)

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
		// 			routerOsmoBalB := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyBRouterAddress)
		// 			routerAtomBalB := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
		// 			println("routerAtomBalB: ", routerAtomBalB)
		// 			println("routerOsmoBalB: ", routerOsmoBalB)
		// 			if routerAtomBalB != 0 && routerOsmoBalB != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})

		// 	t.Run("tick routers until both parties receive their funds", func(t *testing.T) {
		// 		for {
		// 			osmoBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom)
		// 			osmoBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), cosmosOsmosis.Config().Denom)
		// 			atomBalPartyA, _ := cosmosAtom.GetBalance(
		// 				ctx, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix), cosmosAtom.Config().Denom)
		// 			atomBalPartyB, _ := cosmosOsmosis.GetBalance(
		// 				ctx, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom)

		// 			println("party A osmo bal: ", osmoBalPartyA)
		// 			println("party A atom bal: ", atomBalPartyA)
		// 			println("party B osmo bal: ", osmoBalPartyB)
		// 			println("party B atom bal: ", atomBalPartyB)

		// 			if atomBalPartyA != 0 && osmoBalPartyB != 0 {
		// 				break
		// 			} else {
		// 				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
		// 			}
		// 		}
		// 	})
		// })

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
		// 			InterchainRouterCode: routerCodeId,
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
		// 		testCtx.SkipBlocks(5)
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
		// 			forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

		// 			if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
		// 				testCtx.SkipBlocks(3)
		// 				partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
		// 				partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
		// 		testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, rqCaseOsmoAccount, int64(osmoContributionAmount))
		// 		testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, rqCaseHubAccount, int64(atomContributionAmount))

		// 		testCtx.SkipBlocks(3)
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
		// 		testCtx.SkipBlocks(10)
		// 		testCtx.holderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
		// 		testCtx.SkipBlocks(5)
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
		// 			InterchainRouterCode: routerCodeId,
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

		// 		testCtx.SkipBlocks(2)
		// 	})

		// 	t.Run("tick until forwarders create ICA", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
		// 			forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

		// 			if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
		// 				testCtx.SkipBlocks(5)
		// 				partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
		// 				partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
		// 		testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, sideBasedRqCaseOsmoAccount, int64(osmoContributionAmount))
		// 		testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, sideBasedRqCaseHubAccount, int64(atomContributionAmount))

		// 		testCtx.SkipBlocks(3)

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
		// 		testCtx.SkipBlocks(10)
		// 		testCtx.holderRagequit(holderAddress, hubNeutronAccount, keyring.BackendTest)
		// 		testCtx.SkipBlocks(5)
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
		// 		depositBlock := Block(currentHeight + 200)
		// 		lockupBlock := Block(currentHeight + 200)
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
		// 			InterchainRouterCode: routerCodeId,
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

		// 		testCtx.SkipBlocks(2)
		// 	})

		// 	t.Run("tick until forwarders create ICA", func(t *testing.T) {
		// 		for {
		// 			testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

		// 			forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
		// 			forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

		// 			if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
		// 				testCtx.SkipBlocks(5)
		// 				partyADepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_a")
		// 				partyBDepositAddress = testCtx.queryDepositAddress(covenantAddress, "party_b")
		// 				break
		// 			}
		// 		}
		// 	})

		// 	t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
		// 		testCtx.fundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, sideBasedHappyCaseOsmoAccount, int64(osmoContributionAmount))
		// 		testCtx.fundChainAddrs([]string{partyADepositAddress}, cosmosAtom, sideBasedHappyCaseHubAccount, int64(atomContributionAmount))

		// 		testCtx.SkipBlocks(3)

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
		// 	})
	})
}
