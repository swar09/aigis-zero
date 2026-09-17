import { FastifyInstance } from "fastify";

import { nodeService } from "../../services/nodeService";

export async function nodeRoutes(app: FastifyInstance) {
  app.get("/", async (_request, reply) => {
    const nodes = await nodeService.getNodes();

    return reply.send(nodes);
  });
}