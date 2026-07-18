# Aigis-Zero API Backend Examples

This directory contains standalone, runnable examples demonstrating core client interactions and testing utilities for `edr-api-backend`.

## Available Examples

| Example | Command | Description |
|---|---|---|
| **JWT Generation & Verification** | `cargo run --example generate_jwt` | Demonstrates creating signed HMAC-SHA256 operator tokens and parsing JWT claims. |
| **Live WebSocket Feed Listener** | `cargo run --example ws_listener` | Connects to `/api/v1/ws`, subscribes to live logs and alerts, and streams incoming JSON events. |

## Running Examples Locally

1. Start the API backend in one terminal:
   ```bash
   cargo run --bin edr-api-backend
   ```
2. Run the WebSocket listener in another terminal:
   ```bash
   cargo run --example ws_listener
   ```
