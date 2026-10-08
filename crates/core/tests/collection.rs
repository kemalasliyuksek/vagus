use std::sync::Arc;

use serde_json::json;
use vagus_core::{CollectionStore, DoubleBuffer};

#[test]
fn store_returns_the_latest_view_of_each_collection() {
    let store = CollectionStore::default();
    assert!(store.latest("test.numbers").is_none());

    store.publish("test.numbers", Arc::new(vec![1, 2]));
    store.publish("test.names", Arc::new(vec!["a"]));
    store.publish("test.numbers", Arc::new(vec![3]));

    let numbers = store.latest("test.numbers").unwrap().to_json().unwrap();
    assert_eq!(numbers, json!([3]));
    let names = store.latest("test.names").unwrap().to_json().unwrap();
    assert_eq!(names, json!(["a"]));
}

#[test]
fn a_reader_keeps_its_view_after_a_new_publish() {
    let store = CollectionStore::default();
    store.publish("test.numbers", Arc::new(vec![1]));
    let held = store.latest("test.numbers").unwrap();

    store.publish("test.numbers", Arc::new(vec![2]));

    assert_eq!(held.to_json().unwrap(), json!([1]));
}

#[test]
fn double_buffer_reuses_a_snapshot_nobody_holds() {
    let mut buffer: DoubleBuffer<Vec<u32>> = DoubleBuffer::new();
    buffer.next_mut().extend([1, 2, 3]);
    drop(buffer.swap());
    let next = buffer.next_mut();
    next.clear();
    next.push(4);
    drop(buffer.swap());

    // Both buffers are free again, so the first one comes back with its contents and
    // its allocation.
    assert_eq!(*buffer.next_mut(), [1, 2, 3]);
}

#[test]
fn double_buffer_never_writes_to_a_snapshot_a_reader_holds() {
    let mut buffer: DoubleBuffer<Vec<u32>> = DoubleBuffer::new();
    buffer.next_mut().push(1);
    let held = buffer.swap();
    buffer.next_mut().push(2);
    drop(buffer.swap());

    // The spare is the snapshot `held` points to, so a fresh buffer is handed out.
    assert!(buffer.next_mut().is_empty());
    buffer.next_mut().push(3);
    assert_eq!(*held, [1]);
    assert_eq!(*buffer.swap(), [3]);
}
