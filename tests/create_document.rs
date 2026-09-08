use fireplace::firestore::collection;

#[tokio::test]
async fn create_document_in_nested_collection() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = fireplace::firestore::test_helpers::initialise().await?;

    let doc_ref = collection("tales")
        .doc(format!("alice-{}", ulid::Ulid::generate()))
        .collection("in")
        .doc("wonderland");

    client
        .create_document_at_ref(
            &doc_ref,
            &serde_json::json!({
                "title": "Alice in Wonderland",
                "author": "Lewis Carroll",
            }),
        )
        .await?;

    let doc = client
        .get_document::<serde_json::Value>(&doc_ref)
        .await?
        .unwrap();

    assert_eq!(doc["title"], "Alice in Wonderland");
    assert_eq!(doc["author"], "Lewis Carroll");

    Ok(())
}
