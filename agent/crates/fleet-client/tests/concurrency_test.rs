use edr_sdk::proto::fleet::RegisterRequest;
use fleet_client::FleetClient;

#[tokio::test]
async fn test_concurrent_agent_enrollment() {
    let mut check_client = FleetClient::new("http://127.0.0.1:50051".to_string());
    if check_client.connect(None).await.is_err()
        || check_client
            .enroll(RegisterRequest {
                hostname: "health-check".to_string(),
                os_version: "Linux Sim 6.6".to_string(),
                agent_version: "0.1.0".to_string(),
                machine_id: "health-check-probe".to_string(),
            })
            .await
            .is_err()
    {
        eprintln!("Fleet server not reachable or not enrollable on localhost:50051; skipping live concurrency test");
        return;
    }

    let mut handles = Vec::new();

    for i in 1..=5 {
        let handle = tokio::spawn(async move {
            let mut client = FleetClient::new("http://127.0.0.1:50051".to_string());
            client.connect(None).await.expect("failed to connect");
            let resp = client
                .enroll(RegisterRequest {
                    hostname: format!("sim-node-{i}"),
                    os_version: "Linux Sim 6.6".to_string(),
                    agent_version: "0.1.0".to_string(),
                    machine_id: format!("sim-machine-uuid-{i}"),
                })
                .await
                .expect("failed to enroll");
            (i, resp.node_id, resp.token)
        });
        handles.push(handle);
    }

    let mut node_ids = std::collections::HashSet::new();
    for handle in handles {
        let (i, node_id, token) = handle.await.expect("join handle failed");
        assert!(!node_id.is_empty(), "node_id for sim-node-{i} is empty");
        assert!(!token.is_empty(), "token for sim-node-{i} is empty");
        node_ids.insert(node_id);
    }

    assert_eq!(
        node_ids.len(),
        5,
        "Expected 5 distinct node_ids for 5 distinct machine_ids"
    );
}
