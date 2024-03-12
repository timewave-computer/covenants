#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

#[cfg(test)]
pub mod setup;

#[cfg(test)]
pub mod test;

#[cfg(test)]
pub mod test_astroport_liquid_pooler;
#[cfg(test)]
pub mod test_ibc_forwarder;
#[cfg(test)]
pub mod test_interchain_router;
#[cfg(test)]
pub mod test_native_router;
#[cfg(test)]
pub mod test_native_splitter;
#[cfg(test)]
pub mod test_osmo_lp_outpost;
#[cfg(test)]
pub mod test_remote_chain_splitter;
#[cfg(test)]
pub mod test_single_party_covenant;
#[cfg(test)]
pub mod test_single_party_holder;
#[cfg(test)]
pub mod test_swap_covenant;
#[cfg(test)]
pub mod test_swap_holder;
#[cfg(test)]
pub mod test_two_party_covenant;
#[cfg(test)]
pub mod test_two_party_pol_holder;
