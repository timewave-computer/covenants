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
	var osmoOutpost string

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

	initialPoolAtomAmount := int64(60_000_000_000)
	osmoHelperAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(999_000_000_000), osmosis)[0]

	happyCaseHubAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(atomContributionAmount), atom)[0]
	happyCaseOsmoAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", 5*int64(osmoContributionAmount), osmosis)[0]

	osmoPartyNeutronAddr := ibctest.GetAndFundTestUsers(t, ctx, "default", 100000000, neutron)[0]
	hubPartyNeutronAddr := ibctest.GetAndFundTestUsers(t, ctx, "default", 100000000, neutron)[0]

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

		hubOsmoIbcDenom = testCtx.GetIbcDenom(
			testCtx.GaiaTransferChannelIds[cosmosOsmosis.Config().Name],
			nativeOsmoDenom,
		)
	})

	t.Run("two party pol covenant setup", func(t *testing.T) {
		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_two_party_pol.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const interchainRouterContractPath = "wasms/covenant_interchain_router.wasm"
		const ibcForwarderContractPath = "wasms/covenant_ibc_forwarder.wasm"
		const holderContractPath = "wasms/covenant_two_party_pol_holder.wasm"
		const liquidPoolerPath = "wasms/covenant_osmo_liquid_pooler.wasm"
		const osmoOutpostPath = "wasms/covenant_outpost_osmo_liquid_pooler.wasm"
		const nativeRouterContractPath = "wasms/covenant_native_router.wasm"

		// After storing on Neutron, we will receive a code id
		// We parse all the subcontracts into uint64
		// The will be required when we instantiate the covenant.
		var clockCodeId uint64
		var nativeRouterCodeId uint64
		var interchainRouterCodeId uint64
		var ibcForwarderCodeId uint64
		var holderCodeId uint64
		var lperCodeId uint64
		var covenantCodeId uint64
		var covenantRqCodeId uint64
		var covenantSideBasedRqCodeId uint64
		var noteCodeId uint64
		var voiceCodeId uint64
		var proxyCodeId uint64
		var osmoOutpostCodeId uint64

		_, _, _, _, _, _ = clockCodeId, nativeRouterCodeId, interchainRouterCodeId, ibcForwarderCodeId, holderCodeId, lperCodeId
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
			nativeRouterCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, nativeRouterContractPath)

			// store router and get code id
			interchainRouterCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, interchainRouterContractPath)

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

			// store lper, get code
			osmoOutpostCodeId = testCtx.StoreContract(cosmosOsmosis, osmoUser, osmoOutpostPath)

			println("noteCodeId: ", noteCodeId)
			println("voiceCodeId: ", voiceCodeId)
			println("proxyCodeId: ", proxyCodeId)
			println("osmoOutpostCodeId: ", osmoOutpostCodeId)
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

			osmoBal := testCtx.QueryOsmoDenomBalance("uosmo", osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix))
			atomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix))
			println("osmo helper account atom balance: ", atomBal)
			println("osmo helper account osmo balance: ", osmoBal)

			// pool initialized with 0.105~ ratio
			osmosisPoolInitConfig := cosmos.OsmosisPoolParams{
				Weights:        fmt.Sprintf("50%s,50%s", osmosisAtomIbcDenom, osmosis.Config().Denom),
				InitialDeposit: fmt.Sprintf("40000000000%s,330000000000%s", osmosisAtomIbcDenom, osmosis.Config().Denom),
				SwapFee:        "0.003",
				ExitFee:        "0.00",
				FutureGovernor: "",
			}

			// this fails because of wrong gas being set in interchaintest
			// underlying `ExecTx` call. we call this just to write the
			// config file to the node.
			_, err = cosmos.OsmosisCreatePool(
				testCtx.Osmosis,
				testCtx.Ctx,
				osmoHelperAccount.KeyName,
				osmosisPoolInitConfig,
			)
			require.NoError(testCtx.T, err, err)
			testCtx.SkipBlocks(10)

			manualPoolCreationCmd := []string{
				"osmosisd", "tx", "gamm", "create-pool",
				"--pool-file", filepath.Join(testCtx.Osmosis.HomeDir(), "pool.json"),
				"--from", osmoHelperAccount.KeyName,
				"--gas", "3502650",
				"--keyring-backend", keyring.BackendTest,
				"--output", "json",
				"--chain-id", testCtx.Osmosis.Config().ChainID,
				"--node", testCtx.Osmosis.GetRPCAddress(),
				"--home", testCtx.Osmosis.HomeDir(),
				"--fees", "50000uosmo",
				"-y",
			}
			_, _, err = testCtx.Osmosis.Exec(testCtx.Ctx, manualPoolCreationCmd, nil)
			require.NoError(testCtx.T, err, err)
			testCtx.SkipBlocks(5)

			queryPoolCmd := []string{"osmosisd", "q", "gamm", "num-pools",
				"--node", testCtx.Osmosis.GetRPCAddress(),
				"--home", testCtx.Osmosis.HomeDir(),
				"--output", "json",
				"--chain-id", testCtx.Osmosis.Config().ChainID,
			}
			numPoolsQueryStdout, _, err := testCtx.Osmosis.Exec(testCtx.Ctx, queryPoolCmd, nil)
			require.NoError(testCtx.T, err, err)
			var res map[string]string
			err = json.Unmarshal(numPoolsQueryStdout, &res)
			require.NoError(testCtx.T, err, err)
			poolId := res["num_pools"]
			println("pool id: ", poolId)
			newOsmoBal := testCtx.QueryOsmoDenomBalance("uosmo", osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix))
			newAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, osmoHelperAccount.Bech32Address(testCtx.Osmosis.Config().Bech32Prefix))

			println("deposited osmo: ", uint64(osmoBal)-newOsmoBal)
			println("deposited atom: ", uint64(atomBal)-newAtomBal)
			testCtx.SkipBlocks(5)
		})

		t.Run("instantiate osmosis outpost", func(t *testing.T) {
			osmoOutpost = testCtx.InstantiateOsmoOutpost(osmoOutpostCodeId, osmoUser)
			println(osmoOutpost)
		})

		t.Run("instantiate polytone note and listener on neutron", func(t *testing.T) {
			noteInstantiateMsg := NoteInstantiate{
				BlockMaxGas: "3010000",
			}

			noteAddress = testCtx.InstantiateCmdExecNeutron(noteCodeId, "note", noteInstantiateMsg, neutronUser, keyring.BackendTest)
			println("note address: ", noteAddress)
		})

		t.Run("instantiate polytone voice on osmosis", func(t *testing.T) {

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
				AckFee:     "10000",
				TimeoutFee: "10000",
			}

			hubReceiverAddr := happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix)
			osmoReceiverAddr := happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)

			atomCoin := Coin{
				Denom:  cosmosAtom.Config().Denom,
				Amount: strconv.FormatUint(atomContributionAmount, 10),
			}

			osmoCoin := Coin{
				Denom:  cosmosOsmosis.Config().Denom,
				Amount: strconv.FormatUint(osmoContributionAmount, 10),
			}

			outwardsPfm := ForwardMetadata{
				Receiver: gaiaUser.Bech32Address(testCtx.Hub.Config().Bech32Prefix),
				Port:     "transfer",
				Channel:  testCtx.GaiaTransferChannelIds[testCtx.Osmosis.Config().Name],
			}

			inwardsPfm := ForwardMetadata{
				Receiver: gaiaUser.Bech32Address(testCtx.Hub.Config().Bech32Prefix),
				Port:     "transfer",
				Channel:  testCtx.OsmoTransferChannelIds[testCtx.Hub.Config().Name],
			}

			codeIds := ContractCodeIds{
				IbcForwarderCode:     ibcForwarderCodeId,
				InterchainRouterCode: interchainRouterCodeId,
				NativeRouterCode:     nativeRouterCodeId,
				ClockCode:            clockCodeId,
				HolderCode:           holderCodeId,
				LiquidPoolerCode:     lperCodeId,
			}

			denomSplits := []DenomSplit{
				{
					Denom: neutronAtomIbcDenom,
					Type: SplitType{
						Custom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:  "0.5",
								osmoReceiverAddr: "0.5",
							},
						},
					},
				},
				{
					Denom: neutronOsmoIbcDenom,
					Type: SplitType{
						Custom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:  "0.5",
								osmoReceiverAddr: "0.5",
							},
						},
					},
				},
			}

			partyAConfig := InterchainCovenantParty{
				Addr:                      hubPartyNeutronAddr.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
				NativeDenom:               neutronAtomIbcDenom,
				RemoteChainDenom:          "uatom",
				PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
				PartyReceiverAddr:         hubReceiverAddr,
				PartyChainConnectionId:    neutronAtomIBCConnId,
				IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				Contribution:              atomCoin,
			}
			partyBConfig := InterchainCovenantParty{
				Addr:                      osmoPartyNeutronAddr.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
				NativeDenom:               neutronOsmoIbcDenom,
				RemoteChainDenom:          "uosmo",
				PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
				HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
				PartyReceiverAddr:         osmoReceiverAddr,
				PartyChainConnectionId:    neutronOsmosisIBCConnId,
				IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				Contribution:              osmoCoin,
			}

			liquidPoolerConfig := LiquidPoolerConfig{
				Osmosis: &OsmosisLiquidPoolerConfig{
					NoteAddress:    noteAddress,
					PoolId:         "1",
					OsmoIbcTimeout: "300",
					Party1ChainInfo: PartyChainInfo{
						PartyChainToNeutronChannel: testCtx.GaiaTransferChannelIds[testCtx.Neutron.Config().Name],
						NeutronToPartyChainChannel: testCtx.NeutronTransferChannelIds[testCtx.Hub.Config().Name],
						InwardsPfm:                 &inwardsPfm,
						OutwardsPfm:                &outwardsPfm,
						IbcTimeout:                 "300",
					},
					Party2ChainInfo: PartyChainInfo{
						NeutronToPartyChainChannel: testCtx.NeutronTransferChannelIds[testCtx.Osmosis.Config().Name],
						PartyChainToNeutronChannel: testCtx.OsmoTransferChannelIds[testCtx.Neutron.Config().Name],
						IbcTimeout:                 "300",
					},
					OsmoToNeutronChannelId: testCtx.OsmoTransferChannelIds[testCtx.Neutron.Config().Name],
					Party1DenomInfo: PartyDenomInfo{
						OsmosisCoin:       cw.Coin{Denom: osmosisAtomIbcDenom, Amount: strconv.FormatUint(atomContributionAmount, 10)},
						LocalDenom:        neutronAtomIbcDenom,
						SingleSideLpLimit: "10000",
					},
					Party2DenomInfo: PartyDenomInfo{
						OsmosisCoin:       cw.Coin{Denom: testCtx.Osmosis.Config().Denom, Amount: strconv.FormatUint(osmoContributionAmount, 10)},
						LocalDenom:        neutronOsmoIbcDenom,
						SingleSideLpLimit: "975000004",
					},
					LpTokenDenom:           "gamm/pool/1",
					OsmoOutpost:            osmoOutpost,
					FundingDurationSeconds: "200",
				},
			}

			covenantInstantiateMsg := CovenantInstantiateMsg{
				Label:                    "covenant-osmo",
				Timeouts:                 timeouts,
				PresetIbcFee:             presetIbcFee,
				ContractCodeIds:          codeIds,
				LockupConfig:             lockupConfig,
				PartyAConfig:             CovenantPartyConfig{Interchain: &partyAConfig},
				PartyBConfig:             CovenantPartyConfig{Interchain: &partyBConfig},
				DepositDeadline:          depositDeadline,
				CovenantType:             "share",
				PartyAShare:              "50",
				PartyBShare:              "50",
				ExpectedPoolRatio:        "0.1",
				AcceptablePoolRatioDelta: "0.09",
				Splits:                   denomSplits,
				FallbackSplit:            nil,
				EmergencyCommittee:       neutronUser.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
				LiquidPoolerConfig:       liquidPoolerConfig,
			}

			covenantAddress = testCtx.ManualInstantiate(covenantCodeId, covenantInstantiateMsg, neutronUser, keyring.BackendTest)
			println("covenantAddress address: ", covenantAddress)
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
				partyAIbcForwarderAddress,
				partyBIbcForwarderAddress,
				clockAddress,
				partyARouterAddress,
				partyBRouterAddress,
				holderAddress,
				liquidPoolerAddress,
			}
			println("funding addresses with 5000000000untrn")
			testCtx.FundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)
		})

		t.Run("tick until forwarders create ICA", func(t *testing.T) {
			testCtx.SkipBlocks(5)
			for {
				testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)

				forwarderAState := testCtx.QueryContractState(partyAIbcForwarderAddress)
				forwarderBState := testCtx.QueryContractState(partyBIbcForwarderAddress)

				if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
					testCtx.SkipBlocks(3)
					partyADepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_a")
					partyBDepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_b")
					break
				}
			}
		})

		t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
			testCtx.FundChainAddrs([]string{partyBDepositAddress}, cosmosOsmosis, happyCaseOsmoAccount, int64(osmoContributionAmount))
			testCtx.FundChainAddrs([]string{partyADepositAddress}, cosmosAtom, happyCaseHubAccount, int64(atomContributionAmount))

			testCtx.SkipBlocks(5)

			osmoBal := testCtx.QueryOsmoDenomBalance(cosmosOsmosis.Config().Denom, partyBDepositAddress)
			atomBal := testCtx.QueryHubDenomBalance(nativeAtomDenom, partyADepositAddress)
			println("covenant party deposits")
			println(partyADepositAddress, " balance: ", atomBal, nativeAtomDenom)
			println(partyBDepositAddress, " balance: ", osmoBal, cosmosOsmosis.Config().Denom)
		})

		t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
			for {
				holderOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
				holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				holderState := testCtx.QueryContractState(holderAddress)

				println("holder atom bal: ", holderAtomBal)
				println("holder osmo bal: ", holderOsmoBal)
				println("holder state: ", holderState)

				if holderAtomBal == atomContributionAmount && holderOsmoBal == osmoContributionAmount {
					println("holder received atom & osmo")
					break
				} else if holderState == "active" {
					println("holder is active")
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until holder sends funds to LP", func(t *testing.T) {
			for {
				liquidPoolerAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, liquidPoolerAddress)
				liquidPoolerOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, liquidPoolerAddress)

				if liquidPoolerAtomBal == 0 && liquidPoolerOsmoBal != 0 {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				} else {
					break
				}
			}
		})

		t.Run("tick until liquid pooler proxy is created", func(t *testing.T) {
			for {
				lperState := testCtx.QueryContractState(liquidPoolerAddress)
				println("osmo liquid pooler state: ", lperState)
				if lperState == "proxy_created" {
					proxyAddress = testCtx.QueryProxyAddress(liquidPoolerAddress)
					println("proxy address: ", proxyAddress)
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until proxy is funded", func(t *testing.T) {
			for {
				proxyAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
				proxyOsmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
				println("proxy atom bal: ", proxyAtomBal)
				println("proxy osmo bal: ", proxyOsmoBal)
				if proxyAtomBal != 0 && proxyOsmoBal != 0 {
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until liquidity is provided and proxy receives gamm tokens", func(t *testing.T) {
			neutronGammDenom := testCtx.GetIbcDenom(
				testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
				"gamm/pool/1",
			)

			for {
				osmoLiquidPoolerGammBalance := testCtx.QueryNeutronDenomBalance(neutronGammDenom, liquidPoolerAddress)
				proxyGammBalance := testCtx.QueryOsmoDenomBalance("gamm/pool/1", proxyAddress)
				proxyAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
				proxyOsmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
				outpostAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, osmoOutpost)
				outpostOsmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, osmoOutpost)
				outpostGammBalance := testCtx.QueryOsmoDenomBalance("gamm/pool/1", osmoOutpost)

				println("proxy atom bal: ", proxyAtomBal)
				println("proxy osmo bal: ", proxyOsmoBal)
				println("outpost atom bal: ", outpostAtomBal)
				println("outpost osmo bal: ", outpostOsmoBal)
				println("outpost gamm token balance: ", outpostGammBalance)
				println("proxy gamm token balance: ", proxyGammBalance)
				println("osmo liquid pooler gamm token balance: ", osmoLiquidPoolerGammBalance)

				if proxyGammBalance != 0 && proxyAtomBal == 0 && proxyOsmoBal == 0 {
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
					testCtx.SkipBlocks(2)
				}
			}
		})

		t.Run("osmo party claims", func(t *testing.T) {

			// try to withdraw until lp tokens are gone from proxy
			testCtx.SkipBlocks(5)
			testCtx.HolderClaim(holderAddress, osmoPartyNeutronAddr, keyring.BackendTest)

			for {
				testCtx.SkipBlocks(5)

				testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				proxyGammBalance := testCtx.QueryOsmoDenomBalance("gamm/pool/1", proxyAddress)
				proxyAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
				proxyOsmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
				holderOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
				holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				lperAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, liquidPoolerAddress)
				lperOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, liquidPoolerAddress)
				osmoPartyReceiverAddrOsmoBal := testCtx.QueryOsmoDenomBalance("uosmo", happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix))
				osmoPartyReceiverAddrAtomBal := testCtx.QueryOsmoDenomBalance(osmoNeutronAtomIbcDenom, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix))

				println("holder osmo bal: ", holderOsmoBal)
				println("holder atom bal: ", holderAtomBal)

				println("proxy atom bal: ", proxyAtomBal)
				println("proxy osmo bal: ", proxyOsmoBal)
				println("proxy gamm token balance: ", proxyGammBalance)

				println("liquid pooler osmo bal: ", lperOsmoBal)
				println("liquid pooler atom bal: ", lperAtomBal)

				println("osmoPartyReceiverAddrOsmoBal", osmoPartyReceiverAddrOsmoBal)
				println("osmoPartyReceiverAddrAtomBal", osmoPartyReceiverAddrAtomBal)

				if osmoPartyReceiverAddrOsmoBal != 0 && osmoPartyReceiverAddrAtomBal != 0 {
					println("claiming party received the funds")
					break
				}
			}
		})

		t.Run("tick until we are able to withdraw", func(t *testing.T) {
			testCtx.SkipBlocks(10)
			tickCount := 0
			for {
				testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				testCtx.SkipBlocks(2)
				tickCount = tickCount + 1
				if tickCount == 6 {
					break
				}
			}

			testCtx.HolderClaim(holderAddress, hubPartyNeutronAddr, keyring.BackendTest)
			testCtx.SkipBlocks(10)

			for {
				testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				testCtx.SkipBlocks(5)
				proxyGammBalance := testCtx.QueryOsmoDenomBalance("gamm/pool/1", proxyAddress)
				proxyAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
				proxyOsmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
				holderOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
				holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				lperAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, liquidPoolerAddress)
				lperOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, liquidPoolerAddress)

				println("holder osmo bal: ", holderOsmoBal)
				println("holder atom bal: ", holderAtomBal)

				println("proxy atom bal: ", proxyAtomBal)
				println("proxy osmo bal: ", proxyOsmoBal)
				println("proxy gamm token balance: ", proxyGammBalance)

				println("liquid pooler osmo bal: ", lperOsmoBal)
				println("liquid pooler atom bal: ", lperAtomBal)

				hubPartyReceiverAddrAtomBal := testCtx.QueryHubDenomBalance("uatom", happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix))
				hubPartyReceiverAddrOsmoBal := testCtx.QueryHubDenomBalance(gaiaNeutronOsmoIbcDenom, happyCaseHubAccount.Bech32Address(cosmosAtom.Config().Bech32Prefix))

				println("hubPartyReceiverAddrAtomBal", hubPartyReceiverAddrAtomBal)
				println("hubPartyReceiverAddrOsmoBal", hubPartyReceiverAddrOsmoBal)

				if hubPartyReceiverAddrAtomBal != 0 && hubPartyReceiverAddrOsmoBal != 0 {
					println("claiming party received the funds")
					break
				}
			}
		})

		t.Run("osmo party claims", func(t *testing.T) {

			// try to withdraw until lp tokens are gone from proxy
			for {
				testCtx.SkipBlocks(5)
				testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				proxyGammBalance := testCtx.QueryOsmoDenomBalance("gamm/pool/1", proxyAddress)
				println("proxy gamm balance:", proxyGammBalance)
				if proxyGammBalance < 10 {
					break
				} else {
					testCtx.HolderClaim(holderAddress, osmoPartyNeutronAddr, keyring.BackendTest)
				}
			}

			for {
				testCtx.SkipBlocks(5)

				testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				proxyGammBalance := testCtx.QueryOsmoDenomBalance("gamm/pool/1", proxyAddress)
				proxyAtomBal := testCtx.QueryOsmoDenomBalance(osmosisAtomIbcDenom, proxyAddress)
				proxyOsmoBal := testCtx.QueryOsmoDenomBalance(testCtx.Osmosis.Config().Denom, proxyAddress)
				holderOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
				holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				lperAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, liquidPoolerAddress)
				lperOsmoBal := testCtx.QueryNeutronDenomBalance(neutronOsmoIbcDenom, liquidPoolerAddress)
				osmoPartyReceiverAddrOsmoBal := testCtx.QueryOsmoDenomBalance("uosmo", happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix))
				osmoPartyReceiverAddrAtomBal := testCtx.QueryOsmoDenomBalance(osmoNeutronAtomIbcDenom, happyCaseOsmoAccount.Bech32Address(cosmosOsmosis.Config().Bech32Prefix))

				println("holder osmo bal: ", holderOsmoBal)
				println("holder atom bal: ", holderAtomBal)

				println("proxy atom bal: ", proxyAtomBal)
				println("proxy osmo bal: ", proxyOsmoBal)
				println("proxy gamm token balance: ", proxyGammBalance)

				println("liquid pooler osmo bal: ", lperOsmoBal)
				println("liquid pooler atom bal: ", lperAtomBal)

				println("osmoPartyReceiverAddrOsmoBal", osmoPartyReceiverAddrOsmoBal)
				println("osmoPartyReceiverAddrAtomBal", osmoPartyReceiverAddrAtomBal)

				if osmoPartyReceiverAddrOsmoBal != 0 && osmoPartyReceiverAddrAtomBal != 0 {
					println("claiming party received the funds")
					break
				}
			}
		})
	})
}
