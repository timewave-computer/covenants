package ibc_test

import (
	"context"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/cosmos/cosmos-sdk/crypto/keyring"
	"github.com/cosmos/cosmos-sdk/types"
	transfertypes "github.com/cosmos/ibc-go/v3/modules/apps/transfer/types"
	"github.com/icza/dyno"
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

type InstantiateMsg struct {
	StAtomReceiver                  WeightedReceiver `json:"st_atom_receiver"`
	AtomReceiver                    WeightedReceiver `json:"atom_receiver"`
	ClockAddress                    string           `json:"clock_address,string"`
	GaiaNeutronIBCTransferChannelId string           `json:"gaia_neutron_ibc_transfer_channel_id"`
}

type WeightedReceiver struct {
	Amount  uint64 `json:"amount,string"`
	Address string `json:"address,string"`
}

// A query against the Neutron example contract. Note the usage of
// `omitempty` on fields. This means that if that field has no value,
// it will not have a key in the serialized representaiton of the
// struct, thus mimicing the serialization of Rust enums.
type IcaExampleContractQuery struct {
	InterchainAccountAddress InterchainAccountAddressQuery `json:"interchain_account_address,omitempty"`
}

type InterchainAccountAddressQuery struct {
	InterchainAccountId string `json:"interchain_account_id"`
	ConnectionId        string `json:"connection_id"`
}

type QueryResponse struct {
	Data InterchainAccountAddressQueryResponse `json:"data"`
}

type ICAQueryResponse struct {
	Data DepositorInterchainAccountAddressQueryResponse `json:"data"`
}

type InterchainAccountAddressQueryResponse struct {
	InterchainAccountAddress string `json:"interchain_account_address"`
}

type DepositorICAAddressQuery struct {
	DepositorInterchainAccountAddress DepositorInterchainAccountAddressQuery `json:"depositor_interchain_account_address"`
}

type DepositorContractQuery struct {
	ClockAddress ClockAddressQuery `json:"clock_address"`
}

type StAtomWeightedReceiverQuery struct {
	StAtomReceiver StAtomReceiverQuery `json:"st_atom_receiver"`
}

type AtomWeightedReceiverQuery struct {
	AtomReceiver AtomReceiverQuery `json:"atom_receiver"`
}

type ClockAddressQuery struct{}
type StAtomReceiverQuery struct{}
type AtomReceiverQuery struct{}
type DepositorInterchainAccountAddressQuery struct{}

type WeightedReceiverResponse struct {
	Data WeightedReceiver `json:"data"`
}

type ClockQueryResponse struct {
	Data string `json:"data"`
}

// A query response from the Neutron contract. Note that when
// interchaintest returns query responses, it does so in the form
// `{"data": <RESPONSE>}`, so we need this outer data key, which is
// not present in the neutron contract, to properly deserialze.

type DepositorInterchainAccountAddressQueryResponse struct {
	DepositorInterchainAccountAddress string `json:"depositor_interchain_account_address"`
}

// Sets custom fields for the Neutron genesis file that interchaintest isn't aware of by default.
//
// soft_opt_out_threshold - the bottom `soft_opt_out_threshold`
// percentage of validators may opt out of running a Neutron
// node [^1].
//
// reward_denoms - the reward denominations allowed to be sent to the
// provider (atom) from the consumer (neutron) [^2].
//
// provider_reward_denoms - the reward denominations allowed to be
// sent to the consumer by the provider [^2].
//
// [^1]: https://docs.neutron.org/neutron/consumer-chain-launch#relevant-parameters
// [^2]: https://github.com/cosmos/interchain-security/blob/54e9852d3c89a2513cd0170a56c6eec894fc878d/proto/interchain_security/ccv/consumer/v1/consumer.proto#L61-L66
func setupNeutronGenesis(
	soft_opt_out_threshold string,
	reward_denoms []string,
	provider_reward_denoms []string) func(ibc.ChainConfig, []byte) ([]byte, error) {
	return func(chainConfig ibc.ChainConfig, genbz []byte) ([]byte, error) {
		g := make(map[string]interface{})
		if err := json.Unmarshal(genbz, &g); err != nil {
			return nil, fmt.Errorf("failed to unmarshal genesis file: %w", err)
		}

		if err := dyno.Set(g, soft_opt_out_threshold, "app_state", "ccvconsumer", "params", "soft_opt_out_threshold"); err != nil {
			return nil, fmt.Errorf("failed to set soft_opt_out_threshold in genesis json: %w", err)
		}

		if err := dyno.Set(g, reward_denoms, "app_state", "ccvconsumer", "params", "reward_denoms"); err != nil {
			return nil, fmt.Errorf("failed to set reward_denoms in genesis json: %w", err)
		}

		if err := dyno.Set(g, provider_reward_denoms, "app_state", "ccvconsumer", "params", "provider_reward_denoms"); err != nil {
			return nil, fmt.Errorf("failed to set provider_reward_denoms in genesis json: %w", err)
		}

		out, err := json.Marshal(g)

		if err != nil {
			return nil, fmt.Errorf("failed to marshal genesis bytes to json: %w", err)
		}
		return out, nil
	}
}

func setupGaiaGenesis() func(ibc.ChainConfig, []byte) ([]byte, error) {
	return func(chainConfig ibc.ChainConfig, genbz []byte) ([]byte, error) {
		g := make(map[string]interface{})
		if err := json.Unmarshal(genbz, &g); err != nil {
			return nil, fmt.Errorf("failed to unmarshal genesis file: %w", err)
		}

		arr := []string{
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
		}

		if err := dyno.Set(g, arr, "app_state", "interchainaccounts", "host_genesis_state", "params", "allow_messages"); err != nil {
			return nil, fmt.Errorf("failed to set allow_messages for interchainaccount host in genesis json: %w", err)
		}

		out, err := json.Marshal(g)
		if err != nil {
			return nil, fmt.Errorf("failed to marshal genesis bytes to json: %w", err)
		}
		return out, nil
	}
}

// This tests Cosmos Interchain Security, spinning up gaia, neutron, and stride
func TestICS(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping in short mode")
	}

	t.Parallel()

	ctx := context.Background()

	// Chain Factory
	cf := ibctest.NewBuiltinChainFactory(zaptest.NewLogger(t, zaptest.Level(zap.WarnLevel)), []*ibctest.ChainSpec{
		{Name: "gaia", Version: "v9.1.0", ChainConfig: ibc.ChainConfig{
			GasAdjustment: 1.3,
			GasPrices:     "0.0atom",
			ModifyGenesis: setupGaiaGenesis(),
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
						Repository: "ghcr.io/strangelove-ventures/heighliner/stride",
						Version:    "v9.1.1",
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
			},
		},
	})

	chains, err := cf.Chains(t.Name())
	require.NoError(t, err)

	// interchaintest has one interface for a chain with IBC
	// support, and another for a Cosmos blockchain.
	atom, neutron, stride := chains[0], chains[1], chains[2]
	_, cosmosNeutron := atom.(*cosmos.CosmosChain), neutron.(*cosmos.CosmosChain)

	// Relayer Factory
	client, network := ibctest.DockerSetup(t)
	r := ibctest.NewBuiltinRelayerFactory(
		ibc.CosmosRly,
		zaptest.NewLogger(t, zaptest.Level(zap.DebugLevel)),
		relayer.CustomDockerImage("ghcr.io/cosmos/relayer", "v2.3.1", rly.RlyDefaultUidGid),
		relayer.RelayerOptionExtraStartFlags{Flags: []string{"-d", "--log-format", "console"}},
	).Build(t, client, network)

	const clockContractAddress = "clock_contract_address"
	const icaAccountId = "test"
	var icaAccountAddress string
	// Prep Interchain
	const gaiaNeutronICSPath = "gn-ics-path"
	const gaiaNeutronIBCPath = "gn-ibc-path"
	const gaiaStrideIBCPath = "gs-ibc-path"
	ic := ibctest.NewInterchain().
		AddChain(atom).
		AddChain(neutron).
		AddChain(stride).
		AddRelayer(r, "relayer").
		AddProviderConsumerLink(ibctest.ProviderConsumerLink{
			Provider: atom,
			Consumer: neutron,
			Relayer:  r,
			Path:     gaiaNeutronICSPath,
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  atom,
			Chain2:  neutron,
			Relayer: r,
			Path:    gaiaNeutronIBCPath,
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  atom,
			Chain2:  stride,
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
	err = r.StartRelayer(ctx, eRep, gaiaNeutronICSPath, gaiaNeutronIBCPath, gaiaStrideIBCPath)
	require.NoError(t, err, "failed to start relayer with given paths")
	t.Cleanup(func() {
		err = r.StopRelayer(ctx, eRep)
		if err != nil {
			t.Logf("failed to stop relayer: %s", err)
		}
	})

	err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
	require.NoError(t, err, "failed to wait for blocks")

	connections, err := r.GetConnections(ctx, eRep, "neutron-2")
	require.NoError(t, err, "failed to get neutron-2 IBC connections from relayer")
	var neutronIcsConnectionId string
	for _, connection := range connections {
		for _, version := range connection.Versions {
			if version.String() != "transfer" {
				neutronIcsConnectionId = connection.ID
				break
			}
		}
	}

	// Before receiving a validator set change (VSC) packet,
	// consumer chains disallow bank transfers. To trigger a VSC
	// packet, this creates a validator (from a random public key)
	// that will never do anything, triggering a VSC
	// packet. Eventually this validator will become jailed,
	// triggering another one.
	cmd := []string{"gaiad", "tx", "staking", "create-validator",
		"--amount", "1000000uatom",
		"--pubkey", `{"@type":"/cosmos.crypto.ed25519.PubKey","key":"qwrYHaJ7sNHfYBR1nzDr851+wT4ed6p8BbwTeVhaHoA="}`,
		"--moniker", "a",
		"--commission-rate", "0.1",
		"--commission-max-rate", "0.2",
		"--commission-max-change-rate", "0.01",
		"--min-self-delegation", "1000000",
		"--node", atom.GetRPCAddress(),
		"--home", atom.HomeDir(),
		"--chain-id", atom.Config().ChainID,
		"--from", "faucet",
		"--fees", "20000uatom",
		"--keyring-backend", keyring.BackendTest,
		"-y",
	}
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
	_, _ = gaiaUser, strideUser
	neutron.CreateKey(ctx, "lper")
	neutron.CreateKey(ctx, "lser")

	// neutronAddress := neutronUser.Bech32Address(neutron.Config().Bech32Prefix)
	// atomAddress := gaiaUser.Bech32Address(atom.Config().Bech32Prefix)

	lpAddressBytes, _ := neutron.GetAddress(ctx, "lper")
	lsAddressBytes, _ := neutron.GetAddress(ctx, "lser")

	lpAddress, err := types.Bech32ifyAddressBytes(neutron.Config().Bech32Prefix, lpAddressBytes)
	lsAddress, err := types.Bech32ifyAddressBytes(neutron.Config().Bech32Prefix, lsAddressBytes)

	neutronUserBal, err := neutron.GetBalance(
		ctx,
		neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
		neutron.Config().Denom)
	require.NoError(t, err, "failed to fund neutron user")
	require.EqualValues(t, int64(100_000_000), neutronUserBal)

	neutronChannelInfo, _ := r.GetChannels(ctx, eRep, neutron.Config().ChainID)
	var neutronGaiaIBCChannel ibc.ChannelOutput
	var neutronGaiaICSChannel ibc.ChannelOutput
	for _, s := range neutronChannelInfo {
		neutronJson, _ := json.Marshal(s)
		print("\n neutron_channel: ", string(neutronJson))
		if s.State == "STATE_OPEN" && s.Ordering == "ORDER_UNORDERED" && s.PortID == "transfer" {
			if len(s.Counterparty.ChannelID) > 5 && s.Counterparty.PortID == "transfer" {
				neutronGaiaIBCChannel = s
			}
		} else if s.Ordering == "ORDER_ORDERED" {
			neutronGaiaICSChannel = s
		}
	}
	gaiaNeutronIBCChannel := neutronGaiaIBCChannel.Counterparty
	neutronGaiaIBCChannelId := neutronGaiaIBCChannel.ChannelID
	gaiaNeutronIBCChannelId := gaiaNeutronIBCChannel.ChannelID
	gaiaNeutronICSChannelId := neutronGaiaICSChannel.Counterparty.ChannelID
	neutronGaiaICSChannelId := neutronGaiaICSChannel.ChannelID
	print("\nneutronGaiaIBCChannelId = ", neutronGaiaIBCChannelId)
	print("\ngaiaNeutronIBCChannelId = ", gaiaNeutronIBCChannelId, "\n")

	t.Run("instantiate depositor", func(t *testing.T) {
		// Store and instantiate the Neutron ICA example contract. The
		// wasm file is placed in `wasms/` by the `just test` command.
		codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/stride_depositor.wasm")
		require.NoError(t, err, "failed to store neutron ICA contract")
		stAtomWeightedReceiver := WeightedReceiver{
			Amount:  10,
			Address: "neutron1ud6resqzgewt92njs826m5st98n9r6kkjnurup",
		}
		atomWeightedReceiver := WeightedReceiver{
			Amount:  10,
			Address: "neutron1q0z62s0q29ecay5x7vc3lj6yq7md4lmyafqs5p",
		}

		msg := InstantiateMsg{
			StAtomReceiver:                  stAtomWeightedReceiver,
			AtomReceiver:                    atomWeightedReceiver,
			ClockAddress:                    clockContractAddress,
			GaiaNeutronIBCTransferChannelId: gaiaNeutronIBCChannelId,
		}

		str, err := json.Marshal(msg)
		require.NoError(t, err, "Failed to marshall instantiateMsg")

		address, err := cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
		require.NoError(t, err, "failed to instantiate depositor contract: ", err)

		t.Run("query instantiated clock", func(t *testing.T) {
			var response ClockQueryResponse
			err = cosmosNeutron.QueryContract(ctx, address, DepositorContractQuery{
				ClockAddress: ClockAddressQuery{},
			}, &response)
			require.NoError(t, err, "failed to query clock address")
			expectedAddrJson, _ := json.Marshal(clockContractAddress)
			require.Equal(t, string(expectedAddrJson), response.Data)
		})

		t.Run("query instantiated weighted receivers", func(t *testing.T) {
			var stAtomReceiver WeightedReceiverResponse
			err = cosmosNeutron.QueryContract(ctx, address, StAtomWeightedReceiverQuery{
				StAtomReceiver: StAtomReceiverQuery{},
			}, &stAtomReceiver)
			require.NoError(t, err, "failed to query stAtom weighted receiver")
			require.Equal(t, stAtomWeightedReceiver, stAtomReceiver.Data)

			var atomReceiver WeightedReceiverResponse
			err = cosmosNeutron.QueryContract(ctx, address, AtomWeightedReceiverQuery{
				AtomReceiver: AtomReceiverQuery{},
			}, &atomReceiver)
			require.NoError(t, err, "failed to query atom weighted receiver")
			require.Equal(t, atomWeightedReceiver, atomReceiver.Data)
		})

		var addrResponse QueryResponse
		t.Run("first tick instantiates ICA", func(t *testing.T) {
			// should remain constant
			cmd = []string{"neutrond", "tx", "wasm", "execute", address,
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

			// Wait a bit for the ICA packet to get relayed. This takes a
			// long time as the relayer has to do an entire IBC handshake
			// because ICA creates a channel per account.
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			// Finally, we query the contract for the address of the
			// account on Atom.
			var response QueryResponse
			err = cosmosNeutron.QueryContract(ctx, address, IcaExampleContractQuery{
				InterchainAccountAddress: InterchainAccountAddressQuery{
					InterchainAccountId: icaAccountId,
					ConnectionId:        neutronIcsConnectionId,
				},
			}, &response)
			require.NoError(t, err, "failed to query ICA account address")
			require.NotEmpty(t, response.Data.InterchainAccountAddress)
			icaAccountAddress = response.Data.InterchainAccountAddress
			print("\n icaAccountAddress: ", icaAccountAddress, "\n")
			err = cosmosNeutron.QueryContract(ctx, address, DepositorICAAddressQuery{
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
		})

		t.Run("multisig transfers atom to ICA account", func(t *testing.T) {
			// transfer funds from gaiaUser to the newly generated ICA account
			err := atom.SendFunds(ctx, gaiaUser.KeyName, ibc.WalletAmount{
				Address: icaAccountAddress,
				Amount:  20,
				Denom:   atom.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from gaia to neutron ICA")
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, 20, atomBal)
		})

		neutronSrcDenomTrace := transfertypes.ParseDenomTrace(
			transfertypes.GetPrefixedDenom("transfer",
				neutronGaiaIBCChannelId,
				atom.Config().Denom))
		neutronDstIbcDenom := neutronSrcDenomTrace.IBCDenom()

		t.Run("fund depositor contract with some neutron", func(t *testing.T) {
			err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: address,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from neutron user to depositor contract")
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			neutronBal, err := neutron.GetBalance(ctx, address, neutron.Config().Denom)
			require.NoError(t, err, "failed to get depositor neutron balance")
			require.EqualValues(t, 500001, neutronBal)
		})

		t.Run("second tick ibc transfers atom from ICA account to neutron", func(t *testing.T) {
			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, 20, atomBal)

			cmd = []string{"neutrond", "tx", "wasm", "execute", address,
				`{"tick":{}}`,
				"--from", neutronUser.KeyName,
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--home", neutron.HomeDir(),
				"--chain-id", neutron.Config().ChainID,
				"--gas", "auto",
				"--fees", "500000untrn",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			stdout, stderr, err := neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)
			print("\n stdout: ", string(stdout))
			print("\n stderr: ", string(stderr), "\n")

			require.NoError(t, r.FlushPackets(ctx, eRep, gaiaNeutronIBCPath, neutronGaiaIBCChannelId))
			require.NoError(t, r.FlushAcknowledgements(ctx, eRep, gaiaNeutronIBCPath, gaiaNeutronIBCChannelId))

			// relay ics packets and acks
			require.NoError(t, r.FlushPackets(ctx, eRep, gaiaNeutronICSPath, neutronGaiaICSChannelId))
			require.NoError(t, r.FlushAcknowledgements(ctx, eRep, gaiaNeutronICSPath, gaiaNeutronICSChannelId))

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			atomICABal, err := atom.GetBalance(
				ctx,
				icaAccountAddress,
				atom.Config().Denom)
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(0), atomICABal)

			neutronUserBalNew, err := neutron.GetBalance(
				ctx,
				address,
				neutronDstIbcDenom)
			require.NoError(t, err, "failed to query depositor contract atom balance")
			require.Equal(t, int64(20), neutronUserBalNew)

		})

		t.Run("third tick transfers to LS and LP modules", func(t *testing.T) {
			initLiquidStakerAtomBal, err := neutron.GetBalance(ctx, lsAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get LSer balance")
			initLiquidityPoolerAtomBal, err := neutron.GetBalance(ctx, lpAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get LPer balance")
			require.EqualValues(t, int64(0), initLiquidStakerAtomBal)
			require.EqualValues(t, int64(0), initLiquidityPoolerAtomBal)

			print("\n ticking...\n")
			cmd = []string{"neutrond", "tx", "wasm", "execute", address,
				`{"tick":{}}`,
				"--from", neutronUser.KeyName,
				"--gas-prices", "0.0untrn",
				"--gas-adjustment", `1.5`,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--home", neutron.HomeDir(),
				"--chain-id", neutron.Config().ChainID,
				"--from", "faucet",
				"--gas", "50000.0untrn",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err = neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			// Wait a bit for the ICA packet to get relayed. This takes a
			// long time as the relayer has to do an entire IBC handshake
			// because ICA creates a channel per account.
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			depositorAtomBal, err := neutron.GetBalance(ctx, address, atom.Config().Denom)
			require.NoError(t, err, "failed to query depositor atom balance")
			require.Equal(t, int64(0), depositorAtomBal)

			// query respective accounts and validate they received the funds
			liquidStakerAtomBal, err := cosmosNeutron.GetBalance(ctx, lsAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get LSer balance")
			liquidityPoolerAtomBal, err := cosmosNeutron.GetBalance(ctx, lpAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get LPer balance")
			require.EqualValues(t, int64(10), liquidStakerAtomBal, "LS did not receive atom")
			require.EqualValues(t, int64(10), liquidityPoolerAtomBal, "LP did not receive atom")
		})

		t.Run("subsequent ticks do nothing", func(t *testing.T) {
			cmd = []string{"neutrond", "tx", "wasm", "execute", address,
				`{"tick":{}}`,
				"--from", neutronUser.KeyName,
				"--gas-prices", "0.0untrn",
				"--gas-adjustment", `1.5`,
				"--output", "json",
				"--home", "/var/cosmos-chain/neutron-2",
				"--node", neutron.GetRPCAddress(),
				"--home", neutron.HomeDir(),
				"--chain-id", neutron.Config().ChainID,
				"--from", "faucet",
				"--gas", "50000.0untrn",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err = neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			// Wait a bit for the ICA packet to get relayed. This takes a
			// long time as the relayer has to do an entire IBC handshake
			// because ICA creates a channel per account.
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")
		})
	})

}
