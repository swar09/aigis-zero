import { Generated } from "kysely";

export interface OperatorUsersTable {
  user_id: Generated<string>;

  email: string;

  password_hash: string;

  display_name: string;

  role: string;

  is_active: boolean;

  created_at: Date;

  last_login_at: Date | null;

  created_by: string | null;
}
export interface AuditLogTable {
  audit_id: Generated<string>;

  user_id: string | null;

  action: string;

  resource_type: string | null;

  resource_id: string | null;

  detail: unknown | null;

  ip_address: string | null;

  user_agent: string | null;

  created_at: Date;
}
export interface RefreshTokensTable {
  token_id: Generated<string>;

  user_id: string;

  token_hash: string;

  expires_at: Date;

  revoked: boolean;

  created_at: Date;
}
export interface EdrLogsTable {
  log_id: Generated<string>;

  node_id: string;

  event_type: string;

  hostname: string;

  timestamp_ns: bigint;

  payload: unknown;

  raw_sequence: string | null;

  ingested_at: Date;
}
export interface EdrAlertsTable {
  alert_id: Generated<string>;

  node_id: string;

  hostname: string;

  timestamp_ns: bigint;

  severity: string;

  source: string;

  mitre_technique_id: string | null;

  mitre_tactic: string | null;

  description: string;

  triggering_event_id: string | null;

  threat_score: number | null;

  status: string;

  acknowledged_by: string | null;

  acknowledged_at: Date | null;

  dismiss_reason: string | null;

  created_at: Date;
}
export interface YaraRulesTable {
  rule_id: Generated<string>;

  name: string;

  description: string | null;

  content: string;

  tags: string[];

  severity: string;

  mitre_ids: string[];

  is_active: boolean;

  created_by: string | null;

  created_at: Date;

  updated_at: Date;

  version: number;
}
export interface MitreTechniquesTable {
  technique_id: string;

  tactic: string;

  name: string;

  description: string | null;

  url: string | null;

  platform: string[] | null;

  data_sources: string[] | null;
}
export interface NodesTable {
  node_id: string;

  machine_id: string;

  hostname: string;

  os_version: string;

  agent_version: string;

  agent_status: string;

  operator_status: string;

  first_seen_at: Date;

  last_enrolled_at: Date;
}
export interface EnrollmentEventsTable {
  event_id: string;

  node_id: string;

  event_type: string;

  hostname: string;

  os_version: string;

  agent_version: string;

  enrolled_at: Date;
}
export interface NodeHealthTable {
  health_id: string;

  node_id: string;

  agent_status: string;

  events_buffered: bigint;

  recorded_at: Date;
}
export interface Database {
  operator_users: OperatorUsersTable;
  audit_log: AuditLogTable;
  refresh_tokens: RefreshTokensTable;
  edr_logs: EdrLogsTable;
  edr_alerts: EdrAlertsTable;
  yara_rules: YaraRulesTable;
  mitre_techniques: MitreTechniquesTable;
  nodes: NodesTable;
  enrollment_events: EnrollmentEventsTable;
  node_health: NodeHealthTable;
}