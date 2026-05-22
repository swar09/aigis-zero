# event-buffer — Implementation Timeline

> **Phase**: 1 (Agent: OSQuery Integration)
> **Priority**: 🟡 High — durability layer, prevents event loss
> **Estimated Duration**: 2–3 days
> **Depends on**: `sdk v0.1.0`

---

## Overview

Write-ahead buffer using sled embedded DB. Events are persisted to disk immediately and only removed after the Fleet Server acknowledges receipt. Ensures zero event loss during network outages.

---

## PR Plan

### PR #1 — Sled storage layer and buffer API
**Branch**: `feat/event-buffer-core`
**Duration**: 1.5 days

**Files**:
- `src/lib.rs` — public API (`EventBuffer`)
- `src/buffer.rs` — sled read/write, flush logic

**Tasks**:
- [ ] Implement `EventBuffer::new(path)` — opens/creates sled DB at path
- [ ] Implement `push(event: NormalisedEvent)` — serialise and store with auto-increment key
- [ ] Implement `peek_batch(n: usize)` → returns oldest N events without removing
- [ ] Implement `ack(up_to_key)` — removes all events up to acknowledged sequence
- [ ] Implement `len()` — returns count of buffered events
- [ ] Implement `flush_all()` — returns all events as iterator for drain on reconnect
- [ ] Handle sled compaction and disk space limits
- [ ] Unit tests: push → peek → ack cycle, crash recovery simulation

### PR #2 — Backpressure and metrics
**Branch**: `feat/event-buffer-backpressure`
**Duration**: 1 day
**Depends on**: PR #1

**Tasks**:
- [ ] Add configurable max buffer size (bytes and count)
- [ ] Implement backpressure signalling when buffer exceeds threshold
- [ ] Add metrics: `events_buffered`, `events_flushed`, `buffer_size_bytes`
- [ ] Log warnings at 80% capacity, errors at 95%
- [ ] Unit tests for backpressure scenarios
