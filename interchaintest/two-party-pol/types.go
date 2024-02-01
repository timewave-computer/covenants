package covenant_two_party_pol

import (
	cw "github.com/CosmWasm/wasmvm/types"
)

//////////////////////////////////////////////
///// Covenant contracts
//////////////////////////////////////////////

// ----- Covenant Instantiation ------
type CovenantInstantiateMsg struct {
	Label              string                 `json:"label"`
	Timeouts           Timeouts               `json:"timeouts"`
	PresetIbcFee       PresetIbcFee           `json:"preset_ibc_fee"`
	ContractCodeIds    ContractCodeIds        `json:"contract_codes"`
	TickMaxGas         string                 `json:"clock_tick_max_gas,omitempty"`
	LockupConfig       Expiration             `json:"lockup_config"`
	PartyAConfig       CovenantPartyConfig    `json:"party_a_config"`
	PartyBConfig       CovenantPartyConfig    `json:"party_b_config"`
	RagequitConfig     *RagequitConfig        `json:"ragequit_config,omitempty"`
	DepositDeadline    Expiration             `json:"deposit_deadline"`
	CovenantType       string                 `json:"covenant_type"`
	PartyAShare        string                 `json:"party_a_share"`
	PartyBShare        string                 `json:"party_b_share"`
	Splits             map[string]SplitConfig `json:"splits"`
	FallbackSplit      *SplitConfig           `json:"fallback_split,omitempty"`
	EmergencyCommittee string                 `json:"emergency_committee,omitempty"`
	LiquidPoolerConfig LiquidPoolerConfig     `json:"liquid_pooler_config"`
	PfmUnwindingConfig PfmUnwindingConfig     `json:"pfm_unwinding_config"`
	PoolPriceConfig    PoolPriceConfig        `json:"pool_price_config"`
}

type PfmUnwindingConfig struct {
	Party1PfmMap map[string]PacketForwardMiddlewareConfig `json:"party_1_pfm_map"`
	Party2PfmMap map[string]PacketForwardMiddlewareConfig `json:"party_2_pfm_map"`
}

type PacketForwardMiddlewareConfig struct {
	LocalToHopChainChannelId       string `json:"local_to_hop_chain_channel_id"`
	HopToDestinationChainChannelId string `json:"hop_to_destination_chain_channel_id"`
	HopChainReceiverAddress        string `json:"hop_chain_receiver_address"`
}

type LiquidPoolerConfig struct {
	Astroport *AstroportLiquidPoolerConfig `json:"astroport,omitempty"`
	Osmosis   *OsmosisLiquidPoolerConfig   `json:"osmosis,omitempty"`
}

type OsmosisLiquidPoolerConfig struct {
	NoteAddress            string             `json:"note_address"`
	PoolId                 string             `json:"pool_id"`
	OsmoIbcTimeout         string             `json:"osmo_ibc_timeout"`
	OsmoOutpost            string             `json:"osmo_outpost"`
	Party1ChainInfo        PartyChainInfo     `json:"party_1_chain_info"`
	Party2ChainInfo        PartyChainInfo     `json:"party_2_chain_info"`
	LpTokenDenom           string             `json:"lp_token_denom"`
	OsmoToNeutronChannelId string             `json:"osmo_to_neutron_channel_id"`
	Party1DenomInfo        PartyDenomInfo     `json:"party_1_denom_info"`
	Party2DenomInfo        PartyDenomInfo     `json:"party_2_denom_info"`
	FundingDuration        Duration           `json:"funding_duration"`
	SingleSideLpLimits     SingleSideLpLimits `json:"single_side_lp_limits"`
}

type SingleSideLpLimits struct {
	AssetALimit string `json:"asset_a_limit"`
	AssetBLimit string `json:"asset_b_limit"`
}

type AstroportLiquidPoolerConfig struct {
	PairType           PairType           `json:"pool_pair_type"`
	PoolAddress        string             `json:"pool_address"`
	AssetADenom        string             `json:"asset_a_denom"`
	AssetBDenom        string             `json:"asset_b_denom"`
	SingleSideLpLimits SingleSideLpLimits `json:"single_side_lp_limits"`
}

type Receiver struct {
	Address string `json:"addr"`
	Share   string `json:"share"`
}

type SplitConfig struct {
	Receivers map[string]string `json:"receivers"`
}

type ContractCodeIds struct {
	IbcForwarderCode     uint64 `json:"ibc_forwarder_code"`
	InterchainRouterCode uint64 `json:"interchain_router_code"`
	NativeRouterCode     uint64 `json:"native_router_code"`
	ClockCode            uint64 `json:"clock_code"`
	HolderCode           uint64 `json:"holder_code"`
	LiquidPoolerCode     uint64 `json:"liquid_pooler_code"`
}

type Timeouts struct {
	IcaTimeout         string `json:"ica_timeout"`
	IbcTransferTimeout string `json:"ibc_transfer_timeout"`
}

type PresetIbcFee struct {
	AckFee     string `json:"ack_fee"`
	TimeoutFee string `json:"timeout_fee"`
}

type Timestamp string
type Block uint64

type Expiration struct {
	Never    string     `json:"none,omitempty"`
	AtHeight *Block     `json:"at_height,omitempty"`
	AtTime   *Timestamp `json:"at_time,omitempty"`
}

type Duration struct {
	Height *uint64 `json:"height,omitempty"`
	Time   *uint64 `json:"time,omitempty"`
}

type RagequitConfig struct {
	Disabled bool           `json:"disabled,omitempty"`
	Enabled  *RagequitTerms `json:"enabled,omitempty"`
}

type Share string
type Side string

type CovenantType struct {
	Share string `json:"share,omitempty"`
	Side  string `json:"side,omitempty"`
}

type RagequitTerms struct {
	Penalty string         `json:"penalty"`
	State   *RagequitState `json:"state,omitempty"`
}

type RagequitState struct {
	Coins   []Coin        `json:"coins"`
	RqParty CovenantParty `json:"rq_party"`
}

type CovenantParty struct {
	Contribution Coin   `json:"contribution"`
	Addr         string `json:"addr"`
	Allocation   string `json:"allocation"`
	Router       string `json:"router"`
}

type CovenantPartyConfig struct {
	Interchain *InterchainCovenantParty `json:"interchain,omitempty"`
	Native     *NativeCovenantParty     `json:"native,omitempty"`
}

type InterchainCovenantParty struct {
	Addr                      string `json:"addr"`
	NativeDenom               string `json:"native_denom"`
	RemoteChainDenom          string `json:"remote_chain_denom"`
	PartyToHostChainChannelId string `json:"party_to_host_chain_channel_id"`
	HostToPartyChainChannelId string `json:"host_to_party_chain_channel_id"`
	PartyReceiverAddr         string `json:"party_receiver_addr"`
	PartyChainConnectionId    string `json:"party_chain_connection_id"`
	IbcTransferTimeout        string `json:"ibc_transfer_timeout"`
	Contribution              Coin   `json:"contribution"`
}

type NativeCovenantParty struct {
	Addr              string `json:"addr"`
	NativeDenom       string `json:"native_denom"`
	PartyReceiverAddr string `json:"party_receiver_addr"`
	Contribution      Coin   `json:"contribution"`
}

// type CovenantPartyConfig struct {
// 	ControllerAddr            string `json:"controller_addr"`
// 	HostAddr                  string `json:"host_addr"`
// 	Contribution              Coin   `json:"contribution"`
// 	IbcDenom                  string `json:"ibc_denom"`
// 	PartyToHostChainChannelId string `json:"party_to_host_chain_channel_id"`
// 	HostToPartyChainChannelId string `json:"host_to_party_chain_channel_id"`
// 	PartyChainConnectionId    string `json:"party_chain_connection_id"`
// 	IbcTransferTimeout        string `json:"ibc_transfer_timeout"`
// }

type Coin struct {
	Denom  string `json:"denom"`
	Amount string `json:"amount"`
}

// ----- Covenant Queries ------
type ClockAddress struct{}
type ClockAddressQuery struct {
	ClockAddress ClockAddress `json:"clock_address"`
}

type HolderAddress struct{}
type HolderAddressQuery struct {
	HolderAddress HolderAddress `json:"holder_address"`
}

type CovenantParties struct{}
type CovenantPartiesQuery struct {
	CovenantParties CovenantParties `json:"covenant_parties"`
}

type Party struct {
	Party string `json:"party"`
}
type InterchainRouterQuery struct {
	Party Party `json:"interchain_router_address"`
}
type IbcForwarderQuery struct {
	Party Party `json:"ibc_forwarder_address"`
}
type LiquidPoolerAddress struct{}
type LiquidPoolerQuery struct {
	LiquidPoolerAddress LiquidPoolerAddress `json:"liquid_pooler_address"`
}
type CovenantAddressQueryResponse struct {
	Data string `json:"data"`
}

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

type LpPositionQueryResponse struct {
	Data string `json:"data"`
}

type AstroportAsset struct {
	Info   AssetInfo `json:"info"`
	Amount string    `json:"amount"`
}

type LpPositionQuery struct{}

type PairInfo struct {
	LiquidityToken string      `json:"liquidity_token"`
	ContractAddr   string      `json:"contract_addr"`
	PairType       PairType    `json:"pair_type"`
	AssetInfos     []AssetInfo `json:"asset_infos"`
}

type LPPositionQuery struct {
	LpPosition LpPositionQuery `json:"lp_position"`
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

type NativeBalQueryResponse struct {
	Amount string `json:"amount"`
	Denom  string `json:"denom"`
}

// polytone types
type PolytonePair struct {
	ConnectionId string `json:"connection_id"`
	RemotePort   string `json:"remote_port"`
}

type NoteInstantiate struct {
	Pair        *PolytonePair `json:"pair,omitempty"`
	BlockMaxGas string        `json:"block_max_gas,omitempty"`
}

type VoiceInstantiate struct {
	ProxyCodeId uint64 `json:"proxy_code_id,string"`
	BlockMaxGas uint64 `json:"block_max_gas,string"`
}

type CallbackRequest struct {
	Receiver string `json:"receiver"`
	Msg      string `json:"msg"`
}

type CallbackMessage struct {
	Initiator    string   `json:"initiator"`
	InitiatorMsg string   `json:"initiator_msg"`
	Result       Callback `json:"result"`
}

type Callback struct {
	Success []string `json:"success,omitempty"`
	Error   string   `json:"error,omitempty"`
}

type NoteExecuteMsg struct {
	Msgs           []cw.CosmosMsg   `json:"msgs"`
	TimeoutSeconds uint64           `json:"timeout_seconds,string"`
	Callback       *CallbackRequest `json:"callback,omitempty"`
}

type NoteQuery struct {
	Msgs           []cw.CosmosMsg  `json:"msgs"`
	TimeoutSeconds uint64          `json:"timeout_seconds,string"`
	Callback       CallbackRequest `json:"callback"`
}

type NoteExecute struct {
	Query   *NoteQuery      `json:"query,omitempty"`
	Execute *NoteExecuteMsg `json:"execute,omitempty"`
}

type RemoteAddress struct {
	LocalAddress string `json:"local_address"`
}
type NoteQueryMsg struct {
	RemoteAddressQuery RemoteAddress `json:"remote_address"`
}

type TesterInstantiate struct {
}

type StargateMsg struct {
	TypeUrl string `json:"type_url"`
	Value   string `json:"value"`
}

// osmosis.gamm.v1beta1.MsgJoinPool
type MsgJoinPool struct {
	Sender         string    `json:"sender"`
	PoolId         uint64    `json:"pool_id"`
	ShareOutAmount string    `json:"share_out_amount"`
	TokenInMaxs    []cw.Coin `json:"token_in_maxs"`
}

type OsmoLiquidPoolerInstantiateMsg struct {
	ClockAddress           string             `json:"clock_address"`
	HolderAddress          string             `json:"holder_address"`
	NoteAddress            string             `json:"note_address"`
	PoolId                 string             `json:"pool_id"`
	OsmoIbcTimeout         string             `json:"osmo_ibc_timeout"`
	Party1ChainInfo        PartyChainInfo     `json:"party_1_chain_info"`
	Party2ChainInfo        PartyChainInfo     `json:"party_2_chain_info"`
	OsmoToNeutronChannelId string             `json:"osmo_to_neutron_channel_id"`
	Party1DenomInfo        PartyDenomInfo     `json:"party_1_denom_info"`
	Party2DenomInfo        PartyDenomInfo     `json:"party_2_denom_info"`
	OsmoOutpost            string             `json:"osmo_outpost"`
	LpTokenDenom           string             `json:"lp_token_denom"`
	SlippageTolerance      string             `json:"slippage_tolerance"`
	FundingDurationSeconds string             `json:"funding_duration_seconds"`
	SingleSideLpLimits     SingleSideLpLimits `json:"single_side_lp_limits"`
	PoolPriceConfig        PoolPriceConfig    `json:"pool_price_config"`
}

type PoolPriceConfig struct {
	ExpectedSpotPrice     string `json:"expected_spot_price"`
	AcceptablePriceSpread string `json:"acceptable_price_spread"`
}

type PartyDenomInfo struct {
	OsmosisCoin cw.Coin `json:"osmosis_coin"`
	LocalDenom  string  `json:"local_denom"`
}

type PartyChainInfo struct {
	NeutronToPartyChainChannel string           `json:"neutron_to_party_chain_channel"`
	PartyChainToNeutronChannel string           `json:"party_chain_to_neutron_channel"`
	InwardsPfm                 *ForwardMetadata `json:"inwards_pfm,omitempty"`
	OutwardsPfm                *ForwardMetadata `json:"outwards_pfm,omitempty"`
	IbcTimeout                 string           `json:"ibc_timeout"`
}

type PacketMetadata struct {
	ForwardMetadata *ForwardMetadata `json:"forward,omitempty"`
}

type ForwardMetadata struct {
	Receiver string `json:"receiver"`
	Port     string `json:"port"`
	Channel  string `json:"channel"`
	// Timeout  string `json:"timeout,omitempty"`
	// Retries  uint8  `json:"retries,omitempty"`
}
