import Fastify from "fastify";
import { healthRoutes } from "./routes/health";
import { nodeRoutes } from "./routes/v1/nodes";
import { errorHandler } from "./middleware/errorHandler";

import helmet from "@fastify/helmet";
import cors from "@fastify/cors";
import rateLimit from "@fastify/rate-limit";

import { config } from "./config";

export async function buildApp() {
  const app = Fastify({
    logger: {
      level: config.LOG_LEVEL,
    },
  });

  await app.register(helmet);
  await app.register(healthRoutes);
  await app.register(nodeRoutes, {
  prefix: "/api/v1/nodes",
});

  await app.register(cors, {
    origin: config.CORS_ORIGINS.split(","),
  });

  await app.register(rateLimit, {
    max: config.RATE_LIMIT_MAX,
    timeWindow: config.RATE_LIMIT_WINDOW_MS,
  });

  app.setErrorHandler(errorHandler);

  return app;
}