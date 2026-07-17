use std::time::Duration;

use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::{
    models::ws::{LiveEvent, WsClientMessage, WsServerMessage},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub topics: Option<String>,
    pub node_id: Option<String>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, query))
}

async fn handle_socket(socket: WebSocket, state: AppState, initial_query: WsQuery) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();
    let mut ping_interval = interval(Duration::from_secs(30));

    let mut current_node_filter = initial_query.node_id;
    let mut current_topic_filters = parse_topics(initial_query.topics.as_deref());

    info!(
        node_id = ?current_node_filter,
        topics = ?current_topic_filters,
        "WebSocket client connected"
    );

    loop {
        tokio::select! {
            event_result = broadcast_rx.recv() => {
                match event_result {
                    Ok(event) => {
                        if matches_filter(&event, &current_node_filter, &current_topic_filters) {
                            let Ok(json_text) = serde_json::to_string(&event) else {
                                continue;
                            };
                            if sender.send(Message::Text(json_text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged by {n} messages, skipping dropped frames");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }

            client_msg = receiver.next() => {
                match client_msg {
                    Some(Ok(Message::Close(_))) | None => {
                        debug!("WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<WsClientMessage>(&text) {
                            match cmd {
                                WsClientMessage::Subscribe { topics, node_id } => {
                                    if let Some(t) = topics {
                                        current_topic_filters = t;
                                    }
                                    current_node_filter = node_id.clone();

                                    let ack = WsServerMessage::Subscribed {
                                        topics: current_topic_filters.clone(),
                                        node_id,
                                    };
                                    if let Ok(ack_json) = serde_json::to_string(&ack) {
                                        let _ = sender.send(Message::Text(ack_json.into())).await;
                                    }
                                }
                                WsClientMessage::Ping => {
                                    let pong = WsServerMessage::Pong {
                                        timestamp: chrono::Utc::now().timestamp_millis(),
                                    };
                                    if let Ok(pong_json) = serde_json::to_string(&pong) {
                                        let _ = sender.send(Message::Text(pong_json.into())).await;
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        warn!(err = %e, "WebSocket communication error");
                        break;
                    }
                    _ => {}
                }
            }

            _ = ping_interval.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }

    info!("WebSocket client disconnected");
}

fn parse_topics(topics: Option<&str>) -> Vec<String> {
    match topics {
        Some(t) => t.split(',').map(|s| s.trim().to_lowercase()).collect(),
        None => vec!["logs".into(), "alerts".into(), "heartbeats".into()],
    }
}

fn matches_filter(event: &LiveEvent, target_node: &Option<String>, topics: &[String]) -> bool {
    let (event_topic, event_node) = match event {
        LiveEvent::Log { node_id, .. } => ("logs", node_id.as_str()),
        LiveEvent::Alert { node_id, .. } => ("alerts", node_id.as_str()),
        LiveEvent::Heartbeat { node_id, .. } => ("heartbeats", node_id.as_str()),
    };

    if !topics.is_empty() && !topics.iter().any(|t| t == event_topic || t == "all") {
        return false;
    }

    if target_node
        .as_deref()
        .is_some_and(|target| target != event_node)
    {
        return false;
    }

    true
}
