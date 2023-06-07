use cosmwasm_std::Addr;
use cw_fifo::FIFOQueue;

pub(crate) const QUEUE: FIFOQueue<Addr> = FIFOQueue::new("front", "back");
