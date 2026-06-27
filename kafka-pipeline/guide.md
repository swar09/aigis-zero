# Aigis Kafka Pipeline Guide

This guide covers the setup, testing, troubleshooting, and future development of the `edr-kafka-pipeline` component.

## 1. Setup & Initialization

The Kafka pipeline requires a running Kafka broker and pre-configured topics to operate properly.

### Starting the Infrastructure
Use the provided Docker Compose configuration to spin up the development Kafka cluster (KRaft mode) and Kafka-UI:

```bash
cd infra
docker compose -f docker-compose.dev.yml up -d
```

### Initializing Topics
Once the cluster is running, execute the topic creation script. This script connects to the broker and initializes the partitioned topics for the raw event stream, domain-specific streams, dead-letter queue, and alerts:

```bash
bash infra/scripts/create-topics.sh
```

## 2. Testing the Pipeline

### Local Development Run
To test the pipeline locally outside of Docker, use Cargo. Ensure you specify the correct binary name (`edr-kafka-pipeline`):

```bash
cd kafka-pipeline
KAFKA_BROKERS="localhost:29092" cargo run --release --bin edr-kafka-pipeline
```

### Simulating End-to-End Routing
You can verify the event router is working by manually producing a message to the raw topic and consuming it from its target routed topic.

1. **Produce a raw event** (this simulates an incoming event from the EDR agent):
   ```bash
   docker exec -it aigis-kafka-dev bash -c 'echo "{\"event_type\": \"process_start\", \"process_name\": \"malware.exe\"}" | /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server localhost:9092 --topic aigis.events.raw'
   ```

2. **Consume the routed event** (the pipeline should have instantly moved it to `aigis.events.process`):
   ```bash
   docker exec -it aigis-kafka-dev /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic aigis.events.process --from-beginning --max-messages 1
   ```

### Running Unit & Integration Tests
```bash
cargo test -p edr-kafka-pipeline
```

## 3. Troubleshooting

### `CMake` Missing Error During Build
**Symptom:** The `rdkafka-sys` dependency fails to build with an OS Error 2 (No such file or directory) complaining about `cmake`.
**Solution:** The `librdkafka` C-library requires CMake and a C++ compiler to compile statically.
- Ubuntu/Debian: `sudo apt-get install cmake g++`
- macOS: `brew install cmake`

### Pipeline Fails to Start: "Broker Transport Failure"
**Symptom:** The pipeline logs repeatedly show connection failures to `localhost:29092` or `localhost:9092`.
**Solution:** 
- Ensure your `docker-compose` cluster is up.
- Verify `KAFKA_BROKERS` is set correctly. If running the pipeline *inside* Docker Compose, it should be `kafka:9092`. If running it locally on your host, it should be `localhost:29092`.

### Timeout / Configuration Property Errors
**Symptom:** `Client config error: No such configuration property`
**Solution:** `librdkafka` properties are strictly enforced. Reference the official [librdkafka configuration properties](https://github.com/confluentinc/librdkafka/blob/master/CONFIGURATION.md) and ensure typos (like `fetch.max.wait.ms` instead of `fetch.wait.max.ms`) are avoided in `consumer.rs`.

## 4. Further Development

### Adding a New Event Type Route
The `EventRouterProcessor` in `src/event_router.rs` maps incoming JSON events to specific Kafka topics.
To add support for a new topic:
1. Ensure the new topic is added to `infra/scripts/create-topics.sh`.
2. Open `src/event_router.rs`.
3. Locate the `route_topic` function.
4. Add a new `match` arm for your new `event_type` string that returns the new target topic name.

### Creating a New Pipeline Component
The pipeline is designed using a modular `MessageProcessor` trait (`src/consumer.rs`). If you want to create a consumer that does something else entirely (e.g., Anomaly Detection instead of routing):
1. Create a new struct that implements the `MessageProcessor` trait.
2. Override the `process` function to define your custom logic.
3. Instantiate a new `ConsumerWorker` in `main.rs` passing your new processor and its designated source topic.
