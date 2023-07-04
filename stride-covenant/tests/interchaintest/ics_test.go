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
				ModifyGenesis:  setupNeutronGenesis("0.05", []string{"untrn"}, []string{"uatom"}),
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
			},
		},
	})

	chains, err := cf.Chains(t.Name())
	require.NoError(t, err)

	// interchaintest has one interface for a chain with IBC
	// support, and another for a Cosmos blockchain.
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

	const icaAccountId = "test"
	var icaAccountAddress string
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
	users := ibctest.GetAndFundTestUsers(t, ctx, "default", int64(100_000_000), atom, neutron, stride)
	gaiaUser, neutronUser, strideUser := users[0], users[1], users[2]

	strideAdminMnemonic := "tone cause tribe this switch near host damage idle fragile antique tail soda alien depth write wool they rapid unfold body scan pledge soft"
	strideAdmin, _ := ibctest.GetAndFundTestUserWithMnemonic(ctx, "default", strideAdminMnemonic, (100_000_000), cosmosStride)

	cosmosStride.SendFunds(ctx, strideUser.KeyName, ibc.WalletAmount{
		Address: strideAdmin.Bech32Address(stride.Config().Bech32Prefix),
		Denom:   "ustride",
		Amount:  10000000,
	})

	err = testutil.WaitForBlocks(ctx, 20, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")

	neutronUserBal, err := neutron.GetBalance(
		ctx,
		neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
		neutron.Config().Denom)
	require.NoError(t, err, "failed to fund neutron user")
	require.EqualValues(t, int64(100_000_000), neutronUserBal)

	neutronChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosNeutron.Config().ChainID)
	gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosAtom.Config().ChainID)
	strideChannelInfo, _ := r.GetChannels(ctx, eRep, cosmosStride.Config().ChainID)
	strideConnectionInfo, _ := r.GetConnections(ctx, eRep, cosmosStride.Config().ChainID)
	neutronConnectionInfo, _ := r.GetConnections(ctx, eRep, cosmosNeutron.Config().ChainID)
	gaiaConnectionInfo, _ := r.GetConnections(ctx, eRep, cosmosAtom.Config().ChainID)

	/// Find all the pairwise connections
	var strideGaiaConnectionId, gaiaStrideConnectionId string
	var strideNeutronConnectionId, neutronStrideConnectionId string

	// There should be two sets of connections for gaia <> neutron. Tranfer and CCV
	neutronGaiaConnectionIds := make([]string, 2)
	gaiaNeutronConnectionIds := make([]string, 2)

	var neutronGaiaTransferConnectionId, neutronGaiaICSConnectionId string
	var gaiaNeutronTransferConnectionId, gaiaNeutronICSConnectionId string

	// We iterate over stride connections
	for _, strideConn := range strideConnectionInfo {
		for _, neutronConn := range neutronConnectionInfo {
			if neutronConn.ClientID == strideConn.Counterparty.ClientId &&
				strideConn.ClientID == neutronConn.Counterparty.ClientId &&
				neutronConn.ID == strideConn.Counterparty.ConnectionId &&
				strideConn.ID == neutronConn.Counterparty.ConnectionId {
				strideNeutronConnectionId = strideConn.ID
				neutronStrideConnectionId = neutronConn.ID
			}
		}
		for _, gaiaConn := range gaiaConnectionInfo {
			if strideConn.ClientID == gaiaConn.Counterparty.ClientId &&
				gaiaConn.ClientID == strideConn.Counterparty.ClientId &&
				gaiaConn.ID == strideConn.Counterparty.ConnectionId &&
				strideConn.ID == gaiaConn.Counterparty.ConnectionId {
				strideGaiaConnectionId = strideConn.ID
				gaiaStrideConnectionId = gaiaConn.ID
			}
		}
	}
	for _, neutronConn := range neutronConnectionInfo {
		for _, gaiaConn := range gaiaConnectionInfo {
			if neutronConn.ClientID == gaiaConn.Counterparty.ClientId &&
				gaiaConn.ClientID == neutronConn.Counterparty.ClientId &&
				neutronConn.ID == gaiaConn.Counterparty.ConnectionId &&
				gaiaConn.ID == neutronConn.Counterparty.ConnectionId {
				neutronGaiaConnectionIds = append(neutronGaiaConnectionIds, neutronConn.ID)
				gaiaNeutronConnectionIds = append(gaiaNeutronConnectionIds, neutronConn.ID)
				break
			}
		}
	}

	var strideNeutronChannelId, neutronStrideChannelId string
	var strideGaiaChannelId, gaiaStrideChannelId string
	var neutronGaiaICSChannelId, gaiaNeutronICSChannelId string
	var neutronGaiaTransferChannelId, gaiaNeutronTransferChannelId string

	for _, s := range strideChannelInfo {
		found := false
		if !found {
			for _, n := range neutronChannelInfo {
				if s.ChannelID == n.Counterparty.ChannelID &&
					n.ChannelID == s.Counterparty.ChannelID &&
					s.PortID == n.Counterparty.PortID &&
					n.Ordering == "ORDER_UNORDERED" &&
					s.ConnectionHops[0] == strideNeutronConnectionId &&
					n.ConnectionHops[0] == neutronStrideConnectionId {
					strideNeutronChannelId = s.ChannelID
					neutronStrideChannelId = n.ChannelID
					found = true
					break
				}
			}
		}
		if found {
			break
		}
	}

	for _, s := range strideChannelInfo {
		found := false
		if !found {
			for _, g := range gaiaChannelInfo {
				if s.ChannelID == g.Counterparty.ChannelID &&
					s.Counterparty.ChannelID == g.ChannelID &&
					s.PortID == g.Counterparty.PortID &&
					g.Ordering == "ORDER_UNORDERED" &&
					s.ConnectionHops[0] == strideGaiaConnectionId &&
					g.ConnectionHops[0] == gaiaStrideConnectionId {
					strideGaiaChannelId = s.ChannelID
					gaiaStrideChannelId = g.ChannelID
					found = true
					break
				}
			}
		}
		if found {
			break
		}
	}

	for _, n := range neutronChannelInfo {
		for _, g := range gaiaChannelInfo {
			if n.PortID == "consumer" &&
				g.PortID == "provider" &&
				n.ChannelID == g.Counterparty.ChannelID &&
				g.ChannelID == n.Counterparty.ChannelID {
				neutronGaiaICSChannelId = n.ChannelID
				gaiaNeutronICSChannelId = g.ChannelID
				neutronGaiaICSConnectionId = n.ConnectionHops[0]
				gaiaNeutronICSConnectionId = g.ConnectionHops[0]

			}
		}
	}

	for _, ngci := range neutronGaiaConnectionIds {
		if neutronGaiaICSConnectionId != ngci {
			neutronGaiaTransferConnectionId = ngci
		}
	}

	for _, gnci := range gaiaNeutronConnectionIds {
		if gaiaNeutronICSConnectionId != gnci {
			gaiaNeutronTransferConnectionId = gnci
		}
	}

	for _, n := range neutronChannelInfo {
		for _, g := range gaiaChannelInfo {
			if n.PortID == "transfer" &&
				g.PortID == "transfer" &&
				n.ChannelID == g.Counterparty.ChannelID &&
				g.ChannelID == n.Counterparty.ChannelID &&
				n.ConnectionHops[0] == neutronGaiaTransferConnectionId &&
				g.ConnectionHops[0] == gaiaNeutronTransferConnectionId {
				neutronGaiaTransferChannelId = n.ChannelID
				gaiaNeutronTransferChannelId = g.ChannelID
			}
		}
	}

	print("\n strideGaiaConnectionId: ", strideGaiaConnectionId)
	print("\n gaiaStrideConnectionId: ", gaiaStrideConnectionId)
	print("\n strideNeutronConnectionId: ", strideNeutronConnectionId)
	print("\n neutronStrideConnectionId: ", neutronStrideConnectionId)
	print("\n neutronGaiaTransferConnectionId: ", neutronGaiaTransferConnectionId)
	print("\n neutronGaiaICSConnectionId: ", neutronGaiaICSConnectionId)
	print("\n gaiaNeutronTransferConnectionId: ", gaiaNeutronTransferConnectionId)
	print("\n gaiaNeutronICSConnectionId: ", gaiaNeutronICSConnectionId)

	print("\n gaiaStrideChannelId: ", gaiaStrideChannelId)
	print("\n strideGaiaChannelId: ", strideGaiaChannelId)
	print("\n strideNeutronChannelId: ", strideNeutronChannelId)
	print("\n neutronStrideChannelId: ", neutronStrideChannelId)
	print("\n neutronGaiaTransferChannelId: ", neutronGaiaTransferChannelId)
	print("\n gaiaNeutronTransferChannelId: ", gaiaNeutronTransferChannelId)
	print("\n neutronGaiaICSChannelId: ", neutronGaiaICSChannelId)
	print("\n gaiaNeutronICSChannelId: ", gaiaNeutronICSChannelId)

	_, _, _, _, _ = neutronGaiaTransferChannelId, gaiaNeutronTransferChannelId, neutronGaiaICSChannelId, gaiaNeutronICSChannelId, neutronStrideChannelId
	_, _, _ = gaiaStrideConnectionId, strideGaiaConnectionId, strideNeutronConnectionId

	t.Run("stride covenant tests", func(t *testing.T) {
		const clockContractAddress = "clock_contract_address"
		const holderContractAddress = "holder_contract_address"

		var lperContractAddress string
		var depositorContractAddress string
		var lsContractAddress string
		var stAtomWeightedReceiver WeightedReceiver
		var atomWeightedReceiver WeightedReceiver
		var strideICAAddress string

		neutronSrcDenomTrace := transfertypes.ParseDenomTrace(
			transfertypes.GetPrefixedDenom("transfer",
				neutronGaiaTransferChannelId,
				atom.Config().Denom))
		neutronDstIbcDenom := neutronSrcDenomTrace.IBCDenom()
		// gaiaAddr := gaiaUser.Bech32Address(atom.Config().Bech32Prefix)

		atomSrcDenomTrace := transfertypes.ParseDenomTrace(
			transfertypes.GetPrefixedDenom("transfer",
				strideGaiaChannelId,
				atom.Config().Denom))
		strideAtomIbcDenom := atomSrcDenomTrace.IBCDenom()

		neutronStatomDenomTrace := transfertypes.ParseDenomTrace(
			transfertypes.GetPrefixedDenom("transfer",
				neutronStrideChannelId,
				"stuatom"))
		neutronStatomDenom := neutronStatomDenomTrace.IBCDenom()
		print("\nneutronDstIbcDenom: ", neutronDstIbcDenom)
		print("\nstrideAtomIbcDenom: ", strideAtomIbcDenom)
		print("\nneutronStatomDenom: ", neutronStatomDenom)

		print("\n")

		_ = strideAtomIbcDenom
		var coinRegistryAddress string
		var factoryAddress string
		var stableswapAddress string
		var tokenAddress string
		var whitelistAddress string
		_, _ = tokenAddress, whitelistAddress

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
				// no clue how to marshall go struct fields into rust Vec<(String, u8)>
				// so passing as a string for now
				addMessage := `{"add":{"native_coins":[["statom",10],["uatom",10]]}}`
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

			t.Run("stableswap", func(t *testing.T) {

				initParams := StablePoolParams{
					Amp:   9001,
					Owner: nil,
				}
				binaryData, err := json.Marshal(initParams)
				require.NoError(t, err, "error encoding stable pool params to binary")

				stAtom := NativeToken{
					Denom: "statom",
				}
				nativeAtom := NativeToken{
					Denom: atom.Config().Denom,
				}
				assetInfos := []AssetInfo{
					{
						NativeToken: &stAtom,
					},
					{
						NativeToken: &nativeAtom,
					},
				}

				msg := StableswapInstantiateMsg{
					TokenCodeId: tokenCodeId,
					FactoryAddr: factoryAddress,
					AssetInfos:  assetInfos,
					InitParams:  binaryData,
				}

				str, err := json.Marshal(msg)
				require.NoError(t, err, "Failed to marshall DepositorInstantiateMsg")

				stableswapAddr, err := cosmosNeutron.InstantiateContract(
					ctx, neutronUser.KeyName, stablePairCodeIdStr, string(str), true,
					"--label", "stableswap",
					"--gas-prices", "0.0untrn",
					"--gas-adjustment", `1.5`,
					"--output", "json",
					"--node", neutron.GetRPCAddress(),
					"--home", neutron.HomeDir(),
					"--chain-id", neutron.Config().ChainID,
					"--gas", "auto",
					"--keyring-backend", keyring.BackendTest,
					"-y",
				)
				require.NoError(t, err, "Failed to instantiate stableswap")
				stableswapAddress = stableswapAddr
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
				require.NoError(t, err, "failed to wait for blocks")
			})

		})

		t.Run("instantiate lper contract", func(t *testing.T) {
			codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/covenant_lper.wasm")
			require.NoError(t, err, "failed to store neutron ICA contract")
			lpInfo := LpInfo{
				Addr: stableswapAddress,
			}

			lpMsg := LPerInstantiateMsg{
				LpPosition:    lpInfo,
				ClockAddress:  clockContractAddress,
				HolderAddress: holderContractAddress,
			}

			str, err := json.Marshal(lpMsg)
			require.NoError(t, err, "Failed to marshall LPerInstantiateMsg")

			lperContractAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
			require.NoError(t, err, "failed to instantiate lper contract: ", err)
			print("\n lper: ", lperContractAddress)

			t.Run("query instantiated clock", func(t *testing.T) {
				var response ClockQueryResponse
				err = cosmosNeutron.QueryContract(ctx, lperContractAddress, LPContractQuery{
					ClockAddress: ClockAddressQuery{},
				}, &response)
				require.NoError(t, err, "failed to query clock address")
				expectedAddrJson, _ := json.Marshal(clockContractAddress)
				require.Equal(t, string(expectedAddrJson), response.Data)
			})

			t.Run("query lp position", func(t *testing.T) {
				var response LpPositionQueryResponse
				err := cosmosNeutron.QueryContract(ctx, lperContractAddress, LPPositionQuery{
					LpPosition: LpPositionQuery{},
				}, &response)
				require.NoError(t, err, "failed to query lp position address")
				require.Equal(t, stableswapAddress, response.Data.Addr)
			})
		})

		t.Run("instantiate LS contract", func(t *testing.T) {
			codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/covenant_ls.wasm")
			require.NoError(t, err, "failed to store neutron ICA contract")

			msg := LsInstantiateMsg{
				AutopilotPosition:                 "todo",
				ClockAddress:                      clockContractAddress,
				StrideNeutronIBCTransferChannelId: strideNeutronChannelId,
				LpAddress:                         lperContractAddress,
				NeutronStrideIBCConnectionId:      neutronStrideConnectionId,
				LsDenom:                           "stuatom",
			}

			str, err := json.Marshal(msg)
			require.NoError(t, err, "Failed to marshall LsInstantiateMsg")

			lsContractAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
			require.NoError(t, err, "failed to instantiate ls contract: ", err)
			print("\nls contract:", lsContractAddress, "\n")
		})

		t.Run("create stride ICA", func(t *testing.T) {
			// should remain constant
			cmd = []string{"neutrond", "tx", "wasm", "execute", lsContractAddress,
				`{"tick":{}}`,
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

			_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 5, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			var response QueryResponse
			err = cosmosNeutron.QueryContract(ctx, lsContractAddress, IcaExampleContractQuery{
				InterchainAccountAddress: InterchainAccountAddressQuery{
					InterchainAccountId: "stride-ica",
					ConnectionId:        neutronStrideConnectionId,
				},
			}, &response)
			require.NoError(t, err, "failed to query ICA account address")
			require.NotEmpty(t, response.Data.InterchainAccountAddress)
			strideICAAddress = response.Data.InterchainAccountAddress

			print("\nstride ICA instantiated with address ", strideICAAddress, "\n")
		})

		t.Run("instantiate depositor contract", func(t *testing.T) {
			codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/covenant_depositor.wasm")
			require.NoError(t, err, "failed to store neutron ICA contract")

			stAtomWeightedReceiver = WeightedReceiver{
				Amount:  int64(10),
				Address: strideICAAddress,
			}

			atomWeightedReceiver = WeightedReceiver{
				Amount:  int64(10),
				Address: lperContractAddress,
			}

			msg := DepositorInstantiateMsg{
				StAtomReceiver:                  stAtomWeightedReceiver,
				AtomReceiver:                    atomWeightedReceiver,
				ClockAddress:                    clockContractAddress,
				GaiaNeutronIBCTransferChannelId: gaiaNeutronTransferChannelId,
				GaiaStrideIBCTransferChannelId:  gaiaStrideChannelId,
				NeutronGaiaConnectionId:         neutronGaiaTransferConnectionId,
			}

			str, err := json.Marshal(msg)
			require.NoError(t, err, "Failed to marshall DepositorInstantiateMsg")

			depositorContractAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
			require.NoError(t, err, "failed to instantiate depositor contract: ", err)

			print("\ndepositor contract:", depositorContractAddress, "\n")

			t.Run("query instantiated clock", func(t *testing.T) {
				var response ClockQueryResponse
				err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, DepositorContractQuery{
					ClockAddress: ClockAddressQuery{},
				}, &response)
				require.NoError(t, err, "failed to query clock address")
				require.Equal(t, clockContractAddress, response.Data)
			})

			t.Run("query instantiated weighted receivers", func(t *testing.T) {
				var stAtomReceiver WeightedReceiverResponse
				err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, StAtomWeightedReceiverQuery{
					StAtomReceiver: StAtomReceiverQuery{},
				}, &stAtomReceiver)
				require.NoError(t, err, "failed to query stAtom weighted receiver")
				require.Equal(t, stAtomWeightedReceiver, stAtomReceiver.Data)

				var atomReceiver WeightedReceiverResponse
				err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, AtomWeightedReceiverQuery{
					AtomReceiver: AtomReceiverQuery{},
				}, &atomReceiver)
				require.NoError(t, err, "failed to query atom weighted receiver")
				require.Equal(t, int64(10), atomReceiver.Data.Amount)
				require.Equal(t, lperContractAddress, atomReceiver.Data.Address)
			})
		})

		var addrResponse QueryResponse
		t.Run("tick depositor to instantiate ICA", func(t *testing.T) {
			// should remain constant
			cmd = []string{"neutrond", "tx", "wasm", "execute", depositorContractAddress,
				`{"tick":{}}`,
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

			_, _, err = neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			var response QueryResponse
			err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, IcaExampleContractQuery{
				InterchainAccountAddress: InterchainAccountAddressQuery{
					InterchainAccountId: icaAccountId,
					ConnectionId:        neutronGaiaTransferConnectionId,
				},
			}, &response)
			require.NoError(t, err, "failed to query ICA account address")
			require.NotEmpty(t, response.Data.InterchainAccountAddress)
			icaAccountAddress = response.Data.InterchainAccountAddress
			err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, DepositorICAAddressQuery{
				DepositorInterchainAccountAddress: DepositorInterchainAccountAddressQuery{},
			}, &addrResponse)
			require.NoError(t, err, "failed to query ICA account address")
			require.NotEmpty(t, addrResponse.Data.InterchainAccountAddress)

			// validate that querying an address via neutron query
			// and by retrieving it from store is the same
			require.EqualValues(t,
				response.Data.InterchainAccountAddress,
				icaAccountAddress,
			)

			print("\ndepositor ICA instantiated with address ", icaAccountAddress, "\n")
		})

		t.Run("multisig transfers atom to ICA account", func(t *testing.T) {
			// transfer funds from gaiaUser to the newly generated ICA account
			err := cosmosAtom.SendFunds(ctx, gaiaUser.KeyName, ibc.WalletAmount{
				Address: icaAccountAddress,
				Amount:  20,
				Denom:   atom.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from gaia to neutron ICA")
			err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, 20, atomBal)
		})

		t.Run("fund contracts with neutron", func(t *testing.T) {
			err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: depositorContractAddress,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from neutron user to depositor contract")

			err = neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: lsContractAddress,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from neutron user to ls contract")

			err = testutil.WaitForBlocks(ctx, 2, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			depositorNeutronBal, err := neutron.GetBalance(ctx, depositorContractAddress, neutron.Config().Denom)
			require.NoError(t, err, "failed to get depositor neutron balance")
			require.EqualValues(t, 500001, depositorNeutronBal)

			lsNeutronBal, err := neutron.GetBalance(ctx, lsContractAddress, neutron.Config().Denom)
			require.NoError(t, err, "failed to get depositor neutron balance")
			require.EqualValues(t, 500001, lsNeutronBal)
		})

		t.Run("tick depositor to liquid stake on stride", func(t *testing.T) {
			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, 20, atomBal)

			require.NoError(t, err, "failed to wait for blocks")
			cmd = []string{"neutrond", "tx", "wasm", "execute", depositorContractAddress,
				`{"tick":{}}`,
				"--from", neutronUser.KeyName,
				"--gas-adjustment", `1.3`,
				"--gas-prices", "0.0untrn",
				"--output", "json",
				"--node", cosmosNeutron.GetRPCAddress(),
				"--home", cosmosNeutron.HomeDir(),
				"--chain-id", cosmosNeutron.Config().ChainID,
				"--gas", "auto",
				"--fees", "5000untrn",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 20, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			atomICABal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(10), atomICABal)

			strideICABal, err := stride.GetBalance(ctx, strideICAAddress, "stuatom")
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(10), strideICABal)
			print("\n stride ica bal: ", strideICABal, "\n")
		})

		t.Run("tick ls to transfer stuatom to LPer", func(t *testing.T) {

			cmd = []string{"neutrond", "tx", "wasm", "execute", lsContractAddress,
				`{"tick":{}}`,
				"--from", neutronUser.KeyName,
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--node", cosmosNeutron.GetRPCAddress(),
				"--home", cosmosNeutron.HomeDir(),
				"--chain-id", cosmosNeutron.Config().ChainID,
				"--gas", "auto",
				"--fees", "15000untrn",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err := cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 10, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			strideICABal, err := stride.GetBalance(ctx, strideICAAddress, "stuatom")
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(0), strideICABal, "failed to withdraw stuatom to LPer")

			lperStatomBal, err := neutron.GetBalance(
				ctx,
				lperContractAddress,
				neutronStatomDenom,
			)
			require.NoError(t, err, "failed to query lper balance")
			require.Equal(t, int64(10), lperStatomBal, "LPer did not receive stuatoms")
		})

		t.Run("tick depositor to ibc transfer atom from ICA account to neutron", func(t *testing.T) {
			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, 10, atomBal)

			cmd = []string{"neutrond", "tx", "wasm", "execute", depositorContractAddress,
				`{"tick":{}}`,
				"--from", neutronUser.KeyName,
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--node", cosmosNeutron.GetRPCAddress(),
				"--home", cosmosNeutron.HomeDir(),
				"--chain-id", cosmosNeutron.Config().ChainID,
				"--gas", "auto",
				"--fees", "15000untrn",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err = cosmosNeutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			atomICABal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(0), atomICABal)

			neutronUserBalNew, err := neutron.GetBalance(
				ctx,
				lperContractAddress,
				neutronDstIbcDenom)
			require.NoError(t, err, "failed to query lper contract atom balance")
			require.Equal(t, int64(10), neutronUserBalNew)
		})

		require.NoError(t, err, "failed to wait for blocks")
	})

}
