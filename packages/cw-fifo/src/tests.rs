use cosmwasm_std::{testing::mock_dependencies, Addr, BlockInfo, Timestamp};

use crate::FIFOQueue;

fn block_maker() -> impl FnMut() -> BlockInfo {
    let mut t = Timestamp::from_seconds(0);
    move || {
        t = t.plus_seconds(6);
        BlockInfo {
            height: t.seconds(),
            time: t,
            chain_id: String::default(),
        }
    }
}

#[test]
fn test_enqueue_dequeue_dequeue() {
    let mut next_block = block_maker();
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b");

    queue.enqueue(storage, &next_block(), 10).unwrap();
    assert_eq!(queue.dequeue(storage).unwrap(), Some(10));
    assert_eq!(queue.dequeue(storage).unwrap(), None);
}

#[test]
fn test_enqueue_enqueue_remove_dequeue() {
    let mut next_block = block_maker();
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b");

    queue.enqueue(storage, &next_block(), 10).unwrap();
    queue.enqueue(storage, &next_block(), 11).unwrap();
    queue.remove(storage, 10).unwrap();
    assert_eq!(queue.dequeue(storage).unwrap(), Some(11))
}

#[test]
fn test_enqueue_has_dequeue_has() {
    let mut next_block = block_maker();
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b");

    queue
        .enqueue(storage, &next_block(), "hello".to_string())
        .unwrap();
    assert!(queue.has(storage, "hello".to_string()));
    queue.dequeue(storage).unwrap();
    assert!(!queue.has(storage, "hello".to_string()));
}

#[test]
fn test_multi_block_enqueue() {
    let mut next_block = block_maker();
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b");
    let block = next_block();

    queue
        .enqueue(storage, &block, Addr::unchecked("hmm"))
        .unwrap();
    queue
        .enqueue(storage, &block, Addr::unchecked("mmh"))
        .unwrap();

    assert_eq!(
        queue.dequeue(storage).unwrap(),
        Some(Addr::unchecked("hmm"))
    )
}
