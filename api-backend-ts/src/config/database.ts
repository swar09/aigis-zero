import { Kysely, PostgresDialect } from "kysely";
import { Pool } from "pg";

import { config } from "./index";
import type { Database } from "../types/db";

function createDatabase(connectionString: string) {
  return new Kysely<Database>({
    dialect: new PostgresDialect({
      pool: new Pool({
        connectionString,
      }),
    }),
  });
}

export const nodesDb = createDatabase(config.DB_NODES_URL);

export const logsDb = createDatabase(config.DB_LOGS_URL);

export const alertsDb = createDatabase(config.DB_ALERTS_URL);