package ibc_test

import (
	"errors"

	"github.com/strangelove-ventures/interchaintest/v4/ibc"
)

func getPairwiseConnectionIds(aconns ibc.ConnectionOutputs, bconns ibc.ConnectionOutputs) ([]string, []string, error) {
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
		return abconnids, baconnids, errors.New("No connection found")
	}
}

// returns transfer channels and respective connections
func getPairwiseTransferChannelIds(achans []ibc.ChannelOutput, bchans []ibc.ChannelOutput, abconns []string, baconns []string) (string, string, string, string, error) {
	var abchan string
	var bachan string
	var abconn string
	var baconn string

	found := false

	for _, a := range achans {
		for _, b := range bchans {
			if a.ChannelID == b.Counterparty.ChannelID &&
				b.ChannelID == a.Counterparty.ChannelID &&
				a.PortID == "transfer" &&
				b.PortID == "transfer" &&
				a.Ordering == "ORDER_UNORDERED" &&
				b.Ordering == "ORDER_UNORDERED" {
				for _, abcon := range abconns {
					for _, bacon := range baconns {
						if a.ConnectionHops[0] == abcon &&
							b.ConnectionHops[0] == bacon {
							abchan = a.ChannelID
							bachan = b.ChannelID
							abconn = abcon
							baconn = bacon
							found = true
						}
					}
				}
			}
		}
	}
	if found {
		return abchan, bachan, abconn, baconn, nil
	} else {
		return abchan, bachan, abconn, baconn, errors.New("No transfer channel found")
	}
}

// returns ccv channels and respective connections
func getPairwiseCCVChannelIds(achans []ibc.ChannelOutput, bchans []ibc.ChannelOutput, abconns []string, baconns []string) (string, string, string, string, error) {
	var abchan string
	var bachan string
	var abconn string
	var baconn string

	found := false
	for _, a := range achans {
		for _, b := range bchans {
			if a.ChannelID == b.Counterparty.ChannelID &&
				b.ChannelID == a.Counterparty.ChannelID &&
				a.PortID == "provider" &&
				b.PortID == "consumer" &&
				a.Ordering == "ORDER_ORDERED" &&
				b.Ordering == "ORDER_ORDERED" {
				for _, abcon := range abconns {
					for _, bacon := range baconns {
						if a.ConnectionHops[0] == abcon &&
							b.ConnectionHops[0] == bacon {
							abchan = a.ChannelID
							bachan = b.ChannelID
							abconn = abcon
							baconn = bacon
							found = true
						}
					}
				}
			}
		}
	}
	if found {
		return abchan, bachan, abconn, baconn, nil
	} else {
		return abchan, bachan, abconn, baconn, errors.New("No ccv channel found")
	}
}
