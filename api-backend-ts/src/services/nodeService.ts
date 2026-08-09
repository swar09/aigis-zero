import type { Kysely } from "kysely";

import { nodesDb } from "../config/database";
import type { Database } from "../types/db";

export interface NodeSummary {
  nodeId: string;
  machineId: string;
  hostname: string;
  osVersion: string;
  agentVersion: string;
  agentStatus: string;
  operatorStatus: string;
  firstSeenAt: Date;
  lastEnrolledAt: Date;
}

export class NodeService {
  constructor(
    private readonly db: Kysely<Database> = nodesDb,
  ) {}

  async getNodes(): Promise<NodeSummary[]> {
    const nodes = await this.db
      .selectFrom("nodes")
      .select([
        "node_id",
        "machine_id",
        "hostname",
        "os_version",
        "agent_version",
        "agent_status",
        "operator_status",
        "first_seen_at",
        "last_enrolled_at",
      ])
      .orderBy("hostname")
      .execute();

    return nodes.map((node) => ({
      nodeId: node.node_id,
      machineId: node.machine_id,
      hostname: node.hostname,
      osVersion: node.os_version,
      agentVersion: node.agent_version,
      agentStatus: node.agent_status,
      operatorStatus: node.operator_status,
      firstSeenAt: node.first_seen_at,
      lastEnrolledAt: node.last_enrolled_at,
    }));
  }
}

export const nodeService = new NodeService();