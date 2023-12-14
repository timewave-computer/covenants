use cosmwasm_std::{Addr, StdError, Uint64};
use covenant_clock_tester::msg::Mode;

use crate::contract::DEFAULT_TICK_MAX_GAS;
use crate::error::ContractError;

use super::is_error;
use super::suite::SuiteBuilder;

#[test]
fn test_instantiate() {
    let suite = SuiteBuilder::default().build();
    assert_eq!(suite.query_tick_max_gas(), DEFAULT_TICK_MAX_GAS);
    assert!(!suite.query_paused());
}

#[test]
fn test_instanitate_with_zero_tick_max_gas_should_default_to_min() {
    let suite = SuiteBuilder::default().with_tick_max_gas(0).build();

    assert_eq!(suite.query_tick_max_gas(), Uint64::new(200_000));
}

#[test]
fn test_instantiate_with_max_gas_exceeding_limit_defaults_to_limit() {
    let suite = SuiteBuilder::default().with_tick_max_gas(4_000_000).build();
    assert_eq!(suite.query_tick_max_gas(), Uint64::new(3_000_000));
}

// adds an erroring and non-erroring tick receiver to the
// clock. repeatedly calls tick and checks that they get moved
// around. also checks the non-erroring ones receive the ticks.
//
// also tests that the IsQueued query works (and in listing the queue,
// tests the Queue query).
#[test]
fn test_queue() {
    let mut suite_builder = SuiteBuilder::default();

    let non_erroring = suite_builder.generate_tester(Mode::Accept);
    let erroring = suite_builder.generate_tester(Mode::Error);

    let mut suite = suite_builder
        .with_whitelist(vec![non_erroring.clone(), erroring.clone()])
        .build();

    // Enqueue an element and check that it is in the queue. Then
    // enqueue the other and check that it is next in line.
    let queue = suite.enqueue(non_erroring.as_str()).unwrap();
    assert_eq!(queue[0], non_erroring);
    let queue = suite.enqueue(erroring.as_str()).unwrap();
    assert_eq!(queue[1], erroring);

    // Send a tick which ought to reverse the order of our queue
    // elements and increment the tick counter for the non-erroring
    // member.
    suite.tick().unwrap();

    // This fails because when the element is immediately enqueued, it
    // looks at the block's timestamp and notices that there is
    // nothing there (as the other one is stored at timestamp + 1). to
    // get around this, the queue likely needs to be modified to
    // follow the logic of cw-storage-plus. or perhaps that queue can
    // be used with a secondary index for reverse mappings.
    let queue = suite.query_queue_in_order_of_output();
    assert_eq!(queue[0], erroring);
    assert_eq!(queue[1], non_erroring);

    // Ticking again causes another rotation.
    suite.tick().unwrap();
    let queue = suite.query_queue_in_order_of_output();
    assert_eq!(queue[0], non_erroring);
    assert_eq!(queue[1], erroring);

    // Remove an item and verify that ticks work when there is one
    // element in the queue.
    let queue = suite.dequeue(non_erroring.as_str()).unwrap();
    assert_eq!(queue, vec![erroring.clone()]);

    suite.tick().unwrap();
    let queue = suite.dequeue(erroring.as_str()).unwrap();
    assert_eq!(queue.len(), 0);

    // Ticks work but do nothing if there are no elements in the queue.
    suite.tick().unwrap();

    // Check that the testers received the expected number of ticks.
    let non_erroring_tick_count = suite.query_tester_tick_count(&non_erroring);
    assert_eq!(non_erroring_tick_count, 1);
    let erroring_tick_count = suite.query_tester_tick_count(&erroring);
    assert_eq!(erroring_tick_count, 0);
}

// checks that no execute messages can be called while the contract is
// paused, and that they may be called once the contract is unpaused.
#[test]
fn test_pause() {
    let mut suite_builder = SuiteBuilder::default();
    let tester_one = suite_builder.generate_tester(Mode::Accept);
    let tester_two = suite_builder.generate_tester(Mode::Accept);

    let mut suite = suite_builder
        .with_whitelist(vec![tester_one.clone(), tester_two.clone()])
        .build();

    suite.enqueue(tester_one.as_str()).unwrap();

    // pause the clock. no execute messages should be allowed.
    suite.pause().unwrap();

    let res = suite.enqueue(tester_two.as_str());
    is_error!(res, "the contract is paused");
    let res = suite.dequeue(tester_two.as_str());
    is_error!(res, "the contract is paused");
    let res = suite.tick();
    is_error!(res, "the contract is paused");

    // unpause the clock. messages are now allowed.
    suite.unpause().unwrap();

    suite.enqueue(tester_two.as_str()).unwrap();
    suite.dequeue(tester_one.as_str()).unwrap();
    suite.tick().unwrap();
    let queue = suite.query_queue_in_order_of_output();
    assert_eq!(queue, vec![tester_two]);
}

// tests that the tick max gas can be updated and queried for the
// updated values. also checks that tick_max_gas may not be set to
// zero.
#[test]
fn test_update_tick_max_gas() {
    let mut suite = SuiteBuilder::default().build();

    let tmg = suite.query_tick_max_gas();
    suite
        .update_tick_max_gas(tmg.checked_add(Uint64::one()).unwrap())
        .unwrap();
    assert_eq!(
        suite.query_tick_max_gas(),
        tmg.checked_add(Uint64::one()).unwrap()
    );

    let res = suite.update_tick_max_gas(Uint64::zero());
    is_error!(res, "tick max gas must be non-zero")
}

// tests that dequeueing an address that is not in the queue results
// in an error.
#[test]
fn test_dequeue_nonexistant() {
    let mut suite = SuiteBuilder::default().build();
    let err: ContractError = suite.dequeue("nobody").unwrap_err().downcast().unwrap();
    assert!(matches!(
        err,
        ContractError::Std(StdError::NotFound { kind: _ })
    ));
}

// the same tick receiver can not be in the queue more than once.
#[test]
#[should_panic(expected = "sender is already in the queue")]
fn test_enqueue_twice() {
    let mut suite_builder = SuiteBuilder::default();
    let receiver = suite_builder.generate_tester(Mode::Accept);

    let mut suite = suite_builder.with_whitelist(vec![receiver.clone()]).build();

    suite.enqueue(receiver.as_str()).unwrap();
    suite.enqueue(receiver.as_str()).unwrap();
}

// only contract addresses can be enqueued.
#[test]
#[should_panic(expected = "only contracts may be enqueued. error reading contract info:")]
fn test_enqueue_non_contract() {
    let mut suite = SuiteBuilder::default()
        .with_whitelist(vec![Addr::unchecked("nobody")])
        .build();
    suite.enqueue("nobody").unwrap();
}

#[test]
fn test_whitelist() {
    let mut suite_builder = SuiteBuilder::default();
    let receiver = suite_builder.generate_tester(Mode::Accept);

    let suite = suite_builder.with_whitelist(vec![receiver.clone()]).build();

    let whitelist = suite.query_whitelist();

    assert_eq!(whitelist, vec![receiver]);
}

// only contract addresses can be enqueued.
#[test]
fn test_enqueue_non_whitelisted() {
    let mut suite_builder = SuiteBuilder::default();
    let receiver = suite_builder.generate_tester(Mode::Accept);
    let receiver_two = suite_builder.generate_tester(Mode::Accept);

    let mut suite = suite_builder.with_whitelist(vec![receiver.clone()]).build();

    suite.enqueue(receiver.as_str()).unwrap();
    let res = suite.enqueue(receiver_two.as_str());
    is_error!(res, "Caller is not whitelisted, can't enqueue")
}

#[test]
fn test_manage_whitelisted() {
    let mut suite_builder = SuiteBuilder::default();
    let receiver = suite_builder.generate_tester(Mode::Accept);
    let receiver_two = suite_builder.generate_tester(Mode::Accept);

    let mut suite = suite_builder.with_whitelist(vec![receiver.clone()]).build();

    suite.enqueue(receiver.as_str()).unwrap();
    suite.enqueue(receiver_two.as_str()).unwrap_err();

    let queue = suite.query_full_queue();
    assert_eq!(queue.len(), 1);

    // Add new contract to clock whitelist
    suite
        .manage_whitelisted(Some(vec![receiver_two.to_string()]), None)
        .unwrap();

    let res = suite.enqueue(receiver_two.as_str());
    is_error!(res, "sender is already in the queue");

    // Make sure queue is 2 now
    let queue = suite.query_full_queue();
    assert_eq!(queue.len(), 2);

    suite
        .manage_whitelisted(None, Some(vec![receiver.to_string()]))
        .unwrap();
    let queue = suite.query_full_queue();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].0, receiver_two);
}
