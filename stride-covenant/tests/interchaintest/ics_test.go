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
	transfertypes "github.com/cosmos/ibc-go/v3/modules/apps/transfer/types"
	ibctest "github.com/strangelove-ventures/interchaintest/v3"
	"github.com/strangelove-ventures/interchaintest/v3/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v3/ibc"

	"github.com/strangelove-ventures/interchaintest/v3/relayer"
	"github.com/strangelove-ventures/interchaintest/v3/relayer/rly"
	"github.com/strangelove-ventures/interchaintest/v3/testreporter"
	"github.com/strangelove-ventures/interchaintest/v3/testutil"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap"
	"go.uber.org/zap/zaptest"
)

// This tests Cosmos Interchain Security, spinning up gaia, neutron, and stride
func TestICS(t *testing.T) {
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
			ChainConfig: ibc.ChainConfig{
				Type:    "cosmos",
				Name:    "stride",
				ChainID: "stride-3",
				Images: []ibc.DockerImage{
					{
						// Repository: "ghcr.io/strangelove-ventures/heighliner/stride",
						// Version:    "v9.2.1",
						Repository: "stride",
						Version:    "local",
						UidGid:     "1025:1025",
					},
				},
				Bin:            "strided",
				Bech32Prefix:   "stride",
				Denom:          "ustrd",
				GasPrices:      "0.00ustrd",
				GasAdjustment:  1.3,
				TrustingPeriod: "1197504s",
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
		zaptest.NewLogger(t),
		relayer.CustomDockerImage("ghcr.io/cosmos/relayer", "v2.3.1", rly.RlyDefaultUidGid),
		relayer.RelayerOptionExtraStartFlags{Flags: []string{"-d", "--log-format", "console"}},
	).Build(t, client, network)

	// Prep Interchain
	const gaiaNeutronICSPath = "gn-ics-path"
	const gaiaNeutronIBCPath = "gn-ibc-path"
	const gaiaStrideIBCPath = "gs-ibc-path"
	const neutronStrideIBCPath = "ns-ibc-path"

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
			Chain1:  cosmosAtom,
			Chain2:  cosmosStride,
			Relayer: r,
			Path:    gaiaStrideIBCPath,
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  cosmosNeutron,
			Chain2:  cosmosStride,
			Relayer: r,
			Path:    neutronStrideIBCPath,
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

		SkipPathCreation: false,
	})
	require.NoError(t, err, "failed to build interchain")

	err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")

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

	cmd := getCreateValidatorCmd(atom)
	_, _, err = atom.Exec(ctx, cmd, nil)
	require.NoError(t, err)

	// Wait a bit for the VSC packet to get relayed.
	err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
	require.NoError(t, err, "failed to wait for blocks")

	// Once the VSC packet has been relayed, x/bank transfers are
	// enabled on Neutron and we can fund its account.
	// The funds for this are sent from a "faucet" account created
	// by interchaintest in the genesis file.
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(500_000_000_000), atom, neutron, stride)
	gaiaUser, neutronUser, strideUser := users[0], users[1], users[2]

	strideAdminMnemonic := "tone cause tribe this switch near host damage idle fragile antique tail soda alien depth write wool they rapid unfold body scan pledge soft"
	strideAdmin, _ := ibctest.GetAndFundTestUserWithMnemonic(ctx, "default", strideAdminMnemonic, (100_000_000), cosmosStride)

	cosmosStride.SendFunds(ctx, strideUser.KeyName, ibc.WalletAmount{
		Address: strideAdmin.Bech32Address(stride.Config().Bech32Prefix),
		Denom:   "ustride",
		Amount:  10000000,
	})

	err = testutil.WaitForBlocks(ctx, 30, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")

	neutronUserBal, err := neutron.GetBalance(
		ctx,
		neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
		neutron.Config().Denom)
	require.NoError(t, err, "failed to fund neutron user")
	require.EqualValues(t, int64(500_000_000_000), neutronUserBal)

	var strideGaiaConnectionId, gaiaStrideConnectionId string
	var strideNeutronConnectionId, neutronStrideConnectionId string
	var neutronGaiaTransferConnectionId, neutronGaiaICSConnectionId string
	var gaiaNeutronTransferConnectionId, gaiaNeutronICSConnectionId string
	var liquidityTokenAddress string

	neutronGaiaConnectionIds := make([]string, 0)
	gaiaNeutronConnectionIds := make([]string, 0)
	neutronStrideConnectionIds := make([]string, 0)
	strideNeutronConnectionIds := make([]string, 0)
	gaiaStrideConnectionIds := make([]string, 0)
	strideGaiaConnectionIds := make([]string, 0)

	var strideNeutronChannelId, neutronStrideChannelId string
	var strideGaiaChannelId, gaiaStrideChannelId string
	var neutronGaiaICSChannelId, gaiaNeutronICSChannelId string
	var neutronGaiaTransferChannelId, gaiaNeutronTransferChannelId string

	// We attempt to find all channels and connections.
	// They take variable time to build. So we attempt finding them
	// a few times

	connectionChannelsOk := false
	const maxAttempts = 3
	attempts := 1
	for (connectionChannelsOk != true) && (attempts <= maxAttempts) {
		print("\n Finding connections and channels, attempt ", attempts, " of ", maxAttempts)
		neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)
		strideChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosStride.Config().ChainID)
		strideConnectionInfo, _ := r.GetConnections(ctx, eRep, cosmosStride.Config().ChainID)
		neutronConnectionInfo, _ := r.GetConnections(ctx, eRep, cosmosNeutron.Config().ChainID)
		gaiaConnectionInfo, _ := r.GetConnections(ctx, eRep, cosmosAtom.Config().ChainID)

		connectionChannelsOk = true
		/// Find all the pairwise connections
		strideNeutronConnectionIds, neutronStrideConnectionIds, err = getPairwiseConnectionIds(strideConnectionInfo, neutronConnectionInfo)
		if err != nil {
			connectionChannelsOk = false
		}
		neutronGaiaConnectionIds, gaiaNeutronConnectionIds, err = getPairwiseConnectionIds(neutronConnectionInfo, gaiaConnectionInfo)
		if err != nil {
			connectionChannelsOk = false
		}
		strideGaiaConnectionIds, gaiaStrideConnectionIds, err = getPairwiseConnectionIds(strideConnectionInfo, gaiaConnectionInfo)
		if err != nil {
			connectionChannelsOk = false
		}
		// Find all pairwise channels
		strideNeutronChannelId, neutronStrideChannelId, strideNeutronConnectionId, neutronStrideConnectionId, err = getPairwiseTransferChannelIds(strideChannelInfo, neutronChannelInfo, strideNeutronConnectionIds, neutronStrideConnectionIds)
		if err != nil {
			connectionChannelsOk = false
		}
		strideGaiaChannelId, gaiaStrideChannelId, strideGaiaConnectionId, gaiaStrideConnectionId, err = getPairwiseTransferChannelIds(strideChannelInfo, gaiaChannelInfo, strideGaiaConnectionIds, gaiaStrideConnectionIds)
		if err != nil {
			connectionChannelsOk = false
		}
		gaiaNeutronTransferChannelId, neutronGaiaTransferChannelId, gaiaNeutronTransferConnectionId, neutronGaiaTransferConnectionId, err = getPairwiseTransferChannelIds(gaiaChannelInfo, neutronChannelInfo, gaiaNeutronConnectionIds, neutronGaiaConnectionIds)
		if err != nil {
			connectionChannelsOk = false
		}
		gaiaNeutronICSChannelId, neutronGaiaICSChannelId, gaiaNeutronICSConnectionId, neutronGaiaICSConnectionId, err = getPairwiseCCVChannelIds(gaiaChannelInfo, neutronChannelInfo, gaiaNeutronConnectionIds, neutronGaiaConnectionIds)
		if err != nil {
			connectionChannelsOk = false
		}
		// Print out connections and channels for debugging
		print("\n strideGaiaConnectionId: ", strideGaiaConnectionId)
		print("\n strideNeutronConnectionId: ", strideNeutronConnectionId)
		print("\n neutronStrideConnectionId: ", neutronStrideConnectionId)
		print("\n neutronGaiaTransferConnectionId: ", neutronGaiaTransferConnectionId)
		print("\n neutronGaiaICSConnectionId: ", neutronGaiaICSConnectionId)
		print("\n gaiaStrideConnectionId: ", gaiaStrideConnectionId)
		print("\n gaiaNeutronTransferConnectionId: ", gaiaNeutronTransferConnectionId)
		print("\n gaiaNeutronICSConnectionId: ", gaiaNeutronICSConnectionId)
		print("\n strideGaiaChannelId: ", strideGaiaChannelId)
		print("\n strideNeutronChannelId: ", strideNeutronChannelId)
		print("\n neutronStrideChannelId: ", neutronStrideChannelId)
		print("\n neutronGaiaTransferChannelId: ", neutronGaiaTransferChannelId)
		print("\n neutronGaiaICSChannelId: ", neutronGaiaICSChannelId)
		print("\n gaiaStrideChannelId: ", gaiaStrideChannelId)
		print("\n gaiaNeutronTransferChannelId: ", gaiaNeutronTransferChannelId)
		print("\n gaiaNeutronICSChannelId: ", gaiaNeutronICSChannelId)

		if connectionChannelsOk {
			print("\n Connections and channels found!")

		} else {
			if attempts == maxAttempts {
				panic("Initial connections and channels did not build")
			}
			print("\n Connections and channels not found! Waiting some time...")
			err = testutil.WaitForBlocks(ctx, 100, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")
			attempts += 1
		}
	}
	_, _, _, _, _ = neutronGaiaTransferChannelId, gaiaNeutronTransferChannelId, neutronGaiaICSChannelId, gaiaNeutronICSChannelId, neutronStrideChannelId
	_, _, _ = gaiaStrideConnectionId, strideGaiaConnectionId, strideNeutronConnectionId

	// We can determine the ibc denoms of:
	// 1. ATOM on Neutron
	neutronSrcDenomTrace := transfertypes.ParseDenomTrace(
		transfertypes.GetPrefixedDenom("transfer",
			neutronGaiaTransferChannelId,
			atom.Config().Denom))
	neutronAtomIbcDenom := neutronSrcDenomTrace.IBCDenom()
	// 2. ATOM on Stride
	atomSrcDenomTrace := transfertypes.ParseDenomTrace(
		transfertypes.GetPrefixedDenom("transfer",
			strideGaiaChannelId,
			atom.Config().Denom))
	strideAtomIbcDenom := atomSrcDenomTrace.IBCDenom()
	// 3. stATOM on Neutron
	neutronStatomDenomTrace := transfertypes.ParseDenomTrace(
		transfertypes.GetPrefixedDenom("transfer",
			neutronStrideChannelId,
			"stuatom"))
	neutronStatomDenom := neutronStatomDenomTrace.IBCDenom()
	// Print these out to the log
	print("\nneutronAtomIbcDenom: ", neutronAtomIbcDenom)
	print("\nstrideAtomIbcDenom: ", strideAtomIbcDenom)
	print("\nneutronStatomDenom: ", neutronStatomDenom)
	print("\n")
	_ = strideAtomIbcDenom

	t.Run("stride covenant tests", func(t *testing.T) {
		//----------------------------------------------//
		// Testing parameters
		//----------------------------------------------//
		const atomFundsToDepositor uint64 = 100_000_000_000   // in uatom
		const atomToLiquidStake uint64 = 50_000_000_000       // in stuatom
		atomFunds := atomFundsToDepositor - atomToLiquidStake // in uatom

		const strideRedemptionRate uint64 = 1
		//----------------------------------------------//

		// Wasm code that we need to store on Neutron
		const covenantContractPath = "wasms/covenant_covenant.wasm"
		const clockContractPath = "wasms/covenant_clock.wasm"
		const holderContractPath = "wasms/covenant_holder.wasm"
		const depositorContractPath = "wasms/covenant_depositor.wasm"
		const lsContractPath = "wasms/covenant_ls.wasm"
		const lperContractPath = "wasms/covenant_lp.wasm"

		// After storing on Neutron, we will receive a code id
		// We parse all the subcontracts into uint64
		// The will be required when we instantiate the covenant.
		var clockCodeId uint64
		var depositorCodeId uint64
		var lsCodeId uint64
		var lperCodeId uint64
		var holderCodeId uint64
		// We won't need to parse the covenant code into a uint64
		// We store it simply as a string.
		var covenantCodeIdStr string

		// Contract addresses after instantiation
		var covenantContractAddress string
		var clockContractAddress string
		var holderContractAddress string
		var lperContractAddress string
		var depositorContractAddress string
		var lsContractAddress string

		// Instantiation parameters for the depositor
		var stAtomWeightedReceiverAmount WeightedReceiverAmount
		var atomWeightedReceiverAmount WeightedReceiverAmount
		var strideICAAddress string

		const icaAccountId = "test"
		var icaAccountAddress string

		// Addresses for Astroport contract deployments
		var coinRegistryAddress string
		var factoryAddress string
		var stableswapAddress string
		var tokenAddress string
		var whitelistAddress string
		_, _ = tokenAddress, whitelistAddress

		////////////// State machines ///////////////
		// Depositor states
		const depositorStateInstantiated = "instantiated"
		const depositorStateIcaCreated = "i_c_a_created"
		const depositorStateComplete = "completed"
		// LS states
		const lsStateInstantiated = "instantiated"
		const lsStateIcaCreated = "i_c_a_created"
		// LP states
		const lpStateInstantiated = "instantiated"

		var currentDepositorState string
		var currentLsState string
		var currentLpState string

		t.Run("deploy covenant contracts", func(t *testing.T) {
			// store covenant and get code id
			covenantCodeIdStr, err = cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, covenantContractPath)
			require.NoError(t, err, "failed to store stride covenant contract")

			// store clock and get code id
			clockCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, clockContractPath)
			require.NoError(t, err, "failed to store clock contract")
			clockCodeId, err = strconv.ParseUint(clockCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store holder and get code id
			holderCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, holderContractPath)
			require.NoError(t, err, "failed to store holder contract")
			holderCodeId, err = strconv.ParseUint(holderCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store depositor and get code id
			depositorCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, depositorContractPath)
			require.NoError(t, err, "failed to store depositor contract")
			depositorCodeId, err = strconv.ParseUint(depositorCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store ls and get code id
			lsCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, lsContractPath)
			require.NoError(t, err, "failed to store ls contract")
			lsCodeId, err = strconv.ParseUint(lsCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			// store lper and get code id
			lperCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, lperContractPath)
			require.NoError(t, err, "failed to store lper contract")
			lperCodeId, err = strconv.ParseUint(lperCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")
		})

		// Stride is a liquid staking platform. We need to register Gaia (ATOM)
		// as a host zone in order to redeem stATOM in exchange for ATOM
		// stATOM is stride's liquid staked ATOM vouchers.
		t.Run("register stride host zone", func(t *testing.T) {

			cmd := []string{"strided", "tx", "stakeibc", "register-host-zone",
				strideGaiaConnectionId,
				cosmosAtom.Config().Denom,
				cosmosAtom.Config().Bech32Prefix,
				strideAtomIbcDenom,
				strideGaiaChannelId,
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

			err = testutil.WaitForBlocks(ctx, 5, stride)
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
					InitialBalances: []Cw20Coin{
						// Cw20Coin{
						// 	Address: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
						// 	Amount:  1,
						// },
					},
					// Mint: &MinterResponse{
					// 	Minter: depositorContractAddress,
					// 	Cap:    &cap,
					// },
					Mint:      nil,
					Marketing: nil,
				}

				str, err := json.Marshal(msg)
				require.NoError(t, err, "Failed to marshall NativeTokenInstantiateMsg")

				tokenAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, tokenCodeIdStr, string(str), true)
				require.NoError(t, err, "Failed to instantiate Native Token")
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("add coins to registry", func(t *testing.T) {
				// Add ibc native tokens for stuatom and uatom to the native coin registry
				// each of these tokens has a precision of 6
				addMessage := `{"add":{"native_coins":[["` + neutronStatomDenom + `",6],["` + neutronAtomIbcDenom + `",6]]}}`
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
				require.NoError(t, err, "failed to wait for blocks")
			})

			t.Run("create pair on factory", func(t *testing.T) {

				initParams := StablePoolParams{
					Amp: 3,
				}
				binaryData, err := json.Marshal(initParams)
				require.NoError(t, err, "error encoding stable pool params to binary")

				stAtom := NativeToken{
					Denom: neutronStatomDenom,
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
				print("\ncreate pair msg: ", string(str))

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

				print("\ncreate pair cmd: ", string(strings.Join(createCmd, " ")))
				_, _, err = cosmosNeutron.Exec(ctx, createCmd, nil)
				require.NoError(t, err, err)
				err = testutil.WaitForBlocks(ctx, 30, atom, neutron)
				require.NoError(t, err, "failed to wait for blocks")
			})

			// 	t.Run("stableswap", func(t *testing.T) {
			// 		// We can see the actual stuatom and uatom stable swap pool
			// 		// https://www.mintscan.io/neutron/wasm/contract/neutron1gexnrw67sqje6y8taeehlmrl5nyw0tn9vtpq6tgxs62upsjhql5q2glanc
			// 		// int amp is set to 3, with precision 100.
			// 		initParams := StablePoolParams{
			// 			Amp:   3,
			// 			Owner: nil,
			// 		}
			// 		binaryData, err := json.Marshal(initParams)
			// 		require.NoError(t, err, "error encoding stable pool params to binary")

			// 		stAtom := NativeToken{
			// 			Denom: neutronStatomDenom,
			// 		}
			// 		nativeAtom := NativeToken{
			// 			Denom: neutronAtomIbcDenom,
			// 		}
			// 		assetInfos := []AssetInfo{
			// 			{
			// 				NativeToken: &stAtom,
			// 			},
			// 			{
			// 				NativeToken: &nativeAtom,
			// 			},
			// 		}

			// 		msg := StableswapInstantiateMsg{
			// 			TokenCodeId: tokenCodeId,
			// 			FactoryAddr: factoryAddress,
			// 			AssetInfos:  assetInfos,
			// 			InitParams:  binaryData,
			// 		}

			// 		str, err := json.Marshal(msg)
			// 		require.NoError(t, err, "Failed to marshall DepositorInstantiateMsg")

			// 		stableswapAddr, err := cosmosNeutron.InstantiateContract(
			// 			ctx, neutronUser.KeyName, stablePairCodeIdStr, string(str), true,
			// 			"--label", "stableswap",
			// 			"--gas-prices", "0.0untrn",
			// 			"--gas-adjustment", `1.5`,
			// 			"--output", "json",
			// 			"--node", neutron.GetRPCAddress(),
			// 			"--home", neutron.HomeDir(),
			// 			"--chain-id", neutron.Config().ChainID,
			// 			"--gas", "auto",
			// 			"--keyring-backend", keyring.BackendTest,
			// 			"-y",
			// 		)
			// 		require.NoError(t, err, "Failed to instantiate stableswap")
			// 		stableswapAddress = stableswapAddr
			// 		err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
			// 		require.NoError(t, err, "failed to wait for blocks")
			// 	})

		})

		t.Run("add liquidity to the atom-statom stableswap pool", func(t *testing.T) {
			// query neutronUser balance of lp tokens

			stAtom := NativeToken{
				Denom: neutronStatomDenom,
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
			_, err := atom.SendIBCTransfer(ctx, gaiaNeutronTransferChannelId, gaiaUser.KeyName, transferNeutron, ibc.TransferOptions{})
			require.NoError(t, err)

			testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)

			// send 100K atom to stride which we can liquid stake
			autopilotString := `{"autopilot":{"receiver":"` + strideUser.Bech32Address(stride.Config().Bech32Prefix) + `","stakeibc":{"stride_address":"` + strideUser.Bech32Address(stride.Config().Bech32Prefix) + `","action":"LiquidStake"}}}`
			cmd := []string{atom.Config().Bin, "tx", "ibc-transfer", "transfer", "transfer", gaiaStrideChannelId, autopilotString,
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
			_, err = stride.SendIBCTransfer(ctx, strideNeutronChannelId, strideUser.KeyName, transferStAtomNeutron, ibc.TransferOptions{})
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
							Denom: neutronStatomDenom,
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
			amountStr := "100000000000" + neutronAtomIbcDenom + "," + "100000000000" + neutronStatomDenom

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

		t.Run("instantiate covenant", func(t *testing.T) {
			// Clock instantiation message
			clockMsg := PresetClockFields{
				// TickMaxGas: "500000",
				ClockCode: clockCodeId,
				Label:     "covenant-clock",
				Whitelist: []string{},
			}
			// Depositor instantiation message
			// note that clock address needs to be filled
			stAtomWeightedReceiverAmount = WeightedReceiverAmount{
				Amount: atomToLiquidStake,
			}
			atomWeightedReceiverAmount = WeightedReceiverAmount{
				Amount: atomFunds,
			}
			depositorMsg := PresetDepositorFields{
				GaiaNeutronIBCTransferChannelId: gaiaNeutronTransferChannelId,
				NeutronGaiaConnectionId:         neutronGaiaTransferConnectionId,
				GaiaStrideIBCTransferChannelId:  gaiaStrideChannelId,
				DepositorCode:                   depositorCodeId,
				Label:                           "covenant-depositor",
				StAtomReceiverAmount:            stAtomWeightedReceiverAmount,
				AtomReceiverAmount:              atomWeightedReceiverAmount,
				AutopilotFormat:                 "{\"autopilot\": {\"receiver\": \"{st_ica}\",\"stakeibc\": {\"stride_address\": \"{st_ica}\",\"action\": \"LiquidStake\"}}}",
			}
			// LS instantiation message
			lsMsg := PresetLsFields{
				LsCode:                            lsCodeId,
				Label:                             "covenant-ls",
				LsDenom:                           "stuatom",
				StrideNeutronIBCTransferChannelId: strideNeutronChannelId,
				NeutronStrideIBCConnectionId:      neutronStrideConnectionId,
			}

			// For LPer, we need to first gather astroport information
			assets := AssetData{
				NativeAssetDenom: neutronAtomIbcDenom,
				LsAssetDenom:     neutronStatomDenom,
			}

			// slippageTolerance := "0.01"
			singleSideLpLimits := SingleSideLpLimits{
				NativeAssetLimit: "7204688721",
				LsAssetLimit:     "7204688721",
			}
			lpMsg := PresetLpFields{
				LpCode:             lperCodeId,
				Label:              "covenant-lp",
				Autostake:          false,
				Assets:             assets,
				SingleSideLpLimits: singleSideLpLimits,
			}

			holderMsg := PresetHolderFields{
				HolderCode: holderCodeId,
				Label:      "covenant-holder",
				Withdrawer: neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
			}
			// presetIbcFee := PresetIbcFee{
			// AckFee:     CwCoin{Amount: 1000, Denom: "untrn"},
			// TimeoutFee: CwCoin{Amount: 1000, Denom: "untrn"},
			// }

			covenantMsg := CovenantInstantiateMsg{
				Label:                          "stride-covenant",
				PresetClock:                    clockMsg,
				PresetLs:                       lsMsg,
				PresetDepositor:                depositorMsg,
				PresetLp:                       lpMsg,
				PresetHolder:                   holderMsg,
				PoolAddress:                    stableswapAddress,
				IbcMsgTransferTimeoutTimestamp: 10000000000,
				// PresetIbcFee:    presetIbcFee,
			}

			str, err := json.Marshal(covenantMsg)
			require.NoError(t, err, "Failed to marshall CovenantInstantiateMsg")

			covenantContractAddress, err = cosmosNeutron.InstantiateContract(
				ctx,
				neutronUser.KeyName,
				covenantCodeIdStr,
				string(str),
				true, "--gas", "1500000",
			)
			require.NoError(t, err, "failed to instantiate contract: ", err)
			print("\n covenant address: ", covenantContractAddress)
		})

		t.Run("query covenant instantiated contracts", func(t *testing.T) {
			var response CovenantAddressQueryResponse
			// Query clock
			err = cosmosNeutron.QueryContract(ctx, covenantContractAddress, ClockAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated clock address")
			clockContractAddress = response.Data
			print("\nclock addr: ", clockContractAddress)

			// Query depositor
			err = cosmosNeutron.QueryContract(ctx, covenantContractAddress, DepositorAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated depositor address")
			depositorContractAddress = response.Data
			print("\ndepositor addr: ", depositorContractAddress)

			// Query Lser
			err = cosmosNeutron.QueryContract(ctx, covenantContractAddress, LsAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated ls address")
			lsContractAddress = response.Data
			print("\nls addr: ", lsContractAddress)

			// Query Lper
			err = cosmosNeutron.QueryContract(ctx, covenantContractAddress, LpAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated lp address")
			lperContractAddress = response.Data
			print("\nlp addr: ", lperContractAddress)

			// Query Holder
			err = cosmosNeutron.QueryContract(ctx, covenantContractAddress, HolderAddressQuery{}, &response)
			require.NoError(t, err, "failed to query instantiated holder address")
			holderContractAddress = response.Data
			print("\nholder addr: ", holderContractAddress)
		})

		t.Run("fund contracts with neutron", func(t *testing.T) {
			err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: depositorContractAddress,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from neutron user to depositor contract")

			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: clockContractAddress,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})
			require.NoError(t, err, "failed to send funds from neutron user to clock contract")

			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: lsContractAddress,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})
			require.NoError(t, err, "failed to send funds from neutron user to ls contract")

			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: lperContractAddress,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})
			require.NoError(t, err, "failed to send funds from neutron user to lp contract")

			err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			depositorNeutronBal, err := neutron.GetBalance(ctx, depositorContractAddress, neutron.Config().Denom)
			require.NoError(t, err, "failed to get depositor neutron balance")
			require.EqualValues(t, 500001, depositorNeutronBal)

			lsNeutronBal, err := neutron.GetBalance(ctx, lsContractAddress, neutron.Config().Denom)
			require.NoError(t, err, "failed to get depositor neutron balance")
			require.EqualValues(t, 500001, lsNeutronBal)
		})

		tickClock := func() {
			cmd = []string{"neutrond", "tx", "wasm", "execute", clockContractAddress,
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
			resp, _, err := cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			jsonResp, _ := json.Marshal(resp)
			print("\ntick response: ", string(jsonResp), "\n")
		}

		// Tick the clock until the depositor has created i_c_a
		t.Run("tick clock until depositor and Ls create ICA", func(t *testing.T) {
			const maxTicks = 20
			tick := 1
			for tick <= maxTicks {
				print("\n Ticking clock ", tick, " of ", maxTicks)
				tickClock()

				var response ContractStateQueryResponse
				// Query depositor
				err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, ContractStateQuery{}, &response)
				require.NoError(t, err, "failed to query depositor state")
				currentDepositorState = response.Data
				print("\n depositor state: ", currentDepositorState)

				// Query Lser
				err = cosmosNeutron.QueryContract(ctx, lsContractAddress, ContractStateQuery{}, &response)
				require.NoError(t, err, "failed to query ls state")
				currentLsState = response.Data
				print("\n ls state: ", currentLsState)

				// Query Lper
				err = cosmosNeutron.QueryContract(ctx, lperContractAddress, ContractStateQuery{}, &response)
				require.NoError(t, err, "failed to query lp state")
				currentLpState = response.Data
				print("\n lp state: ", currentLpState)

				err = testutil.WaitForBlocks(ctx, 5, atom, neutron, stride)

				if currentDepositorState == depositorStateIcaCreated &&
					currentLsState == lsStateIcaCreated {
					break
				}
				require.NoError(t, err, "failed to wait for blocks")
				tick += 1
			}
			// fail if we haven't created the ICAs under max ticks
			require.LessOrEqual(t, tick, maxTicks)
		})

		t.Run("Query depositor ICA", func(t *testing.T) {
			// Give atom some time before querying
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
			var response QueryResponse
			err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, DepositorICAAddressQuery{}, &response)
			require.NoError(t, err, "failed to query ICA account address")
			require.NotEmpty(t, response.Data.InterchainAccountAddress)
			icaAccountAddress = response.Data.InterchainAccountAddress
			print("\ndepositor ICA instantiated with address ", icaAccountAddress, "\n")
		})

		t.Run("Query stride ICA", func(t *testing.T) {
			var response StrideIcaQueryResponse
			err = cosmosNeutron.QueryContract(ctx, lsContractAddress, LsIcaQuery{}, &response)
			require.NoError(t, err, "failed to query ICA account address")
			require.NotEmpty(t, response.Addr)
			strideICAAddress = response.Addr

			print("\nstride ICA instantiated with address ", strideICAAddress, "\n")
		})

		t.Run("multisig transfers atom to ICA account", func(t *testing.T) {
			// transfer funds from gaiaUser to the newly generated ICA account
			err := cosmosAtom.SendFunds(ctx, gaiaUser.KeyName, ibc.WalletAmount{
				Address: icaAccountAddress,
				Amount:  int64(atomFundsToDepositor),
				Denom:   atom.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from gaia to neutron ICA")
			err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, int64(atomFundsToDepositor), atomBal)
		})

		// Tick the clock until the LSer has received stATOM
		// and Lper has received ATOM
		t.Run("tick clock until LSer receives funds", func(t *testing.T) {
			depositorNeutronBal, err := neutron.GetBalance(ctx, depositorContractAddress, neutron.Config().Denom)
			require.NoError(t, err, "failed to get depositor neutron balance")
			print("\ndepositor neutron balance: ", depositorNeutronBal, "\n")

			lsNeutronBal, err := neutron.GetBalance(ctx, lsContractAddress, neutron.Config().Denom)
			require.NoError(t, err, "failed to get ls neutron balance")
			print("\nls neutron balance: ", lsNeutronBal, "\n")

			strideICABal, err := stride.GetBalance(ctx, strideICAAddress, "stuatom")
			require.NoError(t, err, "failed to query ICA balance")
			print("\n stride ica bal: ", strideICABal, "\n")

			lpAtomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronAtomIbcDenom)
			require.NoError(t, err, "failed to query ICA balance")
			print("\n lp atom bal: ", lpAtomBalance, "\n")

			gaiaIcaBalance, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to query ICA balance")
			print("\n gaia ica atom bal: ", gaiaIcaBalance, "\n")

			const maxTicks = 20
			tick := 1
			for tick <= maxTicks {

				print("\n Ticking clock ", tick, " of ", maxTicks)
				tickClock()
				err = testutil.WaitForBlocks(ctx, 5, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")

				strideICABal, err := stride.GetBalance(ctx, strideICAAddress, "stuatom")
				require.NoError(t, err, "failed to query ICA balance")
				print("\n stride ica bal: ", strideICABal, "\n")

				lpAtomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query ICA balance")
				print("\n lp atom bal: ", lpAtomBalance, "\n")

				gaiaIcaBalance, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
				require.NoError(t, err, "failed to query ICA balance")
				print("\n gaia ica atom bal: ", gaiaIcaBalance, "\n")

				queryCmd := []string{"neutrond", "query", "wasm", "contract-state", "smart",
					clockContractAddress, "'{\"queue\":{}}'",
				}

				clockQueueBytes, _, _ := neutron.Exec(ctx, queryCmd, nil)

				clockQueue, _ := json.Marshal(clockQueueBytes)
				print("\n clock queue bytes: ", string(clockQueueBytes), ", queue itself: ", (clockQueue), "\n")

				if strideICABal == int64(strideRedemptionRate*atomToLiquidStake) &&
					lpAtomBalance == int64(atomFunds) {
					break
				}
				err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
				tick += 1
			}

			// fail if we haven't transferred funds in under maxTicks
			require.LessOrEqual(t, tick, maxTicks)
			atomICABal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(0), atomICABal)
		})

		t.Run("permissionlessly forward funds from Stride to LPer", func(t *testing.T) {
			// Wait for a few blocks
			err = testutil.WaitForBlocks(ctx, 5, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")
			// Construct a transfer message
			msg := TransferExecutionMsg{
				Transfer: TransferAmount{
					Amount: strideRedemptionRate * atomToLiquidStake,
				},
			}
			str, err := json.Marshal(msg)
			require.NoError(t, err)

			// Anyone can call the tranfer function
			print("\n attempting to move funds by executing message: ", string(str))
			cmd = []string{"neutrond", "tx", "wasm", "execute", lsContractAddress,
				string(str),
				"--from", neutronUser.KeyName,
				"--gas-prices", "0.0untrn",
				"--gas-adjustment", `1.8`,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--chain-id", neutron.Config().ChainID,
				"--gas", "auto",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}
			_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
			require.NoError(t, err)

			strideICABal, err := stride.GetBalance(ctx, strideICAAddress, "stuatom")
			require.NoError(t, err, "failed to query ICA balance")
			print("\n stride ica bal: ", strideICABal, "\n")

			lpStatomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronStatomDenom)
			require.NoError(t, err, "failed to query ICA balance")
			print("\n lp statom bal: ", lpStatomBalance, "\n")

			require.Equal(t, int64(0), strideICABal)
			require.Equal(t, int64(strideRedemptionRate*atomToLiquidStake), lpStatomBalance)

		})

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

		// queryAllLpHolders := func(token string) {
		// 	allAccounts := AllAccounts{}

		// 	accountsQueryMsg := Cw20QueryMsg{
		// 		AllAccounts: &allAccounts,
		// 	}
		// 	var response AllAccountsResponse
		// 	err = cosmosNeutron.QueryContract(ctx, token, accountsQueryMsg, &response)
		// 	require.NoError(t, err, "failed to query all accounts")
		// 	jsonResp, _ := json.Marshal(response)
		// 	print("\n all accounts: ", string(jsonResp), "\n")
		// }

		// TEST: LPer provides liquidity, tick the LPer
		// Check if LP tokens arrive in the Holder

		t.Run("LPer provides liqudity when ticked", func(t *testing.T) {
			const maxTicks = 20
			tick := 1
			for tick <= maxTicks {
				print("\n Ticking clock ", tick, " of ", maxTicks)
				tickClock()
				lpTokenBal := queryLpTokenBalance(liquidityTokenAddress, neutronUser.Bech32Address(neutron.Config().Bech32Prefix))
				print("\n lp token balance: ", lpTokenBal, "\n")
				lpAtomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronAtomIbcDenom)
				require.NoError(t, err, "failed to query ICA balance")
				print("\n lp atom bal: ", lpAtomBalance, "\n")

				lpStatomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronStatomDenom)
				require.NoError(t, err, "failed to query ICA balance")
				print("\n lp statom bal: ", lpStatomBalance, "\n")

				// holderbalance, err := cosmosNeutron.GetBalance()

				/*
						let holder_balances = suite.query_cw20_bal(
							pairinfo.liquidity_token.to_string(),
							suite.holder_addr.to_string(),
						);

					    pub fn query_cw20_bal(&self, token: String, addr: String) -> cw20::BalanceResponse {
							self.app
								.wrap()
								.query_wasm_smart(token, &cw20::Cw20QueryMsg::Balance { address: addr })
								.unwrap()
						}
				*/
				// cosmosNeutron.QueryContract(ctx, liquiditytokenaddr, Cw20QueryMsg)
				if lpAtomBalance == int64(0) &&
					lpStatomBalance == int64(0) {
					break
				}
				err = testutil.WaitForBlocks(ctx, 5, neutron)
				require.NoError(t, err, "failed to wait for blocks")
				tick += 1
				// queryAllLpHolders(liquidityTokenAddress)
			}
			// fail if we haven't transferred funds in under maxTicks
			require.LessOrEqual(t, tick, maxTicks)
			// TODO check if they are in holder

		})

		// TEST: Withdraw liquidity
		// Check if LP tokens are burned
		// Check if stATOM and ATOMs are returned

		t.Run("withdraw can withdraw liquidity", func(t *testing.T) {
			lpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderContractAddress)
			print("\n holder lp token bal: ", lpTokenBal, "\n")

			err = testutil.WaitForBlocks(ctx, 5, atom, neutron, stride)
			require.NoError(t, err)

			withdrawLiquidityMsg := WithdrawLiquidityMessage{
				WithdrawLiquidity: WithdrawLiquidity{},
			}
			str, _ := json.Marshal(withdrawLiquidityMsg)

			cmd = []string{"neutrond", "tx", "wasm", "execute", holderContractAddress,
				string(str),
				"--from", neutronUser.KeyName,
				"--gas-prices", "0.0untrn",
				"--gas-adjustment", `1.8`,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--chain-id", neutron.Config().ChainID,
				"--gas", "auto",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}
			_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
			require.NoError(t, err)

			holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderContractAddress)
			print("\n holder lp token bal: ", holderLpTokenBal, "\n")

			lpAtomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronAtomIbcDenom)
			require.NoError(t, err, "failed to query ICA balance")
			print("\n lp atom bal: ", lpAtomBalance, "\n")

			lpStatomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronStatomDenom)
			require.NoError(t, err, "failed to query ICA balance")
			print("\n lp statom bal: ", lpStatomBalance, "\n")
		})

		// TEST: Withdraw funds
		// Check if withdrawer balance increases

		t.Run("withdrawer can withdraw funds", func(t *testing.T) {
			holderLpTokenBal := queryLpTokenBalance(liquidityTokenAddress, holderContractAddress)
			print("\n holder lp token bal: ", holderLpTokenBal, "\n")

			withdrawMsg := WithdrawMessage{
				Withdraw: Withdraw{},
			}
			str, _ := json.Marshal(withdrawMsg)
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
			require.NoError(t, err)
			cmd = []string{"neutrond", "tx", "wasm", "execute", holderContractAddress,
				string(str),
				"--from", neutronUser.KeyName,
				"--gas-prices", "0.0untrn",
				"--gas-adjustment", `1.8`,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--chain-id", neutron.Config().ChainID,
				"--gas", "auto",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}
			_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			// queryAllLpHolders(liquidityTokenAddress)

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
			require.NoError(t, err)
			holderLpTokenBal = queryLpTokenBalance(liquidityTokenAddress, holderContractAddress)
			print("\n holder lp token bal: ", holderLpTokenBal, "\n")

			lpAtomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronAtomIbcDenom)
			require.NoError(t, err, "failed to query ICA balance")
			print("\n lp atom bal: ", lpAtomBalance, "\n")

			lpStatomBalance, err := neutron.GetBalance(ctx, lperContractAddress, neutronStatomDenom)
			require.NoError(t, err, "failed to query ICA balance")
			print("\n lp statom bal: ", lpStatomBalance, "\n")

			holderAtomBalance, err := neutron.GetBalance(ctx, holderContractAddress, neutronAtomIbcDenom)
			require.NoError(t, err, "failed to query holder balance")
			print("\n holder atom bal: ", holderAtomBalance, "\n")

			holderStatomBalance, err := neutron.GetBalance(ctx, holderContractAddress, neutronStatomDenom)
			require.NoError(t, err, "failed to query holder balance")
			print("\n holder statom bal: ", holderStatomBalance, "\n")
		})

		/////////////////////////////////////////////////////////////////////////////////////

		// t.Run("instantiate lper contract", func(t *testing.T) {
		// 	_, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/covenant_lp.wasm")
		// 	require.NoError(t, err, "failed to store neutron ICA contract")
		// 	// lpInfo := LpInfo{
		// 	// 	Addr: stableswapAddress,
		// 	// }
		// 	// assets := []AstroportAsset{
		// 	// 	AstroportAsset{
		// 	// 		Info: AssetInfo{
		// 	// 			NativeToken: &NativeToken{
		// 	// 				Denom: neutronAtomIbcDenom,
		// 	// 			},
		// 	// 		},
		// 	// 		Amount: 10,
		// 	// 	},
		// 	// 	AstroportAsset{
		// 	// 		Info: AssetInfo{
		// 	// 			NativeToken: &NativeToken{
		// 	// 				Denom: neutronStatomDenom,
		// 	// 			},
		// 	// 		},
		// 	// 		Amount: 10,
		// 	// 	},
		// 	// }
		// 	// slippageTolerance := 0.01

		// 	// lpMsg := LPerInstantiateMsg{
		// 	// 	LpPosition:        lpInfo,
		// 	// 	ClockAddress:      neutronUser.Bech32Address(neutron.Config().Bech32Prefix), // we need this to be able to tick the LPer manually
		// 	// 	HolderAddress:     holderContractAddress,
		// 	// 	SlippageTolerance: slippageTolerance,
		// 	// 	Autostake:         nil,
		// 	// 	Assets:            assets,
		// 	// }

		// 	// str, err := json.Marshal(lpMsg)
		// 	// require.NoError(t, err, "Failed to marshall LPerInstantiateMsg")
		// 	// print("\n lp instantiate msg: \n", string(str))
		// 	// lperContractAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
		// 	// require.NoError(t, err, "failed to instantiate lper contract: ", err)
		// 	// print("\n lper: ", lperContractAddress)

		// 	// t.Run("query instantiated clock", func(t *testing.T) {
		// 	// 	var response ClockQueryResponse
		// 	// 	err = cosmosNeutron.QueryContract(ctx, lperContractAddress, LPContractQuery{
		// 	// 		ClockAddress: ClockAddressQuery{},
		// 	// 	}, &response)
		// 	// 	require.NoError(t, err, "failed to query clock address")
		// 	// 	expectedAddrJson, _ := json.Marshal(clockContractAddress)
		// 	// 	require.Equal(t, string(expectedAddrJson), response.Data)
		// 	//})

		// 	t.Run("query lp position", func(t *testing.T) {
		// 		var response LpPositionQueryResponse
		// 		err := cosmosNeutron.QueryContract(ctx, lperContractAddress, LPPositionQuery{
		// 			LpPosition: LpPositionQuery{},
		// 		}, &response)
		// 		require.NoError(t, err, "failed to query lp position address")
		// 		require.Equal(t, stableswapAddress, response.Data.Addr)
		// 	})
		// })

		// t.Run("instantiate LS contract", func(t *testing.T) {
		// 	_, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/covenant_ls.wasm")
		// 	require.NoError(t, err, "failed to store neutron ICA contract")

		// 	// msg := LsInstantiateMsg{
		// 	// 	AutopilotPosition:                 "todo",
		// 	// 	ClockAddress:                      neutronUser.Bech32Address(neutron.Config().Bech32Prefix), // we need this to be able to tick the LPer manually
		// 	// 	StrideNeutronIBCTransferChannelId: strideNeutronChannelId,
		// 	// 	LpAddress:                         lperContractAddress,
		// 	// 	NeutronStrideIBCConnectionId:      neutronStrideConnectionId,
		// 	// 	LsDenom:                           "stuatom",
		// 	// }

		// 	// str, err := json.Marshal(msg)
		// 	// require.NoError(t, err, "Failed to marshall LsInstantiateMsg")

		// 	// lsContractAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
		// 	// require.NoError(t, err, "failed to instantiate ls contract: ", err)
		// 	// print("\nls contract:", lsContractAddress, "\n")
		// })

		// t.Run("create stride ICA", func(t *testing.T) {
		// 	// should remain constant
		// 	cmd = []string{"neutrond", "tx", "wasm", "execute", lsContractAddress,
		// 		`{"tick":{}}`,
		// 		"--from", neutronUser.KeyName,
		// 		"--gas-prices", "0.0untrn",
		// 		"--gas-adjustment", `1.5`,
		// 		"--output", "json",
		// 		"--home", "/var/cosmos-chain/neutron-2",
		// 		"--node", neutron.GetRPCAddress(),
		// 		"--home", neutron.HomeDir(),
		// 		"--chain-id", neutron.Config().ChainID,
		// 		"--from", neutronUser.KeyName,
		// 		"--gas", "auto",
		// 		"--keyring-backend", keyring.BackendTest,
		// 		"-y",
		// 	}

		// 	_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
		// 	require.NoError(t, err)

		// 	// err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
		// 	// require.NoError(t, err, "failed to wait for blocks")

		// 	// var response QueryResponse
		// 	// err = cosmosNeutron.QueryContract(ctx, lsContractAddress, IcaExampleContractQuery{
		// 	// 	InterchainAccountAddress: InterchainAccountAddressQuery{
		// 	// 		InterchainAccountId: "stride-ica",
		// 	// 		ConnectionId:        neutronStrideConnectionId,
		// 	// 	},
		// 	// }, &response)
		// 	// require.NoError(t, err, "failed to query ICA account address")
		// 	// require.NotEmpty(t, response.Data.InterchainAccountAddress)
		// 	// strideICAAddress = response.Data.InterchainAccountAddress

		// 	// print("\nstride ICA instantiated with address ", strideICAAddress, "\n")
		// })

		// t.Run("tick depositor to liquid stake on stride", func(t *testing.T) {
		// 	atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
		// 	require.NoError(t, err, "failed to get ICA balance")
		// 	require.EqualValues(t, 20, atomBal)

		// 	require.NoError(t, err, "failed to wait for blocks")
		// 	cmd = []string{"neutrond", "tx", "wasm", "execute", depositorContractAddress,
		// 		`{"tick":{}}`,
		// 		"--from", neutronUser.KeyName,
		// 		"--gas-adjustment", `1.3`,
		// 		"--gas-prices", "0.0untrn",
		// 		"--output", "json",
		// 		"--node", cosmosNeutron.GetRPCAddress(),
		// 		"--home", cosmosNeutron.HomeDir(),
		// 		"--chain-id", cosmosNeutron.Config().ChainID,
		// 		"--gas", "auto",
		// 		"--fees", "5000untrn",
		// 		"--keyring-backend", keyring.BackendTest,
		// 		"-y",
		// 	}

		// 	_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
		// 	require.NoError(t, err)

		// 	err = testutil.WaitForBlocks(ctx, 20, atom, neutron, stride)
		// 	require.NoError(t, err, "failed to wait for blocks")

		// 	atomICABal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
		// 	require.NoError(t, err, "failed to query ICA balance")
		// 	require.Equal(t, int64(10), atomICABal)

		// 	strideICABal, err := stride.GetBalance(ctx, strideICAAddress, "stuatom")
		// 	require.NoError(t, err, "failed to query ICA balance")
		// 	require.Equal(t, int64(10), strideICABal)
		// 	print("\n stride ica bal: ", strideICABal, "\n")
		// })

		// t.Run("tick ls to transfer stuatom to LPer", func(t *testing.T) {

		// 	cmd = []string{"neutrond", "tx", "wasm", "execute", lsContractAddress,
		// 		`{"tick":{}}`,
		// 		"--from", neutronUser.KeyName,
		// 		"--gas-adjustment", `1.3`,
		// 		"--output", "json",
		// 		"--node", cosmosNeutron.GetRPCAddress(),
		// 		"--home", cosmosNeutron.HomeDir(),
		// 		"--chain-id", cosmosNeutron.Config().ChainID,
		// 		"--gas", "auto",
		// 		"--fees", "15000untrn",
		// 		"--keyring-backend", keyring.BackendTest,
		// 		"-y",
		// 	}

		// 	_, _, err := cosmosNeutron.Exec(ctx, cmd, nil)
		// 	require.NoError(t, err)

		// 	err = testutil.WaitForBlocks(ctx, 10, neutron, stride)
		// 	require.NoError(t, err, "failed to wait for blocks")

		// 	strideICABal, err := stride.GetBalance(ctx, strideICAAddress, "stuatom")
		// 	require.NoError(t, err, "failed to query ICA balance")
		// 	require.Equal(t, int64(0), strideICABal, "failed to withdraw stuatom to LPer")

		// 	lperStatomBal, err := neutron.GetBalance(
		// 		ctx,
		// 		lperContractAddress,
		// 		neutronStatomDenom,
		// 	)
		// 	require.NoError(t, err, "failed to query lper balance")
		// 	require.Equal(t, int64(10), lperStatomBal, "LPer did not receive stuatoms")
		// })

		// t.Run("tick depositor to ibc transfer atom from ICA account to neutron", func(t *testing.T) {
		// 	atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
		// 	require.NoError(t, err, "failed to get ICA balance")
		// 	require.EqualValues(t, 10, atomBal)

		// 	cmd = []string{"neutrond", "tx", "wasm", "execute", depositorContractAddress,
		// 		`{"tick":{}}`,
		// 		"--from", neutronUser.KeyName,
		// 		"--gas-adjustment", `1.3`,
		// 		"--output", "json",
		// 		"--node", cosmosNeutron.GetRPCAddress(),
		// 		"--home", cosmosNeutron.HomeDir(),
		// 		"--chain-id", cosmosNeutron.Config().ChainID,
		// 		"--gas", "auto",
		// 		"--fees", "15000untrn",
		// 		"--keyring-backend", keyring.BackendTest,
		// 		"-y",
		// 	}

		// 	_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
		// 	require.NoError(t, err)

		// 	err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
		// 	require.NoError(t, err, "failed to wait for blocks")

		// 	atomICABal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
		// 	require.NoError(t, err, "failed to query ICA balance")
		// 	require.Equal(t, int64(0), atomICABal)

		// 	neutronUserBalNew, err := neutron.GetBalance(
		// 		ctx,
		// 		lperContractAddress,
		// 		neutronAtomIbcDenom)
		// 	require.NoError(t, err, "failed to query lper contract atom balance")
		// 	require.Equal(t, int64(10), neutronUserBalNew)
		// })

		// t.Run("[temp] ibc transfer atoms to LP", func(t *testing.T) {
		// 	// TODO: introduce a tick on depositor to forward funds, or withdraw directly to LP address
		// 	_, err := cosmosAtom.SendIBCTransfer(ctx,
		// 		gaiaNeutronTransferChannelId,
		// 		gaiaUser.KeyName, ibc.WalletAmount{
		// 			Address: lperContractAddress,
		// 			Amount:  10,
		// 			Denom:   atom.Config().Denom,
		// 		},
		// 		ibc.TransferOptions{},
		// 	)

		// 	require.NoError(t, err, "failed to send funds from gaia to LP")
		// 	err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
		// 	require.NoError(t, err, "failed to wait for blocks")

		// 	lpBal, err := neutron.GetBalance(ctx, lperContractAddress, neutronAtomIbcDenom)
		// 	require.NoError(t, err, "failed to get ICA balance")
		// 	require.EqualValues(t, 10, lpBal)
		// })

		// t.Run("tick LP to enter pool", func(t *testing.T) {

		// 	cmd = []string{"neutrond", "tx", "wasm", "execute", lperContractAddress,
		// 		`{"tick":{}}`,
		// 		"--from", neutronUser.KeyName,
		// 		"--gas-adjustment", `1.3`,
		// 		"--gas-prices", "0.0untrn",
		// 		"--output", "json",
		// 		"--node", cosmosNeutron.GetRPCAddress(),
		// 		"--home", cosmosNeutron.HomeDir(),
		// 		"--chain-id", cosmosNeutron.Config().ChainID,
		// 		"--gas", "auto",
		// 		"--fees", "15000untrn",
		// 		"--keyring-backend", keyring.BackendTest,
		// 		"-y",
		// 	}

		// 	print(strings.Join(cmd, " "))
		// 	err = testutil.WaitForBlocks(ctx, 100, atom, neutron)

		// 	_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
		// 	require.NoError(t, err, "failed to tick LP")

		// 	require.NoError(t, err, "failed to wait for blocks")
		// })
		// err = testutil.WaitForBlocks(ctx, 100, atom, neutron)

	})

}
