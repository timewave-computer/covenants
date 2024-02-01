package covenant_single_party_pol

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
	Amount string `json:"amount"`
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

// single party POL types

type CovenantInstantiationMsg struct {
	Label                     string                    `json:"label"`
	Timeouts                  Timeouts                  `json:"timeouts"`
	PresetIbcFee              PresetIbcFee              `json:"preset_ibc_fee"`
	ContractCodeIds           ContractCodeIds           `json:"contract_codes"`
	TickMaxGas                string                    `json:"clock_tick_max_gas,omitempty"`
	LockupConfig              Expiration                `json:"lockup_period"`
	LsInfo                    LsInfo                    `json:"ls_info"`
	LsForwarderConfig         CovenantPartyConfig       `json:"ls_forwarder_config"`
	LpForwarderConfig         CovenantPartyConfig       `json:"lp_forwarder_config"`
	RemoteChainSplitterConfig RemoteChainSplitterConfig `json:"remote_chain_splitter_config"`
	CovenantPartyConfig       InterchainCovenantParty   `json:"covenant_party_config"`
	LiquidPoolerConfig        LiquidPoolerConfig        `json:"liquid_pooler_config"`
	PoolPriceConfig           PoolPriceConfig           `json:"pool_price_config"`
}

type PoolPriceConfig struct {
	ExpectedSpotPrice     string `json:"expected_spot_price"`
	AcceptablePriceSpread string `json:"acceptable_price_spread"`
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

type PartyDenomInfo struct {
	OsmosisCoin       Coin   `json:"osmosis_coin"`
	LocalDenom        string `json:"local_denom"`
	SingleSideLpLimit string `json:"single_side_lp_limit"`
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

type AstroportLiquidPoolerConfig struct {
	PairType           PairType           `json:"pool_pair_type"`
	PoolAddress        string             `json:"pool_address"`
	AssetADenom        string             `json:"asset_a_denom"`
	AssetBDenom        string             `json:"asset_b_denom"`
	SingleSideLpLimits SingleSideLpLimits `json:"single_side_lp_limits"`
}

type PfmUnwindingConfig struct {
	PartyPfmMap map[string]PacketForwardMiddlewareConfig `json:"party_pfm_map"`
}

type PacketForwardMiddlewareConfig struct {
	LocalToHopChainChannelId       string `json:"local_to_hop_chain_channel_id"`
	HopToDestinationChainChannelId string `json:"hop_to_destination_chain_channel_id"`
	HopChainReceiverAddress        string `json:"hop_chain_receiver_address"`
}

type RemoteChainSplitterConfig struct {
	ChannelId    string `json:"channel_id"`
	ConnectionId string `json:"connection_id"`
	Denom        string `json:"denom"`
	Amount       string `json:"amount"`
	LsShare      string `json:"ls_share"`
	NativeShare  string `json:"native_share"`
}

type CovenantPartyConfig struct {
	Interchain *InterchainCovenantParty `json:"interchain,omitempty"`
	Native     *NativeCovenantParty     `json:"native,omitempty"`
}

type Coin struct {
	Denom  string `json:"denom"`
	Amount string `json:"amount"`
}

type InterchainCovenantParty struct {
	Addr                      string                                   `json:"addr"`
	NativeDenom               string                                   `json:"native_denom"`
	RemoteChainDenom          string                                   `json:"remote_chain_denom"`
	PartyToHostChainChannelId string                                   `json:"party_to_host_chain_channel_id"`
	HostToPartyChainChannelId string                                   `json:"host_to_party_chain_channel_id"`
	PartyReceiverAddr         string                                   `json:"party_receiver_addr"`
	PartyChainConnectionId    string                                   `json:"party_chain_connection_id"`
	IbcTransferTimeout        string                                   `json:"ibc_transfer_timeout"`
	Contribution              Coin                                     `json:"contribution"`
	DenomToPfmMap             map[string]PacketForwardMiddlewareConfig `json:"denom_to_pfm_map"`
}

type NativeCovenantParty struct {
	Addr              string `json:"addr"`
	NativeDenom       string `json:"native_denom"`
	PartyReceiverAddr string `json:"party_receiver_addr"`
	Contribution      Coin   `json:"contribution"`
}

type LsInfo struct {
	LsDenom                   string `json:"ls_denom"`
	LsDenomOnNeutron          string `json:"ls_denom_on_neutron"`
	LsChainToNeutronChannelId string `json:"ls_chain_to_neutron_channel_id"`
	LsNeutronConnectionId     string `json:"ls_neutron_connection_id"`
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

type ContractCodeIds struct {
	IbcForwarderCode        uint64 `json:"ibc_forwarder_code"`
	ClockCode               uint64 `json:"clock_code"`
	HolderCode              uint64 `json:"holder_code"`
	LiquidPoolerCode        uint64 `json:"liquid_pooler_code"`
	LiquidStakerCode        uint64 `json:"liquid_staker_code"`
	RemoteChainSplitterCode uint64 `json:"remote_chain_splitter_code"`
	InterchainRouterCode    uint64 `json:"interchain_router_code"`
}

type SplitConfig struct {
	Receivers map[string]string `json:"receivers"`
}

type LiquidStakerInstantiateMsg struct {
	ClockAddress                      string `json:"clock_address"`
	StrideNeutronIbcTransferChannelID string `json:"stride_neutron_ibc_transfer_channel_id"`
	NeutronStrideIbcConnectionID      string `json:"neutron_stride_ibc_connection_id"`
	NextContract                      string `json:"next_contract"`
	LsDenom                           string `json:"ls_denom"`
	IbcFee                            IbcFee `json:"ibc_fee"` // Assuming IbcFee is defined elsewhere
	IcaTimeout                        string `json:"ica_timeout"`
	IbcTransferTimeout                string `json:"ibc_transfer_timeout"`
	AutopilotFormat                   string `json:"autopilot_format"`
}

type IbcFee struct {
	RecvFee    []CwCoin `json:"recv_fee"`
	AckFee     []CwCoin `json:"ack_fee"`
	TimeoutFee []CwCoin `json:"timeout_fee"`
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
