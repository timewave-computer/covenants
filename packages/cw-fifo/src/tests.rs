use cosmwasm_std::{testing::mock_dependencies, Addr};

use crate::FIFOQueue;

#[test]
fn test_enqueue_dequeue_dequeue() {
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b", "c");

    queue.enqueue(storage, 10).unwrap();
    assert_eq!(queue.dequeue(storage).unwrap(), Some(10));
    assert_eq!(queue.dequeue(storage).unwrap(), None);
}

#[test]
fn test_enqueue_enqueue_remove_dequeue() {
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b", "s");

    queue.enqueue(storage, 10).unwrap();
    queue.enqueue(storage, 11).unwrap();
    queue.remove(storage, 10).unwrap();
    assert_eq!(queue.dequeue(storage).unwrap(), Some(11))
}

#[test]
fn test_enqueue_has_dequeue_has() {
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b", "s");

    queue.enqueue(storage, "hello".to_string()).unwrap();
    assert!(queue.has(storage, "hello".to_string()));
    queue.dequeue(storage).unwrap();
    assert!(!queue.has(storage, "hello".to_string()));
}

#[test]
fn test_querying_queue() {
    let mut deps = mock_dependencies();
    let storage = &mut deps.storage;

    let queue = FIFOQueue::new("f", "b", "s");

    queue.enqueue(storage, Addr::unchecked("hmm")).unwrap();
    queue.enqueue(storage, Addr::unchecked("mmh")).unwrap();

    // I can query the whole queue.
    let q = queue.query_queue(storage, None, None).unwrap();
    assert_eq!(
        q,
        vec![(Addr::unchecked("hmm"), 0), (Addr::unchecked("mmh"), 1)]
    );
    // I can query the first part of the queue.
    let q = queue.query_queue(storage, None, Some(1)).unwrap();
    assert_eq!(q, vec![(Addr::unchecked("hmm"), 0)]);
    // I can query the second part of the queue.
    let q = queue
        .query_queue(storage, Some(Addr::unchecked("hmm")), None)
        .unwrap();
    assert_eq!(q, vec![(Addr::unchecked("mmh"), 1)]);
}
