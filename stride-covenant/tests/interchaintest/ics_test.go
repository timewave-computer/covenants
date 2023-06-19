package ibc_test

import (
	"context"
	"encoding/json"
	"fmt"
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

	t.Parallel()

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
						Repository: "ghcr.io/strangelove-ventures/heighliner/stride",
						Version:    "v9.2.1",
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
	_, _ = gaiaUser, strideUser

	neutronUserBal, err := neutron.GetBalance(
		ctx,
		neutronUser.Bech32Address(neutron.Config().Bech32Prefix),
		neutron.Config().Denom)
	require.NoError(t, err, "failed to fund neutron user")
	require.EqualValues(t, int64(100_000_000), neutronUserBal)

	neutronChannelInfo, _ := r.GetChannels(ctx, eRep, neutron.Config().ChainID)
	var neutronGaiaIBCChannel ibc.ChannelOutput
	var neutronGaiaICSChannel ibc.ChannelOutput
	// find the ics channel
	for _, s := range neutronChannelInfo {
		if s.Ordering == "ORDER_ORDERED" {
			neutronGaiaICSChannel = s
			break
		}
	}
	// find the ibc transfer channel to gaia (same connection hops)
	print("\n neutron channels:\n")
	for _, s := range neutronChannelInfo {
		channelJson, _ := json.Marshal(s)
		print("\n", string(channelJson), "\n")
		if s.State == "STATE_OPEN" && s.Ordering == "ORDER_UNORDERED" && s.PortID == "transfer" {
			if len(s.Counterparty.ChannelID) > 5 && s.Counterparty.PortID == "transfer" && s.ConnectionHops[0] == neutronGaiaICSChannel.ConnectionHops[0] {
				neutronGaiaIBCChannel = s
			}
		}
	}

	gaiaNeutronIBCChannel := neutronGaiaIBCChannel.Counterparty

	print("\n gaia channels:\n")
	gaiaChannelInfo, _ := r.GetChannels(ctx, eRep, atom.Config().ChainID)
	for _, s := range gaiaChannelInfo {
		channelJson, _ := json.Marshal(s)
		print("\n", string(channelJson), "\n")
	}

	t.Run("stride covenant tests", func(t *testing.T) {
		const clockContractAddress = "clock_contract_address"
		const holderContractAddress = "holder_contract_address"
		lpInfo := LpInfo{
			Addr: "test",
		}

		var lperContractAddress string
		var depositorContractAddress string
		var stAtomWeightedReceiver WeightedReceiver
		var atomWeightedReceiver WeightedReceiver

		neutronSrcDenomTrace := transfertypes.ParseDenomTrace(
			transfertypes.GetPrefixedDenom("transfer",
				neutronGaiaIBCChannel.ChannelID,
				atom.Config().Denom))
		neutronDstIbcDenom := neutronSrcDenomTrace.IBCDenom()

		t.Run("instantiate lper contract", func(t *testing.T) {
			codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/stride_lper.wasm")
			require.NoError(t, err, "failed to store neutron ICA contract")

			lpMsg := LPerInstantiateMsg{
				LpPosition:    lpInfo,
				ClockAddress:  clockContractAddress,
				HolderAddress: holderContractAddress,
			}

			str, err := json.Marshal(lpMsg)
			require.NoError(t, err, "Failed to marshall LPerInstantiateMsg")

			lperContractAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
			require.NoError(t, err, "failed to instantiate lper contract: ", err)

			print("\n LP contract instantiated with addr: ", lperContractAddress, "\n")

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
				require.Equal(t, lpInfo.Addr, response.Data.Addr)
			})
		})

		t.Run("instantiate depositor contract", func(t *testing.T) {
			codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/stride_depositor.wasm")
			require.NoError(t, err, "failed to store neutron ICA contract")

			stAtomWeightedReceiver = WeightedReceiver{
				Amount:  int64(10),
				Address: lperContractAddress,
			}

			atomWeightedReceiver = WeightedReceiver{
				Amount:  int64(10),
				Address: lperContractAddress,
			}

			msg := DepositorInstantiateMsg{
				StAtomReceiver:                  stAtomWeightedReceiver,
				AtomReceiver:                    atomWeightedReceiver,
				ClockAddress:                    clockContractAddress,
				GaiaNeutronIBCTransferChannelId: gaiaNeutronIBCChannel.ChannelID,
			}

			str, err := json.Marshal(msg)
			require.NoError(t, err, "Failed to marshall DepositorInstantiateMsg")

			depositorContractAddress, err = cosmosNeutron.InstantiateContract(ctx, neutronUser.KeyName, codeId, string(str), true)
			require.NoError(t, err, "failed to instantiate depositor contract: ", err)

			print("\n depositor contract instantiated with addr: ", depositorContractAddress, "\n")

			t.Run("query instantiated clock", func(t *testing.T) {
				var response ClockQueryResponse
				err = cosmosNeutron.QueryContract(ctx, depositorContractAddress, DepositorContractQuery{
					ClockAddress: ClockAddressQuery{},
				}, &response)
				require.NoError(t, err, "failed to query clock address")
				expectedAddrJson, _ := json.Marshal(clockContractAddress)
				require.Equal(t, string(expectedAddrJson), response.Data)
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

		t.Run("deploy astroport contracts", func(t *testing.T) {
			t.Run("factory", func(t *testing.T) {
				// codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_factory.wasm")
				// require.NoError(t, err, "failed to store astroport factory contract")
			})

			t.Run("stableswap", func(t *testing.T) {
				codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_pair_stable.wasm")
				require.NoError(t, err, "failed to store astroport stableswap contract")

				numCodeId, err := strconv.ParseUint(codeId, 10, 64)
				require.NoError(t, err, "failed to parse codeId into uint64")

				initParams := StablePoolParams{
					Amp:   9001,
					Owner: nil,
				}
				binaryData, err := json.Marshal(initParams)
				require.NoError(t, err, "error encoding stable pool params to binary")

				msg := StableswapInstantiateMsg{
					TokenCodeId: numCodeId,
					FactoryAddr: depositorContractAddress, // random
					AssetInfos: []AssetInfo{
						{
							NativeToken: &NativeToken{
								Denom: "statom",
							},
						},
						{
							NativeToken: &NativeToken{
								Denom: atom.Config().Denom,
							},
						},
					},
					InitParams: binaryData,
				}

				str, err := json.Marshal(msg)
				require.NoError(t, err, "Failed to marshall DepositorInstantiateMsg")
				print("\n stableswap init msg: ", string(str), "\n")

				cmd = []string{"neutrond", "tx", "wasm", "instantiate",
					codeId,
					string(str),
					"--from", neutronUser.KeyName,
					"--label", "stableswap",
					"--no-admin",
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

				print(strings.Join(cmd, " "))
				stdout, stderr, err := neutron.Exec(ctx, cmd, nil)
				require.NoError(t, err)
				print("\nstdout: ", stdout)
				print("\nstderr: ", stderr)
			})

		})

		var addrResponse QueryResponse
		t.Run("first tick instantiates ICA", func(t *testing.T) {
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
					ConnectionId:        neutronIcsConnectionId,
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

		t.Run("fund depositor contract with some neutron", func(t *testing.T) {
			err := neutron.SendFunds(ctx, neutronUser.KeyName, ibc.WalletAmount{
				Address: depositorContractAddress,
				Amount:  500001,
				Denom:   neutron.Config().Denom,
			})

			require.NoError(t, err, "failed to send funds from neutron user to depositor contract")
			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			neutronBal, err := neutron.GetBalance(ctx, depositorContractAddress, neutron.Config().Denom)
			require.NoError(t, err, "failed to get depositor neutron balance")
			require.EqualValues(t, 500001, neutronBal)
		})

		t.Run("second tick ibc transfers atom from ICA account to neutron", func(t *testing.T) {
			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, 20, atomBal)

			cmd = []string{"neutrond", "tx", "wasm", "execute", depositorContractAddress,
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

			_, _, err = neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 20, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")

			atomICABal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(10), atomICABal)

			neutronUserBalNew, err := neutron.GetBalance(
				ctx,
				depositorContractAddress,
				neutronDstIbcDenom)
			require.NoError(t, err, "failed to query depositor contract atom balance")
			require.Equal(t, int64(10), neutronUserBalNew)
		})

		// to keep docker containers alive for debugging
		// err = testutil.WaitForBlocks(ctx, 200, atom, neutron)
		t.Run("subsequent ticks do nothing", func(t *testing.T) {
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
				"--from", "faucet",
				"--gas", "50000.0untrn",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			_, _, err = neutron.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron)
			require.NoError(t, err, "failed to wait for blocks")
		})

	})

}
