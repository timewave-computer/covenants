package ibc_test

import (
	"context"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/cosmos/cosmos-sdk/crypto/keyring"
	"github.com/icza/dyno"
	ibctest "github.com/strangelove-ventures/interchaintest/v3"
	"github.com/strangelove-ventures/interchaintest/v3/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v3/ibc"
	"github.com/strangelove-ventures/interchaintest/v3/relayer"
	"github.com/strangelove-ventures/interchaintest/v3/relayer/rly"
	"github.com/strangelove-ventures/interchaintest/v3/testreporter"
	"github.com/strangelove-ventures/interchaintest/v3/testutil"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap/zaptest"
)

type InstantiateMsg struct {
	StAtomReceiver WeightedReceiver `json:"st_atom_receiver"`
	AtomReceiver   WeightedReceiver `json:"atom_receiver"`
	ClockAddress   string           `json:"clock_address,string"`
}

type WeightedReceiver struct {
	Amount  uint64 `json:"amount,string"`
	Address string `json:"address,string"`
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

// This tests Cosmos Interchain Security, spinning up gaia, neutron, and stride
func TestICS(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping in short mode")
	}

	t.Parallel()

	ctx := context.Background()

	// Chain Factory
	cf := ibctest.NewBuiltinChainFactory(zaptest.NewLogger(t), []*ibctest.ChainSpec{
		{Name: "gaia", Version: "v9.1.0", ChainConfig: ibc.ChainConfig{GasAdjustment: 1.5}},
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
				GasPrices:      "0.0untrn",
				GasAdjustment:  10.3,
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
	_ = cosmosNeutron

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

	t.Run("depositor", func(t *testing.T) {
		// Store and instantiate the Neutron ICA example contract. The
		// wasm file is placed in `wasms/` by the `just test` command.
		codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/stride_depositor.wasm")
		require.NoError(t, err, "failed to store neutron ICA contract")

		msg := InstantiateMsg{
			// StAtomReceiver: WeightedReceiver{
			// 	Amount:  10,
			// 	Address: "st_atom_addr",
			// },
			// AtomReceiver: WeightedReceiver{
			// 	Amount:  10,
			// 	Address: "atom_addr",
			// },
			ClockAddress: "clock_addr",
		}

		str, err := json.Marshal(msg)
		require.NoError(t, err, "Failed to marshall instantiateMsg")

		address, err := cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)

		require.NoError(t, err, "failed to instantiate ICA contract: ", err)

		print("\n\ncontract: ", address)
	})

}
