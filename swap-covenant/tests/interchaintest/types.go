package ibc_test

//////////////////////////////////////////////
///// Covenant contracts
//////////////////////////////////////////////

// ----- Covenant Instantiation ------
type CovenantInstantiateMsg struct {
	Label                  string                 `json:"label"`
	Timeouts               Timeouts               `json:"timeouts"`
	PresetIbcFee           PresetIbcFee           `json:"preset_ibc_fee"`
	IbcForwarderCode       uint64                 `json:"ibc_forwarder_code"`
	InterchainRouterCode   uint64                 `json:"interchain_router_code"`
	InterchainSplitterCode uint64                 `json:"splitter_code"`
	PresetClock            PresetClockFields      `json:"preset_clock_fields"`
	PresetSwapHolder       PresetSwapHolderFields `json:"preset_holder_fields"`
	SwapCovenantParties    SwapCovenantParties    `json:"covenant_parties"`
	PresetSplitterFields   PresetSplitterFields   `json:"preset_splitter_fields"`
}

type Receiver struct {
	Address string `json:"addr"`
	Share   string `json:"share"`
}

type SplitConfig struct {
	Receivers []Receiver `json:"receivers"`
}

type SplitType struct {
	Custom SplitConfig `json:"custom"`
}

type DenomSplit struct {
	Denom string    `json:"denom"`
	Type  SplitType `json:"split"`
}

type PresetSplitterFields struct {
	Splits        []DenomSplit `json:"splits"`
	FallbackSplit *SplitType   `json:"fallback_split,omitempty"`
	Label         string       `json:"label"`
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

type HolderAddress struct{}
type HolderAddressQuery struct {
	HolderAddress HolderAddress `json:"holder_address"`
}

type SplitterAddress struct{}
type SplitterAddressQuery struct {
	SplitterAddress SplitterAddress `json:"splitter_address"`
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

type CovenantAddressQueryResponse struct {
	Data string `json:"data"`
}
