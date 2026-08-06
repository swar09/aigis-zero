use event_buffer::EventBuffer;

#[tokio::test]
async fn test_event_buffer_flow() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("events.db");

    // Create buffer with max_events = 3
    let buffer = EventBuffer::new(&db_path, 3).unwrap();

    // Check empty
    assert!(buffer.is_empty().await.unwrap());
    assert_eq!(buffer.len().await.unwrap(), 0);

    // Push 4 events (should evict the first one because max_events=3)
    buffer.push("event 1".to_string()).await.unwrap();
    buffer.push("event 2".to_string()).await.unwrap();
    buffer.push("event 3".to_string()).await.unwrap();
    buffer.push("event 4".to_string()).await.unwrap();

    // Check length is 3 (max capacity)
    assert_eq!(buffer.len().await.unwrap(), 3);

    // Drain 2 events
    let drained = buffer.drain(2).await.unwrap();
    assert_eq!(drained.len(), 2);

    // Since event 1 was evicted, the oldest remaining is event 2
    assert_eq!(drained[0], "event 2");
    assert_eq!(drained[1], "event 3");

    // Length should now be 1
    assert_eq!(buffer.len().await.unwrap(), 1);

    // Drain remaining
    let remaining = buffer.drain(5).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], "event 4");

    // Empty again
    assert!(buffer.is_empty().await.unwrap());
}
