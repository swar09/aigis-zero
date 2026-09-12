export type AgentStatus = 'healthy' | 'degraded' | 'offline' | 'online';
export type OperatorStatus = 'active' | 'isolated' | 'normal';
export type AlertSeverity = 'critical' | 'high' | 'medium' | 'low';
export type AlertStatus = 'open' | 'acknowledged' | 'dismissed' | 'active' | 'investigating' | 'resolved' | 'false_positive';

export interface NodeSummaryDto {
  node_id: string;
  machine_id: string;
  hostname: string;
  os_version: string;
  agent_version: string;
  agent_status: AgentStatus | string;
  operator_status: OperatorStatus | string;
  first_seen_at: string;
  last_enrolled_at: string;
}

export interface NodeHealthEntity {
  node_id: string;
  recorded_at: string;
  agent_status: string;
  events_buffered?: number;
  events_dropped?: number;
  process_memory_bytes?: number;
  process_cpu_percent?: number;
  disk_free_bytes?: number;
}

export interface NodeDetailDto {
  node: NodeSummaryDto;
  health_history?: NodeHealthEntity[];
}

export interface AlertEntity {
  alert_id: string;
  node_id: string;
  hostname: string;
  severity: AlertSeverity | string;
  source: string;
  mitre_technique_id: string | null;
  mitre_tactic: string | null;
  description: string;
  triggering_event_id: string | null;
  threat_score: number;
  status: AlertStatus | string;
  created_at: string;
}

export interface EventLogEntity {
  event_id: string;
  node_id: string;
  event_type: string;
  hostname: string;
  payload: Record<string, unknown>;
  raw_sequence_id: string | null;
  recorded_at: string;
}

export interface LiveLogData {
  node_id: string;
  hostname: string;
  event_type: string;
  payload: Record<string, unknown>;
  timestamp_ns: number;
}

export interface LiveAlertData {
  id: string;
  node_id: string;
  hostname: string;
  severity: string;
  mitre_technique_id: string | null;
  description: string;
  threat_score: number;
  timestamp_ns: number;
}

export interface LiveHeartbeatData {
  node_id: string;
  agent_status: string;
  events_buffered: number;
  timestamp_ns: number;
}

export type LiveEvent =
  | { type: 'log'; data: LiveLogData }
  | { type: 'alert'; data: LiveAlertData }
  | { type: 'heartbeat'; data: LiveHeartbeatData };

export interface ApiResponse<T> {
  data: T;
  meta: {
    timestamp: string;
  };
  success: boolean;
}

export interface PaginatedResult<T> {
  items: T[];
  total: number | null;
}

export interface UserInfo {
  username: string;
  role: string;
  permissions: string[];
}

export interface LoginResponseData {
  token: string;
  token_type: string;
  expires_in: number;
  user: UserInfo;
}

export interface IsolateNodeResponse {
  node_id: string;
  operator_status: string;
  updated_at: string;
}

export interface UpdateAlertStatusResponse {
  alert_id: string;
  status: string;
  updated_at: string;
}
