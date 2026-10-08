use std::sync::Arc;
use std::time::Instant;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::json;
use vagus_core::{CollectionDescriptor, CollectionStore, SampleContext};

/// A test item. Its doc comments end up in the schema.
#[derive(Serialize, JsonSchema)]
struct Item {
    /// Item id.
    id: u32,
    /// Optional label.
    label: Option<String>,
}

#[test]
fn descriptor_carries_the_item_schema() {
    let descriptor = CollectionDescriptor::new::<Item>("test.items");
    let schema = serde_json::to_value(&descriptor.item_schema).unwrap();

    assert_eq!(descriptor.id, "test.items");
    assert_eq!(schema["properties"]["id"]["description"], "Item id.");
    assert!(schema["properties"]["label"].is_object());
    assert_eq!(schema["required"], json!(["id"]));
}

#[test]
fn context_publishes_into_the_store() {
    let store = CollectionStore::default();
    let ctx = SampleContext::new(7, Instant::now(), &store);
    ctx.publish("test.items", Arc::new(vec![Item { id: 1, label: None }]));

    assert_eq!(ctx.tick(), 7);
    let items = store.latest("test.items").unwrap().to_json().unwrap();
    assert_eq!(items, json!([{ "id": 1, "label": null }]));
}
