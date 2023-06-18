package ibc_test

type DepositorInstantiateMsg struct {
	StAtomReceiver                  WeightedReceiver `json:"st_atom_receiver"`
	AtomReceiver                    WeightedReceiver `json:"atom_receiver"`
	ClockAddress                    string           `json:"clock_address,string"`
	GaiaNeutronIBCTransferChannelId string           `json:"gaia_neutron_ibc_transfer_channel_id"`
}

type LPerInstantiateMsg struct {
	LpPosition    LpInfo `json:"lp_position"`
	ClockAddress  string `json:"clock_address,string"`
	HolderAddress string `json:"holder_address,string"`
}

type LpInfo struct {
	Addr string `json:"addr,string"`
}

type WeightedReceiver struct {
	Amount  int64  `json:"amount"`
	Address string `json:"address,string"`
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

type QueryResponse struct {
	Data InterchainAccountAddressQueryResponse `json:"data"`
}

type ICAQueryResponse struct {
	Data DepositorInterchainAccountAddressQueryResponse `json:"data"`
}

type InterchainAccountAddressQueryResponse struct {
	InterchainAccountAddress string `json:"interchain_account_address"`
}

type DepositorICAAddressQuery struct {
	DepositorInterchainAccountAddress DepositorInterchainAccountAddressQuery `json:"depositor_interchain_account_address"`
}

type DepositorContractQuery struct {
	ClockAddress ClockAddressQuery `json:"clock_address"`
}

type StAtomWeightedReceiverQuery struct {
	StAtomReceiver StAtomReceiverQuery `json:"st_atom_receiver"`
}

type AtomWeightedReceiverQuery struct {
	AtomReceiver AtomReceiverQuery `json:"atom_receiver"`
}

type ClockAddressQuery struct{}
type StAtomReceiverQuery struct{}
type AtomReceiverQuery struct{}
type DepositorInterchainAccountAddressQuery struct{}

type WeightedReceiverResponse struct {
	Data WeightedReceiver `json:"data"`
}

type ClockQueryResponse struct {
	Data string `json:"data"`
}

// A query response from the Neutron contract. Note that when
// interchaintest returns query responses, it does so in the form
// `{"data": <RESPONSE>}`, so we need this outer data key, which is
// not present in the neutron contract, to properly deserialze.

type DepositorInterchainAccountAddressQueryResponse struct {
	DepositorInterchainAccountAddress string `json:"depositor_interchain_account_address"`
}
