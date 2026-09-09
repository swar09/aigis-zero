'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import { LiveEvent } from '../lib/types';
import { getAuthToken } from '../lib/api';

export type ConnectionState = 'connecting' | 'connected' | 'disconnected';

export interface UseLiveEventsOptions {
  topics?: string[];
  nodeId?: string;
  maxEvents?: number;
  enabled?: boolean;
}

export interface UseLiveEventsResult {
  events: LiveEvent[];
  connectionState: ConnectionState;
  totalEventCount: number;
  eventsPerSecond: number;
  clearEvents: () => void;
}

export function useLiveEvents(options: UseLiveEventsOptions = {}): UseLiveEventsResult {
  const {
    topics,
    nodeId,
    maxEvents = 100,
    enabled = true,
  } = options;

  const [events, setEvents] = useState<LiveEvent[]>([]);
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const [totalEventCount, setTotalEventCount] = useState<number>(0);
  const [eventsPerSecond, setEventsPerSecond] = useState<number>(0);

  const socketRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const recentCountRef = useRef<number>(0);
  const pendingBufferRef = useRef<LiveEvent[]>([]);
  const animationFrameRef = useRef<number | null>(null);

  const clearEvents = useCallback(() => {
    setEvents([]);
    pendingBufferRef.current = [];
  }, []);

  // Flush buffer to state on animation frame to prevent layout thrashing
  const scheduleBufferFlush = useCallback(() => {
    if (animationFrameRef.current !== null) return;

    animationFrameRef.current = requestAnimationFrame(() => {
      animationFrameRef.current = null;
      if (pendingBufferRef.current.length === 0) return;

      const toAdd = [...pendingBufferRef.current];
      pendingBufferRef.current = [];

      setEvents((prev) => {
        const next = [...toAdd.reverse(), ...prev];
        return next.slice(0, maxEvents);
      });
    });
  }, [maxEvents]);

  // Calculate events per second every second
  useEffect(() => {
    if (!enabled) return;

    const interval = setInterval(() => {
      setEventsPerSecond(recentCountRef.current);
      recentCountRef.current = 0;
    }, 1000);

    return () => clearInterval(interval);
  }, [enabled]);

  const reconnectAttemptsRef = useRef<number>(0);
  const heartbeatIntervalRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    if (!enabled || typeof window === 'undefined') return;

    let isUnmounted = false;

    function getWebSocketUrl(): string {
      if (process.env.NEXT_PUBLIC_WS_URL) {
        const url = new URL(process.env.NEXT_PUBLIC_WS_URL);
        if (topics && topics.length > 0) url.searchParams.set('topics', topics.join(','));
        if (nodeId) url.searchParams.set('node_id', nodeId);
        return url.toString();
      }

      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      // If running on a local development port (3000-3010) and not proxied, default to API backend port 8088
      const isDevPort = /^30\d\d$/.test(window.location.port);
      const host = isDevPort ? `${window.location.hostname}:8088` : window.location.host;
      const url = new URL(`${protocol}//${host}/api/v1/ws`);

      if (topics && topics.length > 0) {
        url.searchParams.set('topics', topics.join(','));
      }
      if (nodeId) {
        url.searchParams.set('node_id', nodeId);
      }
      const token = getAuthToken();
      if (token) {
        url.searchParams.set('token', token);
      }
      return url.toString();
    }

    function getBackoffDelay(): number {
      const base = 1000;
      const max = 30000;
      const attempts = reconnectAttemptsRef.current;
      const backoff = Math.min(max, base * Math.pow(1.5, attempts));
      const jitter = Math.random() * 500;
      return backoff + jitter;
    }

    function connect() {
      if (isUnmounted) return;
      setConnectionState('connecting');

      const url = getWebSocketUrl();
      const socket = new WebSocket(url);
      socketRef.current = socket;

      socket.onopen = () => {
        if (isUnmounted) {
          socket.close();
          return;
        }
        setConnectionState('connected');
        reconnectAttemptsRef.current = 0;

        // Keep-alive heartbeat ping every 30 seconds
        if (heartbeatIntervalRef.current) clearInterval(heartbeatIntervalRef.current);
        heartbeatIntervalRef.current = setInterval(() => {
          if (socket.readyState === WebSocket.OPEN) {
            try {
              socket.send(JSON.stringify({ action: 'ping' }));
            } catch {
              // Ignore send failure on closing socket
            }
          }
        }, 30000);
      };

      socket.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data) as { type?: string; [key: string]: unknown };
          if (parsed && typeof parsed === 'object' && parsed.type) {
            // Server control messages (pong, subscribed, error)
            if (parsed.type === 'pong' || parsed.type === 'subscribed' || parsed.type === 'error') {
              return;
            }
            recentCountRef.current += 1;
            setTotalEventCount((count) => count + 1);
            pendingBufferRef.current.push(parsed as unknown as LiveEvent);
            scheduleBufferFlush();
          }
        } catch {
          // Discard unparsable frames
        }
      };

      socket.onclose = () => {
        if (heartbeatIntervalRef.current) {
          clearInterval(heartbeatIntervalRef.current);
          heartbeatIntervalRef.current = null;
        }
        if (isUnmounted) return;
        setConnectionState('disconnected');
        socketRef.current = null;

        const delay = getBackoffDelay();
        reconnectAttemptsRef.current += 1;
        reconnectTimeoutRef.current = setTimeout(connect, delay);
      };

      socket.onerror = () => {
        if (socket.readyState === WebSocket.OPEN) {
          socket.close();
        }
      };
    }

    connect();

    return () => {
      isUnmounted = true;
      if (heartbeatIntervalRef.current) {
        clearInterval(heartbeatIntervalRef.current);
      }
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (animationFrameRef.current !== null) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      if (socketRef.current) {
        socketRef.current.close();
      }
    };
  }, [enabled, nodeId, scheduleBufferFlush, topics]);

  return {
    events,
    connectionState,
    totalEventCount,
    eventsPerSecond,
    clearEvents,
  };
}
