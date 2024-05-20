use std::path::PathBuf;
use std::{collections::HashMap, path};

use localic_std::relayer::{Channel, Relayer};
use localic_std::{modules::cosmwasm::CosmWasm, transactions::ChainRequestBuilder};

use crate::chain_tests::{find_pairwise_ccv_channel_ids, find_pairwise_transfer_channel_ids};
use crate::ibc_helpers;
use crate::types::ChainsVec;
use crate::utils::API_URL;

pub struct TestContext {
    pub chains: HashMap<String, LocalChain>,
    // maps (src_chain_id, dest_chain_id) to transfer channel id
    pub transfer_channel_ids: HashMap<(String, String), String>,
    // maps (src_chain_id, dest_chain_id) to ccv channel id
    pub ccv_channel_ids: HashMap<(String, String), String>,
    // maps (src_chain_id, dest_chain_id) to connection id
    pub connection_ids: HashMap<(String, String), String>,
    // maps (src_chain_id, dest_chain_id) to src chain native
    // denom -> ibc denom on dest chain
    pub ibc_denoms: HashMap<(String, String), String>,
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
            let local_chain =
                LocalChain::new(rb, src_addr.to_string(), denom.to_string(), channels);
            chains_map.insert(chain.name.clone(), local_chain);
        }

        let mut ntrn_channels = chains_map.get("neutron").unwrap().channels.clone();
        let mut gaia_channels = chains_map.get("gaia").unwrap().channels.clone();
        let mut stride_channels = chains_map.get("stride").unwrap().channels.clone();

        let mut connection_ids = HashMap::new();

        let (ntrn_to_gaia_consumer_channel, gaia_to_ntrn_provider_channel) =
            find_pairwise_ccv_channel_ids(&gaia_channels, &ntrn_channels).unwrap();

        ntrn_channels.remove(ntrn_to_gaia_consumer_channel.index);
        gaia_channels.remove(gaia_to_ntrn_provider_channel.index);
        connection_ids.insert(
            ("neutron".to_string(), "gaia".to_string()),
            ntrn_to_gaia_consumer_channel.connection_id,
        );
        connection_ids.insert(
            ("gaia".to_string(), "neutron".to_string()),
            gaia_to_ntrn_provider_channel.connection_id,
        );

        let (ntrn_to_gaia_transfer_channel, gaia_to_ntrn_transfer_channel) =
            find_pairwise_transfer_channel_ids(&ntrn_channels, &gaia_channels).unwrap();
        ntrn_channels.remove(ntrn_to_gaia_transfer_channel.index);
        gaia_channels.remove(gaia_to_ntrn_transfer_channel.index);

        let (ntrn_to_stride_transfer_channel, stride_to_ntrn_transfer_channel) =
            find_pairwise_transfer_channel_ids(&ntrn_channels, &stride_channels).unwrap();
        ntrn_channels.remove(ntrn_to_stride_transfer_channel.index);
        stride_channels.remove(stride_to_ntrn_transfer_channel.index);
        connection_ids.insert(
            ("neutron".to_string(), "stride".to_string()),
            ntrn_to_stride_transfer_channel.connection_id,
        );
        connection_ids.insert(
            ("stride".to_string(), "neutron".to_string()),
            stride_to_ntrn_transfer_channel.connection_id,
        );

        let (gaia_to_stride_transfer_channel, stride_to_gaia_transfer_channel) =
            find_pairwise_transfer_channel_ids(&gaia_channels, &stride_channels).unwrap();
        gaia_channels.remove(gaia_to_stride_transfer_channel.index);
        stride_channels.remove(stride_to_gaia_transfer_channel.index);
        connection_ids.insert(
            ("gaia".to_string(), "stride".to_string()),
            gaia_to_stride_transfer_channel.connection_id,
        );
        connection_ids.insert(
            ("stride".to_string(), "gaia".to_string()),
            stride_to_gaia_transfer_channel.connection_id,
        );

        let mut transfer_channel_ids = HashMap::new();
        transfer_channel_ids.insert(
            ("neutron".to_string(), "stride".to_string()),
            ntrn_to_stride_transfer_channel.channel_id.to_string(),
        );
        transfer_channel_ids.insert(
            ("stride".to_string(), "neutron".to_string()),
            stride_to_ntrn_transfer_channel.channel_id.to_string(),
        );
        transfer_channel_ids.insert(
            ("gaia".to_string(), "stride".to_string()),
            gaia_to_stride_transfer_channel.channel_id.to_string(),
        );
        transfer_channel_ids.insert(
            ("stride".to_string(), "gaia".to_string()),
            stride_to_gaia_transfer_channel.channel_id.to_string(),
        );
        transfer_channel_ids.insert(
            ("neutron".to_string(), "gaia".to_string()),
            ntrn_to_gaia_transfer_channel.channel_id.to_string(),
        );
        transfer_channel_ids.insert(
            ("gaia".to_string(), "neutron".to_string()),
            gaia_to_ntrn_transfer_channel.channel_id.to_string(),
        );

        let mut ccv_channel_ids = HashMap::new();
        ccv_channel_ids.insert(
            ("gaia".to_string(), "neutron".to_string()),
            gaia_to_ntrn_provider_channel.channel_id,
        );
        ccv_channel_ids.insert(
            ("neutron".to_string(), "gaia".to_string()),
            ntrn_to_gaia_consumer_channel.channel_id,
        );

        let mut ibc_denoms = HashMap::new();
        ibc_denoms.insert(
            ("neutron".to_string(), "stride".to_string()),
            ibc_helpers::get_ibc_denom(
                "untrn",
                &ntrn_to_stride_transfer_channel.channel_id,
            ),
        );
        ibc_denoms.insert(
            ("stride".to_string(), "neutron".to_string()),
            ibc_helpers::get_ibc_denom(
                "ustrd",
                &stride_to_ntrn_transfer_channel.channel_id,
            ),
        );
        ibc_denoms.insert(
            ("gaia".to_string(), "stride".to_string()),
            ibc_helpers::get_ibc_denom(
                "uatom",
                &gaia_to_stride_transfer_channel.channel_id,
            ),
        );
        ibc_denoms.insert(
            ("stride".to_string(), "gaia".to_string()),
            ibc_helpers::get_ibc_denom(
                "ustrd",
                &stride_to_gaia_transfer_channel.channel_id,
            ),
        );
        ibc_denoms.insert(
            ("neutron".to_string(), "gaia".to_string()),
            ibc_helpers::get_ibc_denom(
                "untrn",
                &ntrn_to_gaia_transfer_channel.channel_id,
            ),
        );
        ibc_denoms.insert(
            ("gaia".to_string(), "neutron".to_string()),
            ibc_helpers::get_ibc_denom(
                "uatom",
                &gaia_to_ntrn_transfer_channel.channel_id,
            ),
        );

        Self {
            chains: chains_map,
            transfer_channel_ids,
            ccv_channel_ids,
            connection_ids,
            ibc_denoms,
        }
    }
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
    pub fn new(
        rb: ChainRequestBuilder,
        admin_addr: String,
        native_denom: String,
        channels: Vec<Channel>,
    ) -> Self {
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
