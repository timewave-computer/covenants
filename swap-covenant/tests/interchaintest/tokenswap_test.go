package ibc_test

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
var neutronAtomIbcDenom, neutronOsmoIbcDenom, osmoNeutronAtomIbcDenom, gaiaNeutronOsmoIbcDenom string
var atomNeutronICSConnectionId, neutronAtomICSConnectionId string
var neutronOsmosisIBCConnId, osmosisNeutronIBCConnId string
var atomNeutronIBCConnId, neutronAtomIBCConnId string
var gaiaOsmosisIBCConnId, osmosisGaiaIBCConnId string

// PARTY_A
const osmoContributionAmount uint64 = 100_000_000_000 // in uosmo

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
				ModifyGenesis:       setupGaiaGenesis(getDefaultInterchainGenesisMessages()),
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

		require.NoError(t,
			r.UpdatePath(ctx, eRep, gaiaNeutronICSPath, ibc.PathUpdateOptions{
				SrcClientID: &neutronClients[0].ClientID,
				DstClientID: &atomClients[0].ClientID,
			}),
		)

		atomNeutronICSConnectionId, neutronAtomICSConnectionId = generateConnections(t, ctx, testCtx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)
		generateICSChannel(t, ctx, r, eRep, gaiaNeutronICSPath, cosmosAtom, cosmosNeutron)
		createValidator(t, ctx, r, eRep, atom, neutron)
		testCtx.skipBlocks(3)
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

	testCtx.skipBlocks(2)

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron, osmosis)
	gaiaUser, neutronUser, osmoUser := users[0], users[1], users[2]
	_, _, _ = gaiaUser, neutronUser, osmoUser

	hubNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]
	osmoNeutronAccount := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), neutron)[0]

	testCtx.skipBlocks(10)

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

	t.Run("tokenswap covenant setup", func(t *testing.T) {
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
		var covenantCodeId uint64

		t.Run("deploy covenant contracts", func(t *testing.T) {
			covenantCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, covenantContractPath)
			clockCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, clockContractPath)
			routerCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, routerContractPath)
			splitterCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, splitterContractPath)
			ibcForwarderCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, ibcForwarderContractPath)
			swapHolderCodeId = testCtx.storeContract(cosmosNeutron, neutronUser, swapHolderContractPath)
		})

		t.Run("instantiate covenant", func(t *testing.T) {
			timeouts := Timeouts{
				IcaTimeout:         "100", // sec
				IbcTransferTimeout: "100", // sec
			}

			swapCovenantTerms := SwapCovenantTerms{
				PartyAAmount: strconv.FormatUint(atomContributionAmount, 10),
				PartyBAmount: strconv.FormatUint(osmoContributionAmount, 10),
			}

			currentHeight, err := cosmosNeutron.Height(ctx)
			require.NoError(t, err, "failed to get neutron height")
			depositBlock := Block(currentHeight + 150)
			lockupConfig := Expiration{
				AtHeight: &depositBlock,
			}

			presetIbcFee := PresetIbcFee{
				AckFee:     "10000",
				TimeoutFee: "10000",
			}
			hubReceiverAddr := gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix)
			osmoReceiverAddr := osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix)

			splits := []DenomSplit{
				{
					Denom: neutronOsmoIbcDenom,
					Type: SplitType{
						Custom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:  "1.0",
								osmoReceiverAddr: "0.0",
							},
						},
					},
				},
				{
					Denom: neutronAtomIbcDenom,
					Type: SplitType{
						Custom: SplitConfig{
							Receivers: map[string]string{
								hubReceiverAddr:  "0.0",
								osmoReceiverAddr: "1.0",
							},
						},
					},
				},
			}

			partyAConfig := SwapPartyConfig{
				Addr:                      hubNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
				NativeDenom:               nativeAtomDenom,
				IbcDenom:                  neutronAtomIbcDenom,
				PartyToHostChainChannelId: testCtx.GaiaTransferChannelIds[cosmosNeutron.Config().Name],
				HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosAtom.Config().Name],
				PartyReceiverAddr:         hubReceiverAddr,
				PartyChainConnectionId:    neutronAtomIBCConnId,
				IbcTransferTimeout:        timeouts.IbcTransferTimeout,
			}
			partyBConfig := SwapPartyConfig{
				Addr:                      osmoNeutronAccount.Bech32Address(cosmosNeutron.Config().Bech32Prefix),
				NativeDenom:               nativeOsmoDenom,
				IbcDenom:                  neutronOsmoIbcDenom,
				PartyToHostChainChannelId: testCtx.OsmoTransferChannelIds[cosmosNeutron.Config().Name],
				HostToPartyChainChannelId: testCtx.NeutronTransferChannelIds[cosmosOsmosis.Config().Name],
				PartyReceiverAddr:         osmoReceiverAddr,
				PartyChainConnectionId:    neutronOsmosisIBCConnId,
				IbcTransferTimeout:        timeouts.IbcTransferTimeout,
			}
			codeIds := SwapCovenantContractCodeIds{
				IbcForwarderCode:       ibcForwarderCodeId,
				InterchainRouterCode:   routerCodeId,
				InterchainSplitterCode: splitterCodeId,
				ClockCode:              clockCodeId,
				HolderCode:             swapHolderCodeId,
			}

			covenantMsg := CovenantInstantiateMsg{
				Label:                       "swap-covenant",
				Timeouts:                    timeouts,
				PresetIbcFee:                presetIbcFee,
				SwapCovenantContractCodeIds: codeIds,
				LockupConfig:                lockupConfig,
				SwapCovenantTerms:           swapCovenantTerms,
				PartyAConfig:                partyAConfig,
				PartyBConfig:                partyBConfig,
				Splits:                      splits,
			}

			covenantAddress = testCtx.manualInstantiate(strconv.FormatUint(covenantCodeId, 10), covenantMsg, neutronUser, keyring.BackendTest)
			println("covenant address: ", covenantAddress)
		})

		t.Run("query covenant contracts", func(t *testing.T) {
			clockAddress = testCtx.queryClockAddress(covenantAddress)
			holderAddress = testCtx.queryHolderAddress(covenantAddress)
			splitterAddress = testCtx.queryInterchainSplitterAddress(covenantAddress)
			partyARouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_a")
			partyBRouterAddress = testCtx.queryInterchainRouterAddress(covenantAddress, "party_b")
			partyAIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_a")
			partyBIbcForwarderAddress = testCtx.queryIbcForwarderAddress(covenantAddress, "party_b")
		})

		t.Run("fund contracts with neutron", func(t *testing.T) {
			addrs := []string{
				partyAIbcForwarderAddress,
				partyBIbcForwarderAddress,
				clockAddress,
				partyARouterAddress,
				partyBRouterAddress,
			}
			testCtx.fundChainAddrs(addrs, cosmosNeutron, neutronUser, 5000000000)
			testCtx.skipBlocks(2)
		})
	})

	t.Run("tokenswap run", func(t *testing.T) {

		t.Run("tick until forwarders create ICA", func(t *testing.T) {
			for {
				testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				forwarderAState := testCtx.queryContractState(partyAIbcForwarderAddress)
				forwarderBState := testCtx.queryContractState(partyBIbcForwarderAddress)

				if forwarderAState == forwarderBState && forwarderBState == "ica_created" {
					partyADepositAddress = testCtx.queryDepositAddress(partyAIbcForwarderAddress)
					partyBDepositAddress = testCtx.queryDepositAddress(partyBIbcForwarderAddress)
					break
				}
			}
		})

		t.Run("fund the forwarders with sufficient funds", func(t *testing.T) {
			require.NoError(t,
				cosmosOsmosis.SendFunds(ctx, osmoUser.KeyName, ibc.WalletAmount{
					Address: partyBDepositAddress,
					Denom:   nativeOsmoDenom,
					Amount:  int64(osmoContributionAmount + 1000),
				}),
				"failed to fund osmo forwarder",
			)

			require.NoError(t,
				cosmosAtom.SendFunds(ctx, gaiaUser.KeyName, ibc.WalletAmount{
					Address: partyADepositAddress,
					Denom:   nativeAtomDenom,
					Amount:  int64(atomContributionAmount + 1000),
				}),
				"failed to fund gaia forwarder",
			)
			testCtx.skipBlocks(5)
		})

		t.Run("tick until forwarders forward the funds to holder", func(t *testing.T) {
			for {
				holderOsmoBal := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, holderAddress)
				holderAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, holderAddress)
				holderState := testCtx.queryContractState(holderAddress)

				if holderAtomBal != 0 && holderOsmoBal != 0 || holderState == "complete" {
					println("holder atom bal: ", holderAtomBal)
					println("holder osmo bal: ", holderOsmoBal)
					break
				} else {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until holder sends the funds to splitter", func(t *testing.T) {
			for {
				splitterOsmoBal := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, splitterAddress)
				splitterAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, splitterAddress)

				if splitterAtomBal != 0 && splitterOsmoBal != 0 {
					println("splitterOsmoBal: ", splitterOsmoBal)
					println("splitterAtomBal: ", splitterAtomBal)
					break
				} else {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until splitter sends the funds to routers", func(t *testing.T) {
			for {
				partyARouterOsmoBal := testCtx.queryNeutronDenomBalance(neutronOsmoIbcDenom, partyARouterAddress)
				partyBRouterAtomBal := testCtx.queryNeutronDenomBalance(neutronAtomIbcDenom, partyBRouterAddress)

				if partyARouterOsmoBal != 0 && partyBRouterAtomBal != 0 {
					println("partyARouter osmo bal: ", partyARouterOsmoBal)
					println("partyBRouterAtomBal: ", partyBRouterAtomBal)
					break
				} else {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})

		t.Run("tick until routers route the funds to final receivers", func(t *testing.T) {
			for {
				osmoBal, err := cosmosOsmosis.GetBalance(ctx, osmoUser.Bech32Address(cosmosOsmosis.Config().Bech32Prefix), osmoNeutronAtomIbcDenom)
				require.NoError(t, err, "failed to query osmoBal")
				gaiaBal, err := cosmosAtom.GetBalance(ctx, gaiaUser.Bech32Address(cosmosAtom.Config().Bech32Prefix), gaiaNeutronOsmoIbcDenom)
				require.NoError(t, err, "failed to query gaiaBal")

				if osmoBal != 0 && gaiaBal != 0 {
					println("gaia user osmo bal: ", gaiaBal)
					println("osmo user atom bal: ", osmoBal)
					break
				} else {
					testCtx.tick(clockAddress, keyring.BackendTest, neutronUser.KeyName)
				}
			}
		})
	})
}
