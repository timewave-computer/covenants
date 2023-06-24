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
				ModifyGenesis:  setupStrideGenesis(),
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
		}).
		AddLink(ibctest.InterchainLink{
			Chain1:  neutron,
			Chain2:  stride,
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
	_, _ = gaiaUser, strideUser

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

	var strideNeutronChannelId, strideGaiaChannelId, gaiaStrideChannelId, neutronStrideChannelId string

	for _, s := range strideChannelInfo {
		for _, n := range neutronChannelInfo {
			if s.ChannelID == n.Counterparty.ChannelID && s.Counterparty.ChannelID == n.ChannelID &&
				s.PortID == n.Counterparty.PortID && n.Ordering == "ORDER_UNORDERED" {
				strideNeutronChannelId = s.ChannelID
				neutronStrideChannelId = n.ChannelID
			}
		}
		for _, g := range gaiaChannelInfo {
			if s.ChannelID == g.Counterparty.ChannelID && g.ChannelID == s.Counterparty.ChannelID &&
				s.PortID == g.Counterparty.PortID && g.Ordering == "ORDER_UNORDERED" {
				strideGaiaChannelId = s.ChannelID
				gaiaStrideChannelId = g.ChannelID
			}
		}
	}
	_, _ = strideGaiaChannelId, neutronStrideChannelId

	var neutronGaiaICSChannelId, gaiaNeutronICSChannelId string
	var neutronGaiaTransferChannelId, gaiaNeutronTransferChannelId string
	var neutronGaiaICSConnectionHopId string

	for _, n := range neutronChannelInfo {
		for _, g := range gaiaChannelInfo {
			if n.PortID == "consumer" && g.PortID == "provider" {
				neutronGaiaICSChannelId = n.ChannelID
				gaiaNeutronICSChannelId = g.ChannelID
				neutronGaiaICSConnectionHopId = n.ConnectionHops[0]
			} else if n.ChannelID == g.Counterparty.ChannelID && g.ChannelID == n.Counterparty.ChannelID &&
				n.PortID == g.Counterparty.PortID && n.Counterparty.PortID == g.PortID {
				neutronGaiaTransferChannelId = n.ChannelID
				gaiaNeutronTransferChannelId = g.ChannelID
			}
		}
	}

	print("\n gaiaStrideChannelId: ", gaiaStrideChannelId)
	print("\n strideGaiaChannelId: ", strideGaiaChannelId)
	print("\n strideNeutronChannelId: ", strideNeutronChannelId)
	print("\n neutronStrideChannelId: ", neutronStrideChannelId)
	print("\n neutronGaiaTransferChannelId: ", neutronGaiaTransferChannelId)
	print("\n gaiaNeutronTransferChannelId: ", gaiaNeutronTransferChannelId)
	print("\n neutronGaiaICSChannelId: ", neutronGaiaICSChannelId)
	print("\n gaiaNeutronICSChannelId: ", gaiaNeutronICSChannelId)

	var strideGaiaConnectionId, gaiaStrideConnectionId, strideNeutronConnectionId, neutronStrideConnectionId string

	// we iterate over stride connections
	for _, strideConn := range strideConnectionInfo {
		for _, neutronConn := range neutronConnectionInfo {
			if neutronConn.ClientID == strideConn.Counterparty.ClientId && strideConn.ClientID == neutronConn.Counterparty.ClientId {
				// strideNeutronTransferChannel.ConnectionHops[0] == strideConn.ID {
				strideNeutronConnectionId = strideConn.ID
				neutronStrideConnectionId = neutronConn.ID
			}
		}
		for _, gaiaConn := range gaiaConnectionInfo {
			if strideConn.ClientID == gaiaConn.Counterparty.ClientId && strideConn.Counterparty.ClientId == gaiaConn.ClientID {
				// strideGaiaTransferChannel.ConnectionHops[0] == strideConn.ID {
				strideGaiaConnectionId = strideConn.ID
				gaiaStrideConnectionId = gaiaConn.ID
			}
		}
	}

	var neutronGaiaTransferConnectionId, neutronGaiaICSConnectionId string
	var gaiaNeutronTransferConnectionId, gaiaNeutronICSConnectionId string
	_, _ = gaiaNeutronTransferConnectionId, gaiaNeutronICSConnectionId
	for _, neutronConn := range neutronConnectionInfo {
		if neutronGaiaICSConnectionHopId == neutronConn.ID {
			neutronGaiaICSConnectionId = neutronConn.ID
			gaiaNeutronICSConnectionId = neutronConn.Counterparty.ConnectionId
			break
		}
	}
	for _, neutronConn := range neutronConnectionInfo {
		for _, gaiaConn := range gaiaConnectionInfo {
			if neutronConn.ID != neutronGaiaICSConnectionId && gaiaConn.ID != gaiaNeutronICSConnectionId &&
				neutronConn.ClientID == gaiaConn.Counterparty.ClientId && gaiaConn.ClientID == neutronConn.Counterparty.ClientId {
				neutronGaiaTransferConnectionId = neutronConn.ID
				gaiaNeutronTransferConnectionId = gaiaConn.ID
				break
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

	_, _, _, _ = neutronGaiaTransferChannelId, gaiaNeutronTransferChannelId, neutronGaiaICSChannelId, gaiaNeutronICSChannelId
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
		gaiaAddr := gaiaUser.Bech32Address(atom.Config().Bech32Prefix)

		atomSrcDenomTrace := transfertypes.ParseDenomTrace(
			transfertypes.GetPrefixedDenom("transfer",
				gaiaStrideChannelId,
				atom.Config().Denom))
		strideAtomIbcDenom := atomSrcDenomTrace.IBCDenom()
		print("\nneutronDstIbcDenom: ", neutronDstIbcDenom)
		print("\nstrideAtomIbcDenom: ", strideAtomIbcDenom)

		_ = strideAtomIbcDenom
		var coinRegistryAddress string
		var factoryAddress string
		var stableswapAddress string
		var tokenAddress string
		var whitelistAddress string
		_, _ = tokenAddress, whitelistAddress

		t.Run("deploy astroport contracts", func(t *testing.T) {
			stablePairCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_pair_stable.wasm")
			require.NoError(t, err, "failed to store astroport stableswap contract")
			stablePairCodeId, err := strconv.ParseUint(stablePairCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			factoryCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_factory.wasm")
			require.NoError(t, err, "failed to store astroport factory contract")
			// factoryCodeId, err := strconv.ParseUint(factoryCodeIdStr, 10, 64)
			// require.NoError(t, err, "failed to parse codeId into uint64")

			whitelistCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_whitelist.wasm")
			require.NoError(t, err, "failed to store astroport whitelist contract")
			whitelistCodeId, err := strconv.ParseUint(whitelistCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			tokenCodeIdStr, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/astroport_token.wasm")
			require.NoError(t, err, "failed to store astroport token contract")
			tokenCodeId, err := strconv.ParseUint(tokenCodeIdStr, 10, 64)
			require.NoError(t, err, "failed to parse codeId into uint64")

			t.Run("astroport token", func(t *testing.T) {

				// cap := uint64(1)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
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

				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
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
				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
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

				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
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

				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
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

				err = testutil.WaitForBlocks(ctx, 2, atom, neutron, stride)
				require.NoError(t, err, "failed to wait for blocks")
			})

		})

		t.Run("instantiate lper contract", func(t *testing.T) {
			codeId, err := cosmosNeutron.StoreContract(ctx, neutronUser.KeyName, "wasms/stride_lper.wasm")
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

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
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

		t.Run("ibc transfer to stride ica", func(t *testing.T) {
			atomBal, _ := atom.GetBalance(ctx, gaiaAddr, atom.Config().Denom)

			cmd := []string{"gaiad", "tx", "ibc-transfer", "transfer", "transfer",
				gaiaStrideChannelId,
				strideICAAddress,
				"100uatom",
				"--from", gaiaUser.KeyName,
				"--gas", "auto",
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--node", atom.GetRPCAddress(),
				"--home", atom.HomeDir(),
				"--chain-id", atom.Config().ChainID,
				"--fees", "10uatom",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			print("\ncmd: \n")
			print(strings.Join(cmd, " "))
			print("\n")
			_, _, err = cosmosAtom.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 25, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			newAtomBal, _ := atom.GetBalance(ctx, gaiaAddr, atom.Config().Denom)
			require.EqualValues(t, atomBal-110, newAtomBal)

			strideBal, _ := stride.GetBalance(ctx, strideICAAddress, strideAtomIbcDenom)
			print("\n stride bal: ", strideBal)
		})

		t.Run("stride one click LS", func(t *testing.T) {
			atomBal, _ := atom.GetBalance(ctx, gaiaAddr, atom.Config().Denom)

			noForwardMemo := fmt.Sprintf(`"{"autopilot": {"receiver": "%s", "stakeibc": {"action": "LiquidStake",}}}"`,
				strideICAAddress)

			cmd := []string{"gaiad", "tx", "ibc-transfer", "transfer", "transfer",
				gaiaStrideChannelId,
				noForwardMemo,
				"100uatom",
				"--from", gaiaUser.KeyName,
				"--gas", "auto",
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--node", atom.GetRPCAddress(),
				"--home", atom.HomeDir(),
				"--chain-id", atom.Config().ChainID,
				"--fees", "10uatom",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			print("\ncmd: \n")
			print(strings.Join(cmd, " "))

			_, _, err = cosmosAtom.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			err = testutil.WaitForBlocks(ctx, 20, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			newAtomBal, _ := atom.GetBalance(ctx, gaiaAddr, atom.Config().Denom)
			require.EqualValues(t, atomBal-110, newAtomBal)

			strideBal, _ := stride.GetBalance(ctx, strideICAAddress, strideAtomIbcDenom)
			print("\n stride bal: ", strideBal)

			// strideBalStatom, _ := stride.GetBalance(ctx, strideICAAddress, strideAtomIbcDenom)
			// print("\n stride bal: ", strideBal)
		})

		t.Run("manual autopilot", func(t *testing.T) {

			memo := fmt.Sprintf(
				`"{"autopilot":{"receiver":"%s","stakeibc":{"stride_address":"%s","action":"LiquidStake","ibc_receiver":"%s","transfer_channel":"%s"}}}`,
				strideICAAddress, strideICAAddress, lperContractAddress, strideNeutronChannelId)

			cmd := []string{"gaiad", "tx", "ibc-transfer", "transfer", "transfer",
				gaiaStrideChannelId,
				memo,
				"100uatom",
				"--from", gaiaUser.KeyName,
				"--gas", "auto",
				"--gas-adjustment", `1.3`,
				"--output", "json",
				"--node", atom.GetRPCAddress(),
				"--home", atom.HomeDir(),
				"--chain-id", atom.Config().ChainID,
				"--fees", "10uatom",
				"--keyring-backend", keyring.BackendTest,
				"-y",
			}

			print("\n cmd: \n")
			print(strings.Join(cmd, " "))
			_, _, err = cosmosAtom.Exec(ctx, cmd, nil)
			require.NoError(t, err)

			// print("\n", cmd, "\n")
			// _, _, err = atom.Exec(ctx, cmd, nil)
			// require.NoError(t, err)

			// ibcTx, _ := atom.SendIBCTransfer(
			// 	ctx,
			// 	gaiaStrideTransferChannel.ChannelID,
			// 	gaiaUser.KeyName,
			// 	ibc.WalletAmount{
			// 		Address: memo,
			// 		Denom:   atom.Config().Denom,
			// 		Amount:  100,
			// 	},
			// 	ibc.TransferOptions{
			// 		Timeout: &ibc.IBCTimeout{
			// 			Height: 700,
			// 		},
			// 		Memo: "",
			// 	})

			// str, _ := json.Marshal(ibcTx)
			// print(string(str))

			err = testutil.WaitForBlocks(ctx, 200, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			// strideBal, err := stride.GetBalance(ctx, strideICAAddress, strideAtomIbcDenom)
			// require.Equal(t, 100, strideBal)
			// require.NoError(t, err, "failed to wait for blocks")

			err = testutil.WaitForBlocks(ctx, 10, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")
		})

		t.Run("second tick liquid stakes on stride", func(t *testing.T) {
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

			err = testutil.WaitForBlocks(ctx, 20, atom, neutron, stride)
			require.NoError(t, err, "failed to wait for blocks")

			atomICABal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to query ICA balance")
			require.Equal(t, int64(10), atomICABal)
		})

		t.Run("third tick ibc transfers atom from ICA account to neutron", func(t *testing.T) {
			atomBal, err := atom.GetBalance(ctx, icaAccountAddress, atom.Config().Denom)
			require.NoError(t, err, "failed to get ICA balance")
			require.EqualValues(t, 10, atomBal)

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
			require.Equal(t, int64(0), atomICABal)

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
