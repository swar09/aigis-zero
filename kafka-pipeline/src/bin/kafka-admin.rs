/// CLI tool for Kafka topic administration
/// Usage:
///   kafka-admin create-topics --brokers localhost:29092
///   kafka-admin verify-topics --brokers localhost:29092
///   kafka-admin describe-topic --brokers localhost:29092 --topic aigis.events.raw
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

/// Administers Kafka topics by executing a command specified via CLI arguments.
///
/// Parses command-line arguments to extract the command name and `--brokers` option, creates
/// an admin client connected to the specified brokers, and dispatches to the command handler.
/// Currently supports `create-topics`, which creates the predefined set of topics with their
/// configured partition counts, retention periods, and cleanup policies. If fewer than four
/// arguments are provided, prints usage information without performing any action. Prints a
/// message for each topic creation attempt, indicating success or failure.
///
/// # Examples
///
/// ```no_run
/// // Invoke the binary with:
/// // cargo run -- create-topics --brokers localhost:9092
/// ```
///
/// # Errors
///
/// Returns an error if the admin client cannot be created or if the topic creation request fails.
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: kafka-admin <command> --brokers <brokers> [--topic <topic>]");
        return Ok(());
    }

    let command = &args[1];
    let brokers = &args[3]; // assuming --brokers is args[2]

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
