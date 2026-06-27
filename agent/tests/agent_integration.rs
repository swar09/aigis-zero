#[tokio::test]
#[ignore = "integration test not yet implemented"]
async fn test_agent_integration() {
    // - Start a mock fleet server (using tonic with JsonCodec)
    // - Start the agent
    // - Verify enrollment completes
    // - Verify events are received by mock server
    // - Verify heartbeats are received
    // - Send a command and verify response
}
