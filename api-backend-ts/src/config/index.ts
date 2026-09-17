import "dotenv/config";
import { z } from "zod";

const envSchema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]),
  PORT: z.coerce.number(),
  LOG_LEVEL: z.string(),

  DB_NODES_URL: z.string().url(),
  DB_LOGS_URL: z.string().url(),
  DB_ALERTS_URL: z.string().url(),

  KAFKA_BROKERS: z.string(),
  KAFKA_GROUP_ID: z.string(),
  KAFKA_TOPIC_ALERTS: z.string(),
  KAFKA_TOPIC_HEALTH: z.string(),

  JWT_SECRET: z.string().min(1),
  JWT_ACCESS_EXPIRES_IN: z.string(),
  JWT_REFRESH_EXPIRES_IN: z.string(),

  CORS_ORIGINS: z.string(),

  RATE_LIMIT_MAX: z.coerce.number(),
  RATE_LIMIT_WINDOW_MS: z.coerce.number(),
});

const env = envSchema.parse(process.env);

export type Config = typeof env;

export const config: Config = env;