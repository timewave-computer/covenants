package ibc_test

//////////////////////////////////////////////
///// Covenant contracts
//////////////////////////////////////////////

// ----- Covenant Instantiation ------
type CovenantInstantiateMsg struct {
	Label                  string                 `json:"label"`
	Timeouts               Timeouts               `json:"timeouts"`
	IbcForwarderCode       uint64                 `json:"ibc_forwarder_code"`
	InterchainRouterCode   uint64                 `json:"interchain_router_code"`
	InterchainSplitterCode uint64                 `json:"splitter_code"`
	PresetClock            PresetClockFields      `json:"preset_clock_fields"`
	PresetSwapHolder       PresetSwapHolderFields `json:"preset_holder_fields"`
	// SwapCovenantTerms      SwapCovenantTerms      `json:"covenant_terms"`
	SwapCovenantParties SwapCovenantParties `json:"covenant_parties"`
}

type Timeouts struct {
	IcaTimeout         string `json:"ica_timeout"`
	IbcTransferTimeout string `json:"ibc_transfer_timeout"`
}

type PresetIbcFee struct {
	AckFee     string `json:"ack_fee"`
	TimeoutFee string `json:"timeout_fee"`
}

type PresetClockFields struct {
	TickMaxGas string   `json:"tick_max_gas,omitempty"`
	ClockCode  uint64   `json:"clock_code"`
	Label      string   `json:"label"`
	Whitelist  []string `json:"whitelist"`
}

type PresetSwapHolderFields struct {
	LockupConfig          LockupConfig          `json:"lockup_config"`
	CovenantPartiesConfig CovenantPartiesConfig `json:"parties_config"`
	CovenantTerms         CovenantTerms         `json:"covenant_terms"`
	CodeId                uint64                `json:"code_id"`
	Label                 string                `json:"label"`
}

type Timestamp string

type LockupConfig struct {
	None  bool       `json:"none,omitempty"`
	Block *uint64    `json:"block,omitempty"`
	Time  *Timestamp `json:"time,omitempty"`
}

type CovenantPartiesConfig struct {
	PartyA CovenantParty `json:"party_a"`
	PartyB CovenantParty `json:"party_b"`
}

type SwapCovenantParties struct {
	PartyA SwapPartyConfig `json:"party_a"`
	PartyB SwapPartyConfig `json:"party_b"`
}

type CovenantParty struct {
	Addr           string         `json:"addr"`
	ProvidedDenom  string         `json:"provided_denom"`
	ReceiverConfig ReceiverConfig `json:"receiver_config"`
}

type SwapPartyConfig struct {
	Addr                   string `json:"addr"`
	ProvidedDenom          string `json:"provided_denom"`
	PartyChainChannelId    string `json:"party_chain_channel_id"`
	PartyReceiverAddr      string `json:"party_receiver_addr"`
	PartyChainConnectionId string `json:"party_chain_connection_id"`
	IbcTransferTimeout     string `json:"ibc_transfer_timeout"`
}

type ReceiverConfig struct {
	Native string `json:"native"`
}

type SwapCovenantTerms struct {
	PartyAAmount string `json:"party_a_amount"`
	PartyBAmount string `json:"party_b_amount"`
}

type CovenantTerms struct {
	TokenSwap SwapCovenantTerms `json:"token_swap,omitempty"`
}

// ----- Covenant Queries ------

type ClockAddress struct{}
type ClockAddressQuery struct {
	ClockAddress ClockAddress `json:"clock_address"`
}

type ContractState struct{}
type ContractStateQuery struct {
	ContractState ContractState `json:"contract_state"`
}

type ContractStateQueryResponse struct {
	Data string `json:"data"`
}

//////////////////////////////////////////////
///// Depositor contract
//////////////////////////////////////////////

// Instantiation
type WeightedReceiver struct {
	Amount  string `json:"amount"`
	Address string `json:"address"`
}

type WeightedReceiverAmount struct {
	Amount string `json:"amount"`
}

type StAtomWeightedReceiverQuery struct {
	StAtomReceiver StAtomReceiverQuery `json:"st_atom_receiver"`
}

type AtomWeightedReceiverQuery struct {
	AtomReceiver AtomReceiverQuery `json:"atom_receiver"`
}

type StAtomReceiverQuery struct{}
type AtomReceiverQuery struct{}

type WeightedReceiverResponse struct {
	Data WeightedReceiver `json:"data"`
}

// Queries
type DepositorICAAddressQuery struct {
	DepositorInterchainAccountAddress DepositorInterchainAccountAddress `json:"depositor_interchain_account_address"`
}
type DepositorInterchainAccountAddress struct{}

type QueryResponse struct {
	Data InterchainAccountAddressQueryResponse `json:"data"`
}

type InterchainAccountAddressQueryResponse struct {
	InterchainAccountAddress string `json:"interchain_account_address"`
}

//////////////////////////////////////////////
///// Ls contract
//////////////////////////////////////////////

// Execute
type TransferExecutionMsg struct {
	Transfer TransferAmount `json:"transfer"`
}

// Rust type here is Uint128 which can't safely be serialized
// to json int. It needs to go as a string over the wire.
type TransferAmount struct {
	Amount uint64 `json:"amount,string"`
}

// Queries
type LsIcaQuery struct {
	StrideIca StrideIcaQuery `json:"stride_i_c_a"`
}
type StrideIcaQuery struct{}

type StrideIcaQueryResponse struct {
	Addr string `json:"data"`
}

//////////////////////////////////////////////
///// Lp contract
//////////////////////////////////////////////

type LPPositionQuery struct {
	LpPosition LpPositionQuery `json:"lp_position"`
}
type LpPositionQuery struct{}

type PairInfo struct {
	LiquidityToken string      `json:"liquidity_token"`
	ContractAddr   string      `json:"contract_addr"`
	PairType       PairType    `json:"pair_type"`
	AssetInfos     []AssetInfo `json:"asset_infos"`
}

type Pair struct {
	AssetInfos []AssetInfo `json:"asset_infos"`
}

type PairQuery struct {
	Pair Pair `json:"pair"`
}

type CreatePair struct {
	PairType   PairType    `json:"pair_type"`
	AssetInfos []AssetInfo `json:"asset_infos"`
	InitParams []byte      `json:"init_params"`
}

type CreatePairMsg struct {
	CreatePair CreatePair `json:"create_pair"`
}

//////////////////////////////////////////////
///// Holder contract
//////////////////////////////////////////////

type CovenantHolderAddressQuery struct {
	Addr string `json:"address"`
}

type WithdrawLiquidityMessage struct {
	WithdrawLiquidity WithdrawLiquidity `json:"withdraw_liquidity"`
}

type WithdrawLiquidity struct{}

type WithdrawMessage struct {
	Withdraw Withdraw `json:"withdraw"`
}

type Withdraw struct {
	Quantity *[]CwCoin `json:"quantity"`
}

//////////////////////////////////////////////
///// Astroport contracts
//////////////////////////////////////////////

// astroport stableswap
type StableswapInstantiateMsg struct {
	TokenCodeId uint64      `json:"token_code_id"`
	FactoryAddr string      `json:"factory_addr"`
	AssetInfos  []AssetInfo `json:"asset_infos"`
	InitParams  []byte      `json:"init_params"`
}

type AssetInfo struct {
	Token       *Token       `json:"token,omitempty"`
	NativeToken *NativeToken `json:"native_token,omitempty"`
}

type StablePoolParams struct {
	Amp   uint64  `json:"amp"`
	Owner *string `json:"owner"`
}

type Token struct {
	ContractAddr string `json:"contract_addr"`
}

type NativeToken struct {
	Denom string `json:"denom"`
}

type CwCoin struct {
	Denom  string `json:"denom"`
	Amount uint64 `json:"amount"`
}

// astroport factory
type FactoryInstantiateMsg struct {
	PairConfigs         []PairConfig `json:"pair_configs"`
	TokenCodeId         uint64       `json:"token_code_id"`
	FeeAddress          *string      `json:"fee_address"`
	GeneratorAddress    *string      `json:"generator_address"`
	Owner               string       `json:"owner"`
	WhitelistCodeId     uint64       `json:"whitelist_code_id"`
	CoinRegistryAddress string       `json:"coin_registry_address"`
}

type PairConfig struct {
	CodeId              uint64   `json:"code_id"`
	PairType            PairType `json:"pair_type"`
	TotalFeeBps         uint64   `json:"total_fee_bps"`
	MakerFeeBps         uint64   `json:"maker_fee_bps"`
	IsDisabled          bool     `json:"is_disabled"`
	IsGeneratorDisabled bool     `json:"is_generator_disabled"`
}

type PairType struct {
	// Xyk    struct{} `json:"xyk,omitempty"`
	Stable struct{} `json:"stable,omitempty"`
	// Custom struct{} `json:"custom,omitempty"`
}

// astroport native coin registry

type NativeCoinRegistryInstantiateMsg struct {
	Owner string `json:"owner"`
}

type AddExecuteMsg struct {
	Add Add `json:"add"`
}

type Add struct {
	NativeCoins []NativeCoin `json:"native_coins"`
}

type NativeCoin struct {
	Name  string `json:"name"`
	Value uint8  `json:"value"`
}

// Add { native_coins: Vec<(String, u8)> },

// astroport native token
type NativeTokenInstantiateMsg struct {
	Name            string                    `json:"name"`
	Symbol          string                    `json:"symbol"`
	Decimals        uint8                     `json:"decimals"`
	InitialBalances []Cw20Coin                `json:"initial_balances"`
	Mint            *MinterResponse           `json:"mint"`
	Marketing       *InstantiateMarketingInfo `json:"marketing"`
}

type Cw20Coin struct {
	Address string `json:"address"`
	Amount  uint64 `json:"amount"`
}

type MinterResponse struct {
	Minter string  `json:"minter"`
	Cap    *uint64 `json:"cap,omitempty"`
}

type InstantiateMarketingInfo struct {
	Project     string `json:"project"`
	Description string `json:"description"`
	Marketing   string `json:"marketing"`
	Logo        Logo   `json:"logo"`
}

type Logo struct {
	Url string `json:"url"`
}

// astroport whitelist
type WhitelistInstantiateMsg struct {
	Admins  []string `json:"admins"`
	Mutable bool     `json:"mutable"`
}

type ProvideLiqudityMsg struct {
	ProvideLiquidity ProvideLiquidityStruct `json:"provide_liquidity"`
}

type ProvideLiquidityStruct struct {
	Assets            []AstroportAsset `json:"assets"`
	SlippageTolerance string           `json:"slippage_tolerance"`
	AutoStake         bool             `json:"auto_stake"`
	Receiver          string           `json:"receiver"`
}

// factory

type FactoryPairResponse struct {
	Data PairInfo `json:"data"`
}

/////////////////////////////////////////////////////////////////////
//--- These are here for debugging but should be likely removed ---//

type CovenantClockAddressQuery struct {
	Addr string `json:"address"`
}

type DepositorContractQuery struct {
	ClockAddress ClockAddressQuery `json:"clock_address"`
}

type LPContractQuery struct {
	ClockAddress ClockAddressQuery `json:"clock_address"`
}

type ClockQueryResponse struct {
	Data string `json:"data"`
}

type LpPositionQueryResponse struct {
	Data string `json:"data"`
}

type AstroportAsset struct {
	Info   AssetInfo `json:"info"`
	Amount string    `json:"amount"`
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

type ICAQueryResponse struct {
	Data DepositorInterchainAccountAddressQueryResponse `json:"data"`
}

type DepositorInterchainAccountAddressQueryResponse struct {
	DepositorInterchainAccountAddress string `json:"depositor_interchain_account_address"`
}

//------------------//

type BalanceResponse struct {
	Balance string `json:"balance"`
}

type Cw20BalanceResponse struct {
	Data BalanceResponse `json:"data"`
}

type AllAccountsResponse struct {
	Data []string `json:"all_accounts_response"`
}

type Cw20QueryMsg struct {
	Balance Balance `json:"balance"`
	// AllAccounts *AllAccounts `json:"all_accounts"`
}

type AllAccounts struct {
}

type Balance struct {
	Address string `json:"address"`
}
