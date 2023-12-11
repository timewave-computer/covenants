package ibc_test

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/cosmos/cosmos-sdk/crypto/keyring"
	"github.com/icza/dyno"
	"github.com/strangelove-ventures/interchaintest/v4/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v4/ibc"
	"github.com/strangelove-ventures/interchaintest/v4/testreporter"
)

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
	provider_reward_denoms []string,
	allowed_messages []string) func(ibc.ChainConfig, []byte) ([]byte, error) {
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

		if err := dyno.Set(g, allowed_messages, "app_state", "interchainaccounts", "host_genesis_state", "params", "allow_messages"); err != nil {
			return nil, fmt.Errorf("failed to set allow_messages for interchainaccount host in genesis json: %w", err)
		}

		if err := dyno.Set(g, "1000000000", "consensus_params", "block", "max_gas"); err != nil {
			return nil, fmt.Errorf("failed to set block max gas: %w", err)
		}

		minGasEntries := []interface{}{
			map[string]string{"denom": "untrn", "amount": "0"},
		}

		if err := dyno.Set(g, minGasEntries, "app_state", "globalfee", "params", "minimum_gas_prices"); err != nil {
			return nil, fmt.Errorf("failed to set min gas entries in genesis json: %w", err)
		}

		if err := dyno.Set(g, "neutron1ctnjk7an90lz5wjfvr3cf6x984a8cjnv8dpmztmlpcq4xteaa2xsj6vhez", "app_state", "tokenfactory", "params", "fee_collector_address"); err != nil {
			return nil, fmt.Errorf("failed to set fee_collector_address in genesis json: %w", err)
		}

		if err := dyno.Set(g, "neutron1ctnjk7an90lz5wjfvr3cf6x984a8cjnv8dpmztmlpcq4xteaa2xsj6vhez", "app_state", "feeburner", "params", "treasury_address"); err != nil {
			return nil, fmt.Errorf("failed to set treasury_address in genesis json: %w", err)
		}

		out, err := json.Marshal(g)
		println("neutron genesis:")
		print(string(out))
		if err != nil {
			return nil, fmt.Errorf("failed to marshal genesis bytes to json: %w", err)
		}
		return out, nil
	}
}

// Sets custom fields for the Gaia genesis file that interchaintest isn't aware of by default.
//
// allowed_messages - explicitly allowed messages to be accepted by the the interchainaccounts section
func setupGaiaGenesis(allowed_messages []string) func(ibc.ChainConfig, []byte) ([]byte, error) {
	return func(chainConfig ibc.ChainConfig, genbz []byte) ([]byte, error) {
		g := make(map[string]interface{})
		if err := json.Unmarshal(genbz, &g); err != nil {
			return nil, fmt.Errorf("failed to unmarshal genesis file: %w", err)
		}

		if err := dyno.Set(g, allowed_messages, "app_state", "interchainaccounts", "host_genesis_state", "params", "allow_messages"); err != nil {
			return nil, fmt.Errorf("failed to set allow_messages for interchainaccount host in genesis json: %w", err)
		}

		out, err := json.Marshal(g)
		if err != nil {
			return nil, fmt.Errorf("failed to marshal genesis bytes to json: %w", err)
		}
		return out, nil
	}
}

func getDefaultInterchainGenesisMessages() []string {
	return []string{
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
}

func getDefaultNeutronInterchainGenesisMessages() []string {
	return []string{
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
		"/ibc.lightclients.localhost.v2.ClientState",
		"/ibc.core.client.v1.MsgCreateClient",
		"/ibc.core.client.v1.Query/ClientState",
		"/ibc.core.client.v1.Query/ConsensusState",
		"/ibc.core.connection.v1.Query/Connection",
	}
}

func setupOsmoGenesis(allowed_messages []string) func(ibc.ChainConfig, []byte) ([]byte, error) {
	return func(chainConfig ibc.ChainConfig, genbz []byte) ([]byte, error) {
		g := make(map[string]interface{})
		if err := json.Unmarshal(genbz, &g); err != nil {
			return nil, fmt.Errorf("failed to unmarshal genesis file: %w", err)
		}

		missingFields := map[string]interface{}{
			"active_channels":     []interface{}{},
			"interchain_accounts": []interface{}{},
			"port":                "icahost",
			"params": map[string]interface{}{
				"host_enabled":   true,
				"allow_messages": []interface{}{},
			},
		}
		if g["app_state"].(map[string]interface{})["interchainaccounts"] == nil {
			g["app_state"].(map[string]interface{})["interchainaccounts"] = make(map[string]interface{})
		}

		if g["app_state"].(map[string]interface{})["interchainaccounts"].(map[string]interface{})["host_genesis_state"] == nil {
			g["app_state"].(map[string]interface{})["interchainaccounts"].(map[string]interface{})["host_genesis_state"] = make(map[string]interface{})
		}

		if err := dyno.Set(g, missingFields, "app_state", "interchainaccounts", "host_genesis_state"); err != nil {
			return nil, fmt.Errorf("failed to set interchainaccounts for app_state in genesis json: %w. \ngenesis json: %s", err, g)
		}

		if err := dyno.Set(g, allowed_messages, "app_state", "interchainaccounts", "host_genesis_state", "params", "allow_messages"); err != nil {
			return nil, fmt.Errorf("failed to set allow_messages for interchainaccount host in genesis json: %w. \ngenesis json: %s", err, g)
		}

		out, err := json.Marshal(g)
		if err != nil {
			return nil, fmt.Errorf("failed to marshal genesis bytes to json: %w", err)
		}
		return out, nil
	}
}

func getCreateValidatorCmd(chain ibc.Chain) []string {
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
		"--node", chain.GetRPCAddress(),
		"--home", chain.HomeDir(),
		"--chain-id", chain.Config().ChainID,
		"--from", "faucet",
		"--fees", "20000uatom",
		"--keyring-backend", keyring.BackendTest,
		"-y",
	}

	return cmd
}

func getChannelMap(r ibc.Relayer, ctx context.Context, eRep *testreporter.RelayerExecReporter,
	cosmosStride *cosmos.CosmosChain, cosmosNeutron *cosmos.CosmosChain, cosmosAtom *cosmos.CosmosChain) map[string]string {
	channelMap := map[string]string{
		"hi": "Dog",
	}

	return channelMap
}
