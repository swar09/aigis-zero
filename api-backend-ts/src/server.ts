import { config } from "./config";
import { buildApp } from "./app";

async function start() {
  const app = await buildApp();

  try {
    await app.listen({
      port: config.PORT,
      host: "0.0.0.0",
    });

    app.log.info(`Server listening on port ${config.PORT}`);
  } catch (error) {
    app.log.error(error);
    process.exit(1);
  }
}

start();