package covenant_single_party_pol

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
