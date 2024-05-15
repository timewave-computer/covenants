use std::path::PathBuf;
use std::{collections::HashMap, path};

use cosmwasm_std::{StdError, StdResult};
use localic_std::relayer::{Channel, Relayer};
use localic_std::{modules::cosmwasm::CosmWasm, transactions::ChainRequestBuilder};

use crate::types::ChainsVec;
use crate::utils::API_URL;


pub struct TestContext {
    pub chains: HashMap<String, LocalChain>,
    pub transfer_channel_ids: HashMap<(String, String), String>,
    pub ccv_channel_ids: HashMap<(String, String), String>,
}

impl From<ChainsVec> for TestContext {
    fn from(chains: ChainsVec) -> Self {
        let mut chains_map = HashMap::new();
        for chain in chains.chains {
            let rb = ChainRequestBuilder::new(
                API_URL.to_string(),
                chain.chain_id.clone(),
                chain.debugging,
            )
            .unwrap();

            let relayer: Relayer = Relayer::new(&rb);
            let channels = relayer.get_channels(&rb.chain_id).unwrap();

            for (i, channel) in channels.iter().enumerate() {
                println!("{} channel #{}: {:?}", rb.chain_id, i, channel);
            }

            let (src_addr, denom) = match rb.chain_id.as_str() {
                "localneutron-1" => ("neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xpznmsky", "untrn"),
                "localcosmos-1" => ("cosmos1hj5fveer5cjtn4wd6wstzugjfdxzl0xpxvjjvr", "uatom"),
                "localstride-3" => ("stride1u20df3trc2c2zdhm8qvh2hdjx9ewh00sv6eyy8", "ustrd"),
                _ => ("err", "err"),
            };
            let local_chain = LocalChain::new(rb, src_addr.to_string(), denom.to_string(), channels);
            chains_map.insert(chain.name.clone(), local_chain);
        }

        let mut ntrn_channels = chains_map.get("neutron").unwrap().channels.clone();
        let mut gaia_channels = chains_map.get("gaia").unwrap().channels.clone();
        let mut stride_channels = chains_map.get("stride").unwrap().channels.clone();

        let (ntrn_to_gaia_consumer_channel, gaia_to_ntrn_provider_channel) =
            find_pairwise_ccv_channel_ids(&gaia_channels, &ntrn_channels).unwrap();

        ntrn_channels.remove(ntrn_to_gaia_consumer_channel.index);
        gaia_channels.remove(gaia_to_ntrn_provider_channel.index);

        let (ntrn_to_stride_transfer_channel, stride_to_ntrn_transfer_channel) =
            find_pairwise_transfer_channel_ids(&ntrn_channels, &stride_channels).unwrap();
        ntrn_channels.remove(ntrn_to_stride_transfer_channel.index);
        stride_channels.remove(stride_to_ntrn_transfer_channel.index);

        let (gaia_to_stride_transfer_channel, stride_to_gaia_transfer_channel) =
            find_pairwise_transfer_channel_ids(&gaia_channels, &stride_channels).unwrap();
        gaia_channels.remove(gaia_to_stride_transfer_channel.index);
        stride_channels.remove(stride_to_gaia_transfer_channel.index);

        let (ntrn_to_gaia_transfer_channel, gaia_to_ntrn_transfer_channel) =
            find_pairwise_transfer_channel_ids(&ntrn_channels, &gaia_channels).unwrap();
        ntrn_channels.remove(ntrn_to_gaia_transfer_channel.index);
        gaia_channels.remove(gaia_to_ntrn_transfer_channel.index);

        let mut transfer_channel_ids = HashMap::new();
        transfer_channel_ids.insert(("neutron".to_string(), "stride".to_string()), ntrn_to_stride_transfer_channel.channel_id);
        transfer_channel_ids.insert(("stride".to_string(), "neutron".to_string()), stride_to_ntrn_transfer_channel.channel_id);
        transfer_channel_ids.insert(("gaia".to_string(), "stride".to_string()), gaia_to_stride_transfer_channel.channel_id);
        transfer_channel_ids.insert(("stride".to_string(), "gaia".to_string()), stride_to_gaia_transfer_channel.channel_id);
        transfer_channel_ids.insert(("neutron".to_string(), "gaia".to_string()), ntrn_to_gaia_transfer_channel.channel_id);
        transfer_channel_ids.insert(("gaia".to_string(), "neutron".to_string()), gaia_to_ntrn_transfer_channel.channel_id);

        let mut ccv_channel_ids = HashMap::new();
        ccv_channel_ids.insert(("gaia".to_string(), "neutron".to_string()), gaia_to_ntrn_provider_channel.channel_id);
        ccv_channel_ids.insert(("neutron".to_string(), "gaia".to_string()), ntrn_to_gaia_consumer_channel.channel_id);

        Self {
            chains: chains_map,
            transfer_channel_ids,
            ccv_channel_ids,
        }
    }
}

fn find_pairwise_transfer_channel_ids(a: &Vec<Channel>, b: &Vec<Channel>) -> StdResult<(PairwiseChannelResult, PairwiseChannelResult)> {
    for (a_i, a_chan) in a.iter().enumerate() {
        for (b_i, b_chan) in b.iter().enumerate() {
            if a_chan.channel_id == b_chan.counterparty.channel_id
                && b_chan.channel_id == a_chan.counterparty.channel_id
                && a_chan.port_id == "transfer"
                && b_chan.port_id == "transfer"
                && a_chan.ordering == "ORDER_UNORDERED"
                && b_chan.ordering == "ORDER_UNORDERED"
            {
                let a_channel_result = PairwiseChannelResult {
                    index: a_i,
                    channel_id: a_chan.channel_id.to_string(),
                };
                let b_channel_result = PairwiseChannelResult {
                    index: b_i,
                    channel_id: b_chan.channel_id.to_string(),
                };

                return Ok((a_channel_result, b_channel_result))
            }
        }
    }
    Err(StdError::generic_err("failed to match pairwise transfer channels"))
}

fn find_pairwise_ccv_channel_ids(provider_channels: &Vec<Channel>, consumer_channels: &Vec<Channel>) -> StdResult<(PairwiseChannelResult, PairwiseChannelResult)> {
    for (a_i, a_chan) in provider_channels.iter().enumerate() {
        for (b_i, b_chan) in consumer_channels.iter().enumerate() {
            if a_chan.channel_id == b_chan.counterparty.channel_id
                && b_chan.channel_id == a_chan.counterparty.channel_id
                && a_chan.port_id == "provider"
                && b_chan.port_id == "consumer"
                && a_chan.ordering == "ORDER_ORDERED"
                && b_chan.ordering == "ORDER_ORDERED"
            {
                let provider_channel_result = PairwiseChannelResult {
                    index: a_i,
                    channel_id: a_chan.channel_id.to_string(),
                };
                let consumer_channel_result = PairwiseChannelResult {
                    index: b_i,
                    channel_id: b_chan.channel_id.to_string(),
                };
                return Ok((provider_channel_result, consumer_channel_result))
            }
        }
    }
    Err(StdError::generic_err("failed to match pairwise ccv channels"))
}

pub struct PairwiseChannelResult {
    pub index: usize,
    pub channel_id: String,
}

pub struct LocalChain {
    /// ChainRequestBuilder
    pub rb: ChainRequestBuilder,
    /// contract codes stored on this chain (filename -> code_id)
    pub contract_codes: HashMap<String, u64>,
    /// outgoing channel ids
    pub channels: Vec<Channel>,
    /// outgoing connection ids available (dest_chain_id -> connection_id)
    pub connection_ids: HashMap<String, String>,
    pub admin_addr: String,
    pub native_denom: String,
}

impl LocalChain {
    pub fn new(rb: ChainRequestBuilder, admin_addr: String, native_denom: String, channels: Vec<Channel>) -> Self {
        Self {
            rb,
            contract_codes: Default::default(),
            channels,
            connection_ids: Default::default(),
            admin_addr,
            native_denom,
        }
    }

    pub fn get_cw(&mut self) -> CosmWasm {
        CosmWasm::new(&self.rb)
    }

    pub fn save_code(&mut self, abs_path: PathBuf, code: u64) {
        let id = abs_path.file_stem().unwrap().to_str().unwrap();
        self.contract_codes.insert(id.to_string(), code);
    }
}

/// Will panic if the current directory path is not found.
#[must_use]
pub fn get_current_dir() -> path::PathBuf {
    match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => panic!("Could not get current dir: {e}"),
    }
}

/// Will panic if the `local_interchain` directory is not found in the parent path.
#[must_use]
pub fn get_local_interchain_dir() -> path::PathBuf {
    let current_dir = get_current_dir();
    let Some(parent_dir) = current_dir.parent() else { panic!("Could not get parent dir") };
    parent_dir.to_path_buf()
}

/// local-interchain/contracts directory
#[must_use]
pub fn get_contract_path() -> path::PathBuf {
    get_local_interchain_dir().join("contracts")
}

/// local-interchain/configs/contract.json file
#[must_use]
pub fn get_contract_cache_path() -> path::PathBuf {
    get_local_interchain_dir()
        .join("configs")
        .join("contract.json")
}
