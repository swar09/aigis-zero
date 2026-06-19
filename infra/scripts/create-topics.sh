#!/bin/bash
set -euo pipefail

BOOTSTRAP="localhost:29092"
KAFKA_BIN="/opt/kafka/bin"

# create_topic creates a Kafka topic with the specified partitions, retention time, and replication factor in the aigis-kafka-dev container.
create_topic() {
    local topic=$1
    local partitions=$2
    local retention_ms=$3
    local replication=${4:-1}

    echo "Creating topic: $topic (partitions=$partitions, retention=${retention_ms}ms, replication=$replication)"
    docker exec aigis-kafka-dev /opt/kafka/bin/kafka-topics.sh --bootstrap-server kafka:9092 \
        --create --if-not-exists \
        --topic "$topic" \
        --partitions "$partitions" \
        --replication-factor "$replication" \
        --config retention.ms="$retention_ms" \
        --config cleanup.policy=delete
}

# Create all topics
create_topic "aigis.events.raw"      64  604800000   # 7 days
create_topic "aigis.events.process"  32  1209600000  # 14 days
create_topic "aigis.events.network"  32  1209600000  # 14 days
create_topic "aigis.events.file"     16  1209600000  # 14 days
create_topic "aigis.events.auth"     16  2592000000  # 30 days
create_topic "aigis.heartbeats"      8   259200000   # 3 days
create_topic "aigis.alerts"          8   7776000000  # 90 days
create_topic "aigis.events.dlq"      4   2592000000  # 30 days

echo "All topics created successfully"
docker exec aigis-kafka-dev /opt/kafka/bin/kafka-topics.sh --bootstrap-server kafka:9092 --list
