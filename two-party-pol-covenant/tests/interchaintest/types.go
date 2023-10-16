package ibc_test

//////////////////////////////////////////////
///// Covenant contracts
//////////////////////////////////////////////

// ----- Covenant Instantiation ------
type CovenantInstantiateMsg struct {
	Label           string              `json:"label"`
	Timeouts        Timeouts            `json:"timeouts"`
	PresetIbcFee    PresetIbcFee        `json:"preset_ibc_fee"`
	ContractCodeIds ContractCodeIds     `json:"contract_codes"`
	TickMaxGas      string              `json:"clock_tick_max_gas,omitempty"`
	LockupConfig    ExpiryConfig        `json:"lockup_config"`
	PartyAConfig    CovenantPartyConfig `json:"party_a_config"`
	PartyBConfig    CovenantPartyConfig `json:"party_b_config"`
	PoolAddress     string              `json:"pool_address"`
	RagequitConfig  *RagequitConfig     `json:"ragequit_config,omitempty"`
	DepositDeadline *ExpiryConfig       `json:"deposit_deadline,omitempty"`
	PartyAShare     string              `json:"party_a_share"`
	PartyBShare     string              `json:"party_b_share"`
}

type ContractCodeIds struct {
	IbcForwarderCode     uint64 `json:"ibc_forwarder_code"`
	InterchainRouterCode uint64 `json:"router_code"`
	ClockCode            uint64 `json:"clock_code"`
	HolderCode           uint64 `json:"holder_code"`
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

type ExpiryConfig struct {
	None        string     `json:"none,omitempty"`
	BlockHeight *Block     `json:"block,omitempty"`
	Time        *Timestamp `json:"time,omitempty"`
}

type RagequitConfig struct {
	Disabled bool           `json:"disabled,omitempty"`
	Enabled  *RagequitTerms `json:"enabled,omitempty"`
}

type RagequitTerms struct {
	Penalty string         `json:"penalty,omitempty"`
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
	Addr                      string `json:"addr"`
	Contribution              Coin   `json:"contribution"`
	IbcDenom                  string `json:"ibc_denom"`
	PartyToHostChainChannelId string `json:"party_to_host_chain_channel_id"`
	HostToPartyChainChannelId string `json:"host_to_party_chain_channel_id"`
	PartyReceiverAddr         string `json:"party_receiver_addr"`
	PartyChainConnectionId    string `json:"party_chain_connection_id"`
	IbcTransferTimeout        string `json:"ibc_transfer_timeout"`
}

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

type CovenantAddressQueryResponse struct {
	Data string `json:"data"`
}
