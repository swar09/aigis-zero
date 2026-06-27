/// CLI tool for Kafka topic administration
/// Usage:
///   kafka-admin create-topics --brokers localhost:29092
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use std::env;

struct TopicSpec {
    name: &'static str,
    partitions: i32,
    retention_ms: i64,
}

const TOPICS: &[TopicSpec] = &[
    TopicSpec {
        name: "aigis.events.raw",
        partitions: 64,
        retention_ms: 604800000,
    },
    TopicSpec {
        name: "aigis.events.process",
        partitions: 32,
        retention_ms: 1209600000,
    },
    TopicSpec {
        name: "aigis.events.network",
        partitions: 32,
        retention_ms: 1209600000,
    },
    TopicSpec {
        name: "aigis.events.file",
        partitions: 16,
        retention_ms: 1209600000,
    },
    TopicSpec {
        name: "aigis.events.auth",
        partitions: 16,
        retention_ms: 2592000000,
    },
    TopicSpec {
        name: "aigis.heartbeats",
        partitions: 8,
        retention_ms: 259200000,
    },
    TopicSpec {
        name: "aigis.alerts",
        partitions: 8,
        retention_ms: 7776000000,
    },
    TopicSpec {
        name: "aigis.events.dlq",
        partitions: 4,
        retention_ms: 2592000000,
    },
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: kafka-admin <command> --brokers <brokers> [--topic <topic>]");
        return Ok(());
    }

    let command = &args[1];

    // Parse flags in any order
    let mut brokers_opt: Option<&str> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--brokers" => {
                if i + 1 < args.len() {
                    brokers_opt = Some(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --brokers requires a value");
                    return Ok(());
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    let brokers = match brokers_opt {
        Some(b) => b,
        None => {
            eprintln!("Error: --brokers flag is required");
            return Ok(());
        }
    };

    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", brokers);

    let admin_client: AdminClient<DefaultClientContext> = config.create()?;

    match command.as_str() {
        "create-topics" => {
            let retention_strings: Vec<String> =
                TOPICS.iter().map(|t| t.retention_ms.to_string()).collect();
            let mut new_topics = Vec::new();
            for (i, topic) in TOPICS.iter().enumerate() {
                let new_topic =
                    NewTopic::new(topic.name, topic.partitions, TopicReplication::Fixed(1))
                        .set("retention.ms", &retention_strings[i])
                        .set("cleanup.policy", "delete");
                new_topics.push(new_topic);
            }

            let opts = AdminOptions::new();
            let results = admin_client.create_topics(&new_topics, &opts).await?;
            for result in results {
                match result {
                    Ok(topic_name) => println!("Created topic: {}", topic_name),
                    Err((topic_name, err)) => {
                        eprintln!("Failed to create topic {}: {:?}", topic_name, err)
                    }
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
        }
    }

    Ok(())
}
