use std::num::NonZeroUsize;

use vagus_core::RingBuffer;

fn capacity(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("test capacity must be non-zero")
}

#[test]
fn keeps_items_in_insertion_order_until_full() {
    let mut buffer = RingBuffer::new(capacity(3));

    for sample in 1..=3 {
        assert_eq!(buffer.push(sample), None);
    }

    assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), [1, 2, 3]);
}

#[test]
fn evicts_and_returns_oldest_item_when_full() {
    let mut buffer = RingBuffer::new(capacity(2));
    buffer.push("a");
    buffer.push("b");

    assert_eq!(buffer.push("c"), Some("a"));
    assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), ["b", "c"]);
}

#[test]
fn latest_returns_most_recent_item() {
    let mut buffer = RingBuffer::new(capacity(2));
    assert_eq!(buffer.latest(), None);

    for sample in [10, 20, 30] {
        buffer.push(sample);
    }

    assert_eq!(buffer.latest(), Some(&30));
}

#[test]
fn never_grows_beyond_capacity() {
    let mut buffer = RingBuffer::new(capacity(4));

    for sample in 0..1_000 {
        buffer.push(sample);
    }

    assert_eq!(buffer.len(), 4);
    assert_eq!(buffer.capacity().get(), 4);
    assert_eq!(
        buffer.iter().copied().collect::<Vec<_>>(),
        [996, 997, 998, 999]
    );
}
