package covenant_swap

import (
	"context"
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
const gaiaOsmosisIBCPath = "go-ibc-path"
const neutronOsmosisIBCPath = "no-ibc-path"
const nativeAtomDenom = "uatom"
const nativeOsmoDenom = "uosmo"
const nativeNtrnDenom = "untrn"

var covenantAddress string
var clockAddress string
var splitterAddress string
var partyARouterAddress, partyBRouterAddress string
var partyAIbcForwarderAddress, partyBIbcForwarderAddress string
var partyADepositAddress, partyBDepositAddress string
var holderAddress string
var neutronAtomIbcDenom, neutronOsmoIbcDenom string
var atomNeutronICSConnectionId, neutronAtomICSConnectionId string
var neutronOsmosisIBCConnId, osmosisNeutronIBCConnId string
var atomNeutronIBCConnId, neutronAtomIBCConnId string
var gaiaOsmosisIBCConnId, osmosisGaiaIBCConnId string
var hubNeutronIbcDenom string

// PARTY_A
const neutronContributionAmount uint64 = 100_000_000_000 // in untrn

// PARTY_B
const atomContributionAmount uint64 = 5_000_000_000 // in uatom

// sets up and tests a tokenswap between hub and osmo facilitated by neutron
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
		{
			Name:    "gaia",
			Version: "v9.1.0",
			ChainConfig: ibc.ChainConfig{
				GasAdjustment:       1.3,
				GasPrices:           "0.0atom",
				ModifyGenesis:       utils.SetupGaiaGenesis(utils.GetDefaultInterchainGenesisMessages()),
				ConfigFileOverrides: configFileOverrides,
			},
		},
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

		require.NoError(t,
			r.UpdatePath(ctx, eRep, gaiaNeutronICSPath, ibc.PathUpdateOptions{
				SrcClientID: &neutronClients[0].ClientID,
				DstClientID: &atomClients[0].ClientID,
			}),
		)

		atomNeutronICSConnectionId, neutronAtomICSConnectionId = utils.GenerateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)
		utils.GenerateICSChannel(t, ctx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)
		utils.CreateValidator(t, ctx, r, eRep, atom, neutron)
		testCtx.SkipBlocks(3)
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
	_, _, _ = gaiaUser, neutronUser, osmoUser

	hubNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]
	neutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(1), neutron)[0]

	var neutronReceiverAddr string
	var hubReceiverAddr string

	testCtx.SkipBlocks(10)

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
		neutronAtomIbcDenom = testCtx.GetIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
			nativeAtomDenom,
		)
		neutronOsmoIbcDenom = testCtx.GetIbcDenom(
			testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
			nativeOsmoDenom,
		)
		hubNeutronIbcDenom = testCtx.GetIbcDenom(
			testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
			cosmosNeutron.Config().Denom,
		)
	})

	t.Run("tokenswap covenant setup", func(t *testing.T) {
		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_swap.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const interchainRouterContractPath = "wasms/covenant_interchain_router.wasm"
		const nativeRouterContractPath = "wasms/covenant_native_router.wasm"
		const splitterContractPath = "wasms/covenant_native_splitter.wasm"
		const ibcForwarderContractPath = "wasms/covenant_ibc_forwarder.wasm"
		const swapHolderContractPath = "wasms/covenant_swap_holder.wasm"

		// After storing on Neutron, we will receive a code id
		// We parse all the subcontracts into uint64
		// The will be required when we instantiate the covenant.
		var clockCodeId uint64
		var interchainRouterCodeId uint64
		var nativeRouterCodeId uint64
		var splitterCodeId uint64
		var ibcForwarderCodeId uint64
		var swapHolderCodeId uint64
		var covenantCodeId uint64

		t.Run("deploy covenant contracts", func(t *testing.T) {
			covenantCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, covenantContractPath)
			clockCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, clockContractPath)
			interchainRouterCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, interchainRouterContractPath)
			nativeRouterCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, nativeRouterContractPath)
			splitterCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, splitterContractPath)
			ibcForwarderCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, ibcForwarderContractPath)
			swapHolderCodeId = testCtx.StoreContract(cosmosNeutron, neutronUser, swapHolderContractPath)
		})

		t.Run("instantiate covenant", func(t *testing.T) {
			timeouts := Timeouts{
				IcaTimeout:         "10000", // sec
				IbcTransferTimeout: "10000", // sec
			}

			partyACoin := Coin{
				Denom:  nativeAtomDenom,
				Amount: strconv.FormatUint(atomContributionAmount, 10),
			}
			partyBCoin := Coin{
				Denom:  cosmosNeutron.Config().Denom,
				Amount: strconv.FormatUint(neutronContributionAmount, 10),
			}

			currentHeight, err := cosmosNeutron.Height(ctx)
			require.NoError(t, err, "failed to get neutron height")
			depositBlock := Block(currentHeight + 350)
			lockupConfig := Expiration{
				AtHeight: &depositBlock,
			}

			neutronReceiverAddr = neutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix)
			hubReceiverAddr = gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix)

			splits := map[string]SplitConfig{
				neutronAtomIbcDenom: {
					Receivers: map[string]string{
						neutronReceiverAddr: "1.0",
						hubReceiverAddr:     "0.0",
					},
				},
				cosmosNeutron.Config().Denom: {
					Receivers: map[string]string{
						neutronReceiverAddr: "0.0",
						hubReceiverAddr:     "1.0",
					},
				},
			}

			denomToPfmMap := map[string]PacketForwardMiddlewareConfig{}

			partyAConfig := InterchainCovenantParty{
				Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
				NativeDenom:               neutronAtomIbcDenom,
				RemoteChainDenom:          nativeAtomDenom,
				PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
				PartyReceiverAddr:         hubReceiverAddr,
				PartyChainConnectionId:    neutronAtomIBCConnId,
				IbcTransferTimeout:        timeouts.IbcTransferTimeout,
				DenomToPfmMap:             denomToPfmMap,
				Contribution:              partyACoin,
			}
			partyBConfig := NativeCovenantParty{
				Addr:              neutronReceiverAddr,
				NativeDenom:       cosmosNeutron.Config().Denom,
				PartyReceiverAddr: neutronReceiverAddr,
				Contribution:      partyBCoin,
			}
			codeIds := SwapCovenantContractCodeIds{
				IbcForwarderCode:       ibcForwarderCodeId,
				InterchainRouterCode:   interchainRouterCodeId,
				NativeRouterCode:       nativeRouterCodeId,
				InterchainSplitterCode: splitterCodeId,
				ClockCode:              clockCodeId,
				HolderCode:             swapHolderCodeId,
			}

			covenantMsg := CovenantInstantiateMsg{
				Label:                       "swap-covenant",
				Timeouts:                    timeouts,
				SwapCovenantContractCodeIds: codeIds,
				LockupConfig:                lockupConfig,
				PartyAConfig: CovenantPartyConfig{
					Interchain: &partyAConfig,
				},
				PartyBConfig: CovenantPartyConfig{
					Native: &partyBConfig,
				},
				Splits: splits,
			}

			covenantAddress = testCtx.ManualInstantiate(covenantCodeId, covenantMsg, neutronUser, keyring.BackendTest)
			println("covenant address: ", covenantAddress)
		})

		t.Run("query covenant contracts", func(t *testing.T) {
			clockAddress = testCtx.QueryClockAddress(covenantAddress)
			holderAddress = testCtx.QueryHolderAddress(covenantAddress)
			splitterAddress = testCtx.QueryInterchainSplitterAddress(covenantAddress)
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
				splitterAddress,
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
	})

	t.Run("tokenswap run", func(t *testing.T) {

		t.Run("tick until forwarders create ICA", func(t *testing.T) {
			for {
				testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				forwarderAState := testCtx.QueryContractState(partyAIbcForwarderAddress)

				if forwarderAState == "ica_created" {
					partyADepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_a")
					partyBDepositAddress = testCtx.QueryDepositAddress(covenantAddress, "party_b")
					break
				}
			}
		})

		t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
			require.NoError(t,
				cosmosNeutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
					Address: partyBDepositAddress,
					Denom:   cosmosNeutron.Config().Denom,
					Amount:  int64(neutronContributionAmount),
				}),
				"failed to deposit neutron",
			)

			require.NoError(t,
				cosmosAtom.SendFunds(ctx, gaiaUser.KeyName, ibc.WalletAmount{
					Address: partyADepositAddress,
					Denom:   nativeAtomDenom,
					Amount:  int64(atomContributionAmount),
				}),
				"failed to fund gaia forwarder",
			)
			testCtx.SkipBlocks(5)
		})

		t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
			for {
				holderNeutronBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, holderAddress)
				holderAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				holderState := testCtx.QueryContractState(holderAddress)

				if holderAtomBal >= atomContributionAmount && holderNeutronBal >= neutronContributionAmount {
					println("holder atom bal: ", holderAtomBal)
					println("holder neutron bal: ", holderNeutronBal)
					break
				} else if holderState == "complete" {
					println("holder state: ", holderState)
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until holder sends the funds to splitter", func(t *testing.T) {
			for {
				splitterNeutronBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, splitterAddress)
				splitterAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, splitterAddress)
				println("splitterNeutronBal: ", splitterNeutronBal)
				println("splitterAtomBal: ", splitterAtomBal)
				if splitterAtomBal >= atomContributionAmount && splitterNeutronBal >= neutronContributionAmount {
					println("splitter received contributions")
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until splitter sends the funds to routers", func(t *testing.T) {
			for {
				partyARouterNeutronBal := testCtx.QueryNeutronDenomBalance(cosmosNeutron.Config().Denom, partyARouterAddress)
				partyBRouterAtomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)
				println("partyARouterNeutronBal: ", partyARouterNeutronBal)
				println("partyBRouterAtomBal: ", partyBRouterAtomBal)

				if partyARouterNeutronBal >= neutronContributionAmount && partyBRouterAtomBal >= atomContributionAmount {
					println("both routers received contributions")
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until routers route the funds to final receivers", func(t *testing.T) {
			println("hub receiver address: ", hubReceiverAddr)
			println("neutron receiver address: ", neutronReceiverAddr)
			for {
				neutronBal := testCtx.QueryHubDenomBalance(hubNeutronIbcDenom, hubReceiverAddr)
				atomBal := testCtx.QueryNeutronDenomBalance(neutronAtomIbcDenom, neutronReceiverAddr)
				println("gaia user neutron bal: ", neutronBal)
				println("neutron user atom bal: ", atomBal)
				if neutronBal >= neutronContributionAmount && atomBal >= atomContributionAmount {
					println("complete")
					break
				} else {
					testCtx.Tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})
	})
}
