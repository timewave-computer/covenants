package ibc_test

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"testing"

	"github.com/cosmos/cosmos-sdk/crypto/keyring"
	transfertypes "github.com/cosmos/ibc-go/v4/modules/apps/transfer/types"
	"github.com/strangelove-ventures/interchaintest/v4/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v4/ibc"
	"github.com/strangelove-ventures/interchaintest/v4/testreporter"
	"github.com/strangelove-ventures/interchaintest/v4/testutil"
	"github.com/stretchr/testify/require"
)

type TestContext struct {
	Neutron                   *cosmos.CosmosChain
	Hub                       *cosmos.CosmosChain
	Osmosis                   *cosmos.CosmosChain
	OsmoClients               []*ibc.ClientOutput
	GaiaClients               []*ibc.ClientOutput
	NeutronClients            []*ibc.ClientOutput
	OsmoConnections           []*ibc.ConnectionOutput
	GaiaConnections           []*ibc.ConnectionOutput
	NeutronConnections        []*ibc.ConnectionOutput
	NeutronTransferChannelIds map[string]string
	GaiaTransferChannelIds    map[string]string
	OsmoTransferChannelIds    map[string]string
	GaiaIcsChannelIds         map[string]string
	NeutronIcsChannelIds      map[string]string
	t                         *testing.T
	ctx                       context.Context
}

func (testCtx *TestContext) tick(clock string, keyring string, from string) {
	neutronHeight, _ := testCtx.Neutron.Height(testCtx.ctx)
	println("tick neutron@", neutronHeight)
	cmd := []string{"neutrond", "tx", "wasm", "execute", clock,
		`{"tick":{}}`,
		"--gas-prices", "0.0untrn",
		"--gas-adjustment", `1.5`,
		"--output", "json",
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--home", testCtx.Neutron.HomeDir(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
		"--from", from,
		"--gas", "1500000",
		"--keyring-backend", keyring,
		"-y",
	}

	tickResponse, _, err := testCtx.Neutron.Exec(testCtx.ctx, cmd, nil)
	require.NoError(testCtx.t, err)
	println("tick response: ", string(tickResponse))
	println("\n")
	testCtx.skipBlocks(3)
}

func (testCtx *TestContext) queryClockAddress(contract string) string {
	var response CovenantAddressQueryResponse

	err := testCtx.Neutron.QueryContract(testCtx.ctx, contract, ClockAddressQuery{}, &response)
	require.NoError(
		testCtx.t,
		err,
		"failed to query clock address",
	)
	println("clock address: ", response.Data)
	return response.Data
}

func (testCtx *TestContext) queryHolderAddress(contract string) string {
	var response CovenantAddressQueryResponse

	err := testCtx.Neutron.QueryContract(testCtx.ctx, contract, HolderAddressQuery{}, &response)
	require.NoError(
		testCtx.t,
		err,
		"failed to query holder address",
	)
	println("holder address: ", response.Data)
	return response.Data
}

func (testCtx *TestContext) queryLiquidPoolerAddress(contract string) string {
	var response CovenantAddressQueryResponse

	err := testCtx.Neutron.QueryContract(testCtx.ctx, contract, LiquidPoolerQuery{}, &response)
	require.NoError(
		testCtx.t,
		err,
		"failed to query liquid pooler address",
	)
	println("liquid pooler address: ", response.Data)
	return response.Data
}

func (testCtx *TestContext) queryIbcForwarderAddress(contract string, party string) string {
	var response CovenantAddressQueryResponse
	query := IbcForwarderQuery{
		Party: Party{
			Party: party,
		},
	}
	err := testCtx.Neutron.QueryContract(testCtx.ctx, contract, query, &response)
	require.NoError(
		testCtx.t,
		err,
		"failed to query ibc forwarder address",
	)
	println(party, " ibc forwarder address: ", response.Data)
	return response.Data
}

func (testCtx *TestContext) queryInterchainRouterAddress(contract string, party string) string {
	var response CovenantAddressQueryResponse
	query := InterchainRouterQuery{
		Party: Party{
			Party: party,
		},
	}
	err := testCtx.Neutron.QueryContract(testCtx.ctx, contract, query, &response)
	require.NoError(
		testCtx.t,
		err,
		"failed to query interchain router address",
	)
	println(party, " interchain router address: ", response.Data)
	return response.Data
}

func (testCtx *TestContext) queryContractState(contract string) string {
	var response CovenantAddressQueryResponse
	type ContractState struct{}
	type ContractStateQuery struct {
		ContractState ContractState `json:"contract_state"`
	}
	contractStateQuery := ContractStateQuery{
		ContractState: ContractState{},
	}

	err := testCtx.Neutron.QueryContract(testCtx.ctx, contract, contractStateQuery, &response)
	require.NoError(
		testCtx.t,
		err,
		fmt.Sprintf("failed to query %s state", contract),
	)
	return response.Data
}

func (testCtx *TestContext) queryDepositAddress(covenant string, party string) string {
	var depositAddressResponse CovenantAddressQueryResponse

	type PartyDepositAddress struct {
		Party string `json:"party"`
	}
	type PartyDepositAddressQuery struct {
		PartyDepositAddress PartyDepositAddress `json:"party_deposit_address"`
	}
	depositAddressQuery := PartyDepositAddressQuery{
		PartyDepositAddress: PartyDepositAddress{
			Party: party,
		},
	}

	err := testCtx.Neutron.QueryContract(testCtx.ctx, covenant, depositAddressQuery, &depositAddressResponse)
	require.NoError(
		testCtx.t,
		err,
		fmt.Sprintf("failed to query %s deposit address", party),
	)
	println(party, " deposit address: ", depositAddressResponse.Data)
	return depositAddressResponse.Data
}

func (testCtx *TestContext) holderClaim(contract string, from *ibc.Wallet, keyring string) {

	cmd := []string{"neutrond", "tx", "wasm", "execute", contract,
		`{"claim":{}}`,
		"--from", from.GetKeyName(),
		"--gas-prices", "0.0untrn",
		"--gas-adjustment", `1.5`,
		"--output", "json",
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--home", testCtx.Neutron.HomeDir(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
		"--gas", "42069420",
		"--keyring-backend", keyring,
		"-y",
	}

	resp, _, err := testCtx.Neutron.Exec(testCtx.ctx, cmd, nil)
	require.NoError(testCtx.t, err, "claim failed")
	println("claim response: ", string(resp))
	require.NoError(testCtx.t,
		testutil.WaitForBlocks(testCtx.ctx, 2, testCtx.Hub, testCtx.Neutron, testCtx.Osmosis))

}

func (testCtx *TestContext) holderRagequit(contract string, from *ibc.Wallet, keyring string) {

	cmd := []string{"neutrond", "tx", "wasm", "execute", contract,
		`{"ragequit":{}}`,
		"--from", from.GetKeyName(),
		"--gas-prices", "0.0untrn",
		"--gas-adjustment", `1.5`,
		"--output", "json",
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--home", testCtx.Neutron.HomeDir(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
		"--gas", "42069420",
		"--keyring-backend", keyring,
		"-y",
	}

	_, _, err := testCtx.Neutron.Exec(testCtx.ctx, cmd, nil)
	require.NoError(testCtx.t, err, "ragequit failed")
}

func (testCtx *TestContext) getNeutronHeight() uint64 {
	currentHeight, err := testCtx.Neutron.Height(testCtx.ctx)
	require.NoError(testCtx.t, err, "failed to get neutron height")
	println("neutron height: ", currentHeight)
	return currentHeight
}

func (testCtx *TestContext) manualInstantiate(codeId uint64, msg CovenantInstantiateMsg, from *ibc.Wallet, keyring string) string {
	codeIdStr := strconv.FormatUint(codeId, 10)

	str, err := json.Marshal(msg)
	require.NoError(testCtx.t, err, "Failed to marshall CovenantInstantiateMsg")
	instantiateMsg := string(str)

	cmd := []string{"neutrond", "tx", "wasm", "instantiate", codeIdStr,
		instantiateMsg,
		"--label", fmt.Sprintf("two-party-pol-covenant-%s", codeIdStr),
		"--no-admin",
		"--from", from.KeyName,
		"--output", "json",
		"--home", testCtx.Neutron.HomeDir(),
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
		"--gas", "900090000",
		"--keyring-backend", keyring,
		"-y",
	}

	prettyJson, _ := json.MarshalIndent(msg, "", "    ")
	println("covenant instantiation message:")
	fmt.Println(string(prettyJson))

	covInstantiationResp, _, err := testCtx.Neutron.Exec(testCtx.ctx, cmd, nil)
	require.NoError(testCtx.t, err, "manual instantiation failed")
	println("covenant instantiation response: ", string(covInstantiationResp))
	require.NoError(testCtx.t,
		testutil.WaitForBlocks(testCtx.ctx, 5, testCtx.Hub, testCtx.Neutron, testCtx.Osmosis))

	queryCmd := []string{"neutrond", "query", "wasm",
		"list-contract-by-code", codeIdStr,
		"--output", "json",
		"--home", testCtx.Neutron.HomeDir(),
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
	}

	queryResp, _, err := testCtx.Neutron.Exec(testCtx.ctx, queryCmd, nil)
	require.NoError(testCtx.t, err, "failed to query")

	type QueryContractResponse struct {
		Contracts  []string `json:"contracts"`
		Pagination any      `json:"pagination"`
	}

	contactsRes := QueryContractResponse{}
	require.NoError(testCtx.t, json.Unmarshal(queryResp, &contactsRes), "failed to unmarshal contract response")

	covenantAddress := contactsRes.Contracts[len(contactsRes.Contracts)-1]

	return covenantAddress
}

func (testCtx *TestContext) fundChainAddrs(addrs []string, chain *cosmos.CosmosChain, from *ibc.Wallet, amount int64) {
	for i := 0; i < len(addrs); i++ {
		err := chain.SendFunds(testCtx.ctx, from.KeyName, ibc.WalletAmount{
			Address: addrs[i],
			Denom:   chain.Config().Denom,
			Amount:  int64(amount),
		})
		require.NoError(testCtx.t, err, "failed to send funds to addr")
	}
}

func (testCtx *TestContext) storeContract(chain *cosmos.CosmosChain, from *ibc.Wallet, path string) uint64 {
	codeIdStr, err := chain.StoreContract(testCtx.ctx, from.KeyName, path)
	require.NoError(testCtx.t, err, "failed to store contract")
	codeId, err := strconv.ParseUint(codeIdStr, 10, 64)
	require.NoError(testCtx.t, err, "failed to parse codeId")
	return codeId
}

func (testCtx *TestContext) instantiateAstroportFactory(pairCodeId uint64, tokenCodeId uint64, whitelistCodeId uint64, factoryCodeId uint64, coinRegistryAddr string, from *ibc.Wallet) string {
	msg := FactoryInstantiateMsg{
		PairConfigs: []PairConfig{
			PairConfig{
				CodeId: pairCodeId,
				PairType: PairType{
					Stable: struct{}{},
				},
				TotalFeeBps:         0,
				MakerFeeBps:         0,
				IsDisabled:          false,
				IsGeneratorDisabled: true,
			},
		},
		TokenCodeId:         tokenCodeId,
		FeeAddress:          nil,
		GeneratorAddress:    nil,
		Owner:               from.Bech32Address(testCtx.Neutron.Config().Bech32Prefix),
		WhitelistCodeId:     whitelistCodeId,
		CoinRegistryAddress: coinRegistryAddr,
	}

	str, _ := json.Marshal(msg)
	factoryAddr, err := testCtx.Neutron.InstantiateContract(
		testCtx.ctx, from.KeyName, strconv.FormatUint(factoryCodeId, 10), string(str), true)
	require.NoError(testCtx.t, err, "Failed to instantiate Factory")

	return factoryAddr
}

func (testCtx *TestContext) queryAstroLpTokenAndStableswapAddress(
	factoryAddress string, denom1 string, denom2 string,
) (string, string) {
	pairQueryMsg := PairQuery{
		Pair: Pair{
			AssetInfos: []AssetInfo{
				{
					NativeToken: &NativeToken{
						Denom: denom1,
					},
				},
				{
					NativeToken: &NativeToken{
						Denom: denom2,
					},
				},
			},
		},
	}
	queryJson, _ := json.Marshal(pairQueryMsg)
	queryCmd := []string{"neutrond", "query", "wasm", "contract-state", "smart",
		factoryAddress, string(queryJson),
	}

	factoryQueryRespBytes, _, _ := testCtx.Neutron.Exec(testCtx.ctx, queryCmd, nil)
	print(string(factoryQueryRespBytes))

	var response FactoryPairResponse
	err := testCtx.Neutron.QueryContract(testCtx.ctx, factoryAddress, pairQueryMsg, &response)
	require.NoError(testCtx.t, err, "failed to query pair info")

	stableswapAddress := response.Data.ContractAddr
	print("\n stableswap address: ", stableswapAddress, "\n")
	liquidityTokenAddress := response.Data.LiquidityToken
	print("\n liquidity token: ", liquidityTokenAddress, "\n")

	return liquidityTokenAddress, stableswapAddress
}

func (testCtx *TestContext) provideAstroportLiquidity(denom1 string, denom2 string, amount1 uint64, amount2 uint64, from *ibc.Wallet, pool string) {

	msg := ProvideLiqudityMsg{
		ProvideLiquidity: ProvideLiquidityStruct{
			Assets: []AstroportAsset{
				AstroportAsset{
					Info: AssetInfo{
						NativeToken: &NativeToken{
							Denom: denom1,
						},
					},
					Amount: strconv.FormatUint(amount1, 10),
				},
				AstroportAsset{
					Info: AssetInfo{
						NativeToken: &NativeToken{
							Denom: denom2,
						},
					},
					Amount: strconv.FormatUint(amount2, 10),
				},
			},
			SlippageTolerance: "0.01",
			AutoStake:         false,
			Receiver:          from.Bech32Address(testCtx.Neutron.Config().Bech32Prefix),
		},
	}

	str, err := json.Marshal(msg)
	require.NoError(testCtx.t, err, "Failed to marshall provide liquidity msg")
	amountStr := strconv.FormatUint(amount1, 10) + denom1 + "," + strconv.FormatUint(amount2, 10) + denom2

	cmd := []string{"neutrond", "tx", "wasm", "execute", pool,
		string(str),
		"--from", from.KeyName,
		"--amount", amountStr,
		"--output", "json",
		"--home", testCtx.Neutron.HomeDir(),
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
		"--gas", "900000",
		"--keyring-backend", keyring.BackendTest,
		"-y",
	}
	println("liq provision msg: \n ", strings.Join(cmd, " "), "\n")

	_, _, err = testCtx.Neutron.Exec(testCtx.ctx, cmd, nil)
	require.NoError(testCtx.t, err)
}

func (testCtx *TestContext) createAstroportFactoryPair(amp uint64, denom1 string, denom2 string, factory string, from *ibc.Wallet, keyring string) {
	initParams := StablePoolParams{
		Amp: amp,
	}
	binaryData, _ := json.Marshal(initParams)

	createPairMsg := CreatePairMsg{
		CreatePair: CreatePair{
			PairType: PairType{
				Stable: struct{}{},
			},
			AssetInfos: []AssetInfo{
				{
					NativeToken: &NativeToken{
						Denom: denom1,
					},
				},
				{
					NativeToken: &NativeToken{
						Denom: denom2,
					},
				},
			},
			InitParams: binaryData,
		},
	}

	str, _ := json.Marshal(createPairMsg)

	createCmd := []string{"neutrond", "tx", "wasm", "execute",
		factory,
		string(str),
		"--gas-prices", "0.0untrn",
		"--gas-adjustment", `1.5`,
		"--output", "json",
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--home", testCtx.Neutron.HomeDir(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
		"--from", from.KeyName,
		"--gas", "auto",
		"--keyring-backend", keyring,
		"-y",
	}

	_, _, err := testCtx.Neutron.Exec(testCtx.ctx, createCmd, nil)
	require.NoError(testCtx.t, err, err)
	testCtx.skipBlocks(3)
}

func (testCtx *TestContext) skipBlocks(n uint64) {
	require.NoError(
		testCtx.t,
		testutil.WaitForBlocks(testCtx.ctx, 3, testCtx.Hub, testCtx.Neutron, testCtx.Osmosis),
		"failed to wait for blocks")
}

func (testCtx *TestContext) queryLpTokenBalance(token string, addr string) uint64 {
	bal := Balance{
		Address: addr,
	}

	balanceQueryMsg := Cw20QueryMsg{
		Balance: bal,
	}
	var response Cw20BalanceResponse
	require.NoError(
		testCtx.t,
		testCtx.Neutron.QueryContract(testCtx.ctx, token, balanceQueryMsg, &response),
		"failed to query lp token balance",
	)
	balString := response.Data.Balance

	lpBal, err := strconv.ParseUint(balString, 10, 64)
	if err != nil {
		panic(err)
	}
	println(addr, " lp token balance: ", lpBal)
	return lpBal
}

func (testCtx *TestContext) queryNeutronDenomBalance(denom string, addr string) uint64 {
	queryCmd := []string{"neutrond", "query", "bank",
		"balances", addr,
		"--denom", denom,
		"--output", "json",
		"--home", testCtx.Neutron.HomeDir(),
		"--node", testCtx.Neutron.GetRPCAddress(),
		"--chain-id", testCtx.Neutron.Config().ChainID,
	}
	var nativeBalanceResponse NativeBalQueryResponse

	queryResp, _, err := testCtx.Neutron.Exec(testCtx.ctx, queryCmd, nil)
	require.NoError(testCtx.t, err, "failed to query")

	require.NoError(
		testCtx.t,
		json.Unmarshal(queryResp, &nativeBalanceResponse),
		"failed to unmarshal json",
	)
	parsedBalance, err := strconv.ParseUint(nativeBalanceResponse.Amount, 10, 64)
	require.NoError(testCtx.t, err, "failed to parse balance response to uint64")
	return parsedBalance
}

func (testCtx *TestContext) queryHubDenomBalance(denom string, addr string) uint64 {
	bal, err := testCtx.Hub.GetBalance(testCtx.ctx, addr, denom)
	require.NoError(testCtx.t, err, "failed to get hub denom balance")

	uintBal := uint64(bal)
	return uintBal
}

func (testCtx *TestContext) queryOsmosisDenomBalance(denom string, addr string) uint64 {
	bal, err := testCtx.Osmosis.GetBalance(testCtx.ctx, addr, denom)
	require.NoError(testCtx.t, err, "failed to get osmosis denom balance")

	uintBal := uint64(bal)
	return uintBal
}

func (testCtx *TestContext) getIbcDenom(channelId string, denom string) string {
	prefixedDenom := transfertypes.GetPrefixedDenom("transfer", channelId, denom)
	srcDenomTrace := transfertypes.ParseDenomTrace(prefixedDenom)
	return srcDenomTrace.IBCDenom()
}

// channel trace should be an ordered list of the path denom would take,
// starting from the source chain, and ending on the destination chain.
// assumes "transfer" ports.
func (testCtx *TestContext) getMultihopIbcDenom(channelTrace []string, denom string) string {
	var portChannelTrace []string

	for _, channel := range channelTrace {
		portChannelTrace = append(portChannelTrace, fmt.Sprintf("%s/%s", "transfer", channel))
	}

	prefixedDenom := fmt.Sprintf("%s/%s", strings.Join(portChannelTrace, "/"), denom)

	denomTrace := transfertypes.ParseDenomTrace(prefixedDenom)
	return denomTrace.IBCDenom()

}

func (testCtx *TestContext) getChainClients(chain string) []*ibc.ClientOutput {
	switch chain {
	case "neutron-2":
		return testCtx.NeutronClients
	case "gaia-1":
		return testCtx.GaiaClients
	case "osmosis-3":
		return testCtx.OsmoClients
	default:
		return ibc.ClientOutputs{}
	}
}

func (testCtx *TestContext) setTransferChannelId(chain string, destChain string, channelId string) {
	switch chain {
	case "neutron-2":
		testCtx.NeutronTransferChannelIds[destChain] = channelId
	case "gaia-1":
		testCtx.GaiaTransferChannelIds[destChain] = channelId
	case "osmosis-3":
		testCtx.OsmoTransferChannelIds[destChain] = channelId
	default:
	}
}

func (testCtx *TestContext) setIcsChannelId(chain string, destChain string, channelId string) {
	switch chain {
	case "neutron-2":
		testCtx.NeutronIcsChannelIds[destChain] = channelId
	case "gaia-1":
		testCtx.GaiaIcsChannelIds[destChain] = channelId
	default:
	}
}

func (testCtx *TestContext) updateChainClients(chain string, clients []*ibc.ClientOutput) {
	switch chain {
	case "neutron-2":
		testCtx.NeutronClients = clients
	case "gaia-1":
		testCtx.GaiaClients = clients
	case "osmosis-3":
		testCtx.OsmoClients = clients
	default:
	}
}

func (testCtx *TestContext) getChainConnections(chain string) []*ibc.ConnectionOutput {
	switch chain {
	case "neutron-2":
		return testCtx.NeutronConnections
	case "gaia-1":
		return testCtx.GaiaConnections
	case "osmosis-3":
		return testCtx.OsmoConnections
	default:
		println("error finding connections for chain ", chain)
		return []*ibc.ConnectionOutput{}
	}
}

func (testCtx *TestContext) updateChainConnections(chain string, connections []*ibc.ConnectionOutput) {
	switch chain {
	case "neutron-2":
		testCtx.NeutronConnections = connections
	case "gaia-1":
		testCtx.GaiaConnections = connections
	case "osmosis-3":
		testCtx.OsmoConnections = connections
	default:
	}
}

func generatePath(
	t *testing.T,
	ctx context.Context,
	r ibc.Relayer,
	eRep *testreporter.RelayerExecReporter,
	chainAId string,
	chainBId string,
	path string,
) {
	err := r.GeneratePath(ctx, eRep, chainAId, chainBId, path)
	require.NoError(t, err)
}

func generateICSChannel(
	t *testing.T,
	ctx context.Context,
	r ibc.Relayer,
	eRep *testreporter.RelayerExecReporter,
	icsPath string,
	chainA ibc.Chain,
	chainB ibc.Chain,
) {

	err := r.CreateChannel(ctx, eRep, icsPath, ibc.CreateChannelOptions{
		SourcePortName: "consumer",
		DestPortName:   "provider",
		Order:          ibc.Ordered,
		Version:        "1",
	})
	require.NoError(t, err)
	err = testutil.WaitForBlocks(ctx, 2, chainA, chainB)
	require.NoError(t, err, "failed to wait for blocks")
}

func createValidator(
	t *testing.T,
	ctx context.Context,
	r ibc.Relayer,
	eRep *testreporter.RelayerExecReporter,
	chain ibc.Chain,
	counterparty ibc.Chain,
) {
	cmd := getCreateValidatorCmd(chain)
	_, _, err := chain.Exec(ctx, cmd, nil)
	require.NoError(t, err)

	// Wait a bit for the VSC packet to get relayed.
	err = testutil.WaitForBlocks(ctx, 2, chain, counterparty)
	require.NoError(t, err, "failed to wait for blocks")
}

func linkPath(
	t *testing.T,
	ctx context.Context,
	r ibc.Relayer,
	eRep *testreporter.RelayerExecReporter,
	chainA ibc.Chain,
	chainB ibc.Chain,
	path string,
) {
	err := r.LinkPath(ctx, eRep, path, ibc.DefaultChannelOpts(), ibc.DefaultClientOpts())
	require.NoError(t, err)
	err = testutil.WaitForBlocks(ctx, 2, chainA, chainB)
	require.NoError(t, err, "failed to wait for blocks")
}

func generateClient(
	t *testing.T,
	ctx context.Context,
	testCtx *TestContext,
	r ibc.Relayer,
	eRep *testreporter.RelayerExecReporter,
	path string,
	chainA ibc.Chain,
	chainB ibc.Chain,
) (string, string) {
	chainAClients := testCtx.getChainClients(chainA.Config().Name)
	chainBClients := testCtx.getChainClients(chainB.Config().Name)

	err := r.CreateClients(ctx, eRep, path, ibc.CreateClientOptions{TrustingPeriod: "330h"})
	require.NoError(t, err)
	err = testutil.WaitForBlocks(ctx, 2, chainA, chainB)
	require.NoError(t, err, "failed to wait for blocks")

	newChainAClients, _ := r.GetClients(ctx, eRep, chainA.Config().ChainID)
	newChainBClients, _ := r.GetClients(ctx, eRep, chainB.Config().ChainID)
	var newClientA, newClientB string

	aClientDiff := clientDifference(chainAClients, newChainAClients)
	bClientDiff := clientDifference(chainBClients, newChainBClients)

	if len(aClientDiff) > 0 {
		newClientA = aClientDiff[0]
	} else {
		newClientA = ""
	}

	if len(bClientDiff) > 0 {
		newClientB = bClientDiff[0]
	} else {
		newClientB = ""
	}

	testCtx.updateChainClients(chainA.Config().Name, newChainAClients)
	testCtx.updateChainClients(chainB.Config().Name, newChainBClients)

	return newClientA, newClientB
}

func generateConnections(
	t *testing.T,
	ctx context.Context,
	testCtx *TestContext,
	r ibc.Relayer,
	eRep *testreporter.RelayerExecReporter,
	path string,
	chainA ibc.Chain,
	chainB ibc.Chain,
) (string, string) {
	chainAConns := testCtx.getChainConnections(chainA.Config().Name)
	chainBConns := testCtx.getChainConnections(chainB.Config().Name)

	err := r.CreateConnections(ctx, eRep, path)
	require.NoError(t, err)
	err = testutil.WaitForBlocks(ctx, 2, chainA, chainB)
	require.NoError(t, err, "failed to wait for blocks")

	newChainAConns, _ := r.GetConnections(ctx, eRep, chainA.Config().ChainID)
	newChainBConns, _ := r.GetConnections(ctx, eRep, chainB.Config().ChainID)

	newChainAConnection := connectionDifference(chainAConns, newChainAConns)
	newChainBConnection := connectionDifference(chainBConns, newChainBConns)

	require.NotEqual(t, 0, len(newChainAConnection), "more than one connection generated", strings.Join(newChainAConnection, " "))
	require.NotEqual(t, 0, len(newChainBConnection), "more than one connection generated", strings.Join(newChainBConnection, " "))

	testCtx.updateChainConnections(chainA.Config().Name, newChainAConns)
	testCtx.updateChainConnections(chainB.Config().Name, newChainBConns)

	return newChainAConnection[0], newChainBConnection[0]
}

func connectionDifference(a, b []*ibc.ConnectionOutput) (diff []string) {
	m := make(map[string]bool)

	// we first mark all existing connections
	for _, item := range a {
		m[item.ID] = true
	}

	// and append all new ones
	for _, item := range b {
		if _, ok := m[item.ID]; !ok {
			diff = append(diff, item.ID)
		}
	}
	return
}

func clientDifference(a, b []*ibc.ClientOutput) (diff []string) {
	m := make(map[string]bool)

	// we first mark all existing clients
	for _, item := range a {
		m[item.ClientID] = true
	}

	// and append all new ones
	for _, item := range b {
		if _, ok := m[item.ClientID]; !ok {
			diff = append(diff, item.ClientID)
		}
	}
	return
}

func printChannels(channels []ibc.ChannelOutput, chain string) {
	for _, channel := range channels {
		print("\n\n", chain, " channels after create channel :", channel.ChannelID, " to ", channel.Counterparty.ChannelID, "\n")
	}
}

func printConnections(connections ibc.ConnectionOutputs) {
	for _, connection := range connections {
		print(connection.ID, "\n")
	}
}

func channelDifference(oldChannels, newChannels []ibc.ChannelOutput) (diff []string) {
	m := make(map[string]bool)
	// we first mark all existing channels
	for _, channel := range newChannels {
		m[channel.ChannelID] = true
	}

	// then find the new ones
	for _, channel := range oldChannels {
		if _, ok := m[channel.ChannelID]; !ok {
			diff = append(diff, channel.ChannelID)
		}
	}

	return
}

func getPairwiseConnectionIds(
	aconns ibc.ConnectionOutputs,
	bconns ibc.ConnectionOutputs,
) ([]string, []string, error) {
	abconnids := make([]string, 0)
	baconnids := make([]string, 0)
	found := false
	for _, a := range aconns {
		for _, b := range bconns {
			if a.ClientID == b.Counterparty.ClientId &&
				b.ClientID == a.Counterparty.ClientId &&
				a.ID == b.Counterparty.ConnectionId &&
				b.ID == a.Counterparty.ConnectionId {
				found = true
				abconnids = append(abconnids, a.ID)
				baconnids = append(baconnids, b.ID)
			}
		}
	}
	if found {
		return abconnids, baconnids, nil
	} else {
		return abconnids, baconnids, errors.New("no connection found")
	}
}

// returns transfer channel ids
func getPairwiseTransferChannelIds(
	testCtx *TestContext,
	achans []ibc.ChannelOutput,
	bchans []ibc.ChannelOutput,
	aToBConnId string,
	bToAConnId string,
	chainA string,
	chainB string,
) (string, string) {

	for _, a := range achans {
		for _, b := range bchans {
			if a.ChannelID == b.Counterparty.ChannelID &&
				b.ChannelID == a.Counterparty.ChannelID &&
				a.PortID == "transfer" &&
				b.PortID == "transfer" &&
				a.Ordering == "ORDER_UNORDERED" &&
				b.Ordering == "ORDER_UNORDERED" &&
				a.ConnectionHops[0] == aToBConnId &&
				b.ConnectionHops[0] == bToAConnId {
				testCtx.setTransferChannelId(chainA, chainB, a.ChannelID)
				testCtx.setTransferChannelId(chainB, chainA, b.ChannelID)
				return a.ChannelID, b.ChannelID
			}
		}
	}
	panic("failed to match pairwise transfer channels")
}

// returns ccv channel ids
func getPairwiseCCVChannelIds(
	testCtx *TestContext,
	achans []ibc.ChannelOutput,
	bchans []ibc.ChannelOutput,
	aToBConnId string,
	bToAConnId string,
	chainA string,
	chainB string,
) (string, string) {
	for _, a := range achans {
		for _, b := range bchans {
			if a.ChannelID == b.Counterparty.ChannelID &&
				b.ChannelID == a.Counterparty.ChannelID &&
				a.PortID == "provider" &&
				b.PortID == "consumer" &&
				a.Ordering == "ORDER_ORDERED" &&
				b.Ordering == "ORDER_ORDERED" &&
				a.ConnectionHops[0] == aToBConnId &&
				b.ConnectionHops[0] == bToAConnId {
				testCtx.setIcsChannelId(chainA, chainB, a.ChannelID)
				testCtx.setIcsChannelId(chainB, chainA, b.ChannelID)
				return a.ChannelID, b.ChannelID
			}
		}
	}
	panic("failed to match pairwise ICS channels")
}
