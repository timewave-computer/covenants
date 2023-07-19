package ibc_test

//////////////////////////////////////////////
///// Covenant contracts
//////////////////////////////////////////////

// ----- Covenant Instantiation ------

type PresetLsFields struct {
	LsCode                            uint64 `json:"ls_code"`
	Label                             string `json:"label"`
	LsDenom                           string `json:"ls_denom"`
	StrideNeutronIBCTransferChannelId string `json:"stride_neutron_ibc_transfer_channel_id"`
	NeutronStrideIBCConnectionId      string `json:"neutron_stride_ibc_connection_id"`
}

type CovenantInstantiateMsg struct {
	Label                          string                `json:"label"`
	PresetClock                    PresetClockFields     `json:"preset_clock_fields"`
	PresetLs                       PresetLsFields        `json:"preset_ls_fields"`
	PresetDepositor                PresetDepositorFields `json:"preset_depositor_fields"`
	PresetLp                       PresetLpFields        `json:"preset_lp_fields"`
	PresetHolder                   PresetHolderFields    `json:"preset_holder_fields"`
	PoolAddress                    string                `json:"pool_address"`
	IbcMsgTransferTimeoutTimestamp uint64                `json:"ibc_msg_transfer_timeout_timestamp"`
	// PresetIbcFee                   PresetIbcFee          `json:"preset_ibc_fee"`
}

type PresetIbcFee struct {
	AckFee     CwCoin `json:"ack_fee"`
	TimeoutFee CwCoin `json:"timeout_fee"`
}

type PresetClockFields struct {
	TickMaxGas string   `json:"tick_max_gas,omitempty"`
	ClockCode  uint64   `json:"clock_code"`
	Label      string   `json:"label"`
	Whitelist  []string `json:"whitelist"`
}

type PresetHolderFields struct {
	Withdrawer string `json:"withdrawer,omitempty"`
	HolderCode uint64 `json:"holder_code"`
	Label      string `json:"label"`
}

type PresetDepositorFields struct {
	GaiaNeutronIBCTransferChannelId string                 `json:"gaia_neutron_ibc_transfer_channel_id"`
	NeutronGaiaConnectionId         string                 `json:"neutron_gaia_connection_id"`
	GaiaStrideIBCTransferChannelId  string                 `json:"gaia_stride_ibc_transfer_channel_id"`
	DepositorCode                   uint64                 `json:"depositor_code"`
	Label                           string                 `json:"label"`
	StAtomReceiverAmount            WeightedReceiverAmount `json:"st_atom_receiver_amount"`
	AtomReceiverAmount              WeightedReceiverAmount `json:"atom_receiver_amount"`
	AutopilotFormat                 string                 `json:"autopilot_format"`
}

type PresetLpFields struct {
	SlippageTolerance string    `json:"slippage_tolerance,omitempty"`
	Autostake         bool      `json:"autostake,omitempty"`
	Assets            AssetData `json:"assets"`
	LpCode            uint64    `json:"lp_code"`
	Label             string    `json:"label"`
}

type AssetData struct {
	NativeAssetDenom string `json:"native_asset_denom"`
	LsAssetDenom     string `json:"ls_asset_denom"`
}

// ----- Covenant Queries ------

type DepositorAddress struct{}
type DepositorAddressQuery struct {
	DepositorAddress DepositorAddress `json:"depositor_address"`
}

type ClockAddress struct{}
type ClockAddressQuery struct {
	ClockAddress ClockAddress `json:"clock_address"`
}

type HolderAddress struct{}
type HolderAddressQuery struct {
	HolderAddress HolderAddress `json:"holder_address"`
}

type LsAddress struct{}
type LsAddressQuery struct {
	LsAddress LsAddress `json:"ls_address"`
}

type LpAddress struct{}
type LpAddressQuery struct {
	LpAddress LpAddress `json:"lp_address"`
}

type CovenantAddressQueryResponse struct {
	Data string `json:"data"`
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
	Amount  uint64 `json:"amount"`
	Address string `json:"address"`
}

type WeightedReceiverAmount struct {
	Amount uint64 `json:"amount"`
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

type LpInfo struct {
	Addr string `json:"addr"`
}

//////////////////////////////////////////////
///// Holder contract
//////////////////////////////////////////////

type CovenantHolderAddressQuery struct {
	Addr string `json:"address"`
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
	Data LpInfo `json:"data"`
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
