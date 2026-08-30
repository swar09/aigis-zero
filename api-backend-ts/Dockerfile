# Dockerfile (multi-stage)

# Stage 1: Build
FROM node:20-alpine AS builder

WORKDIR /app

COPY api-backend-ts/package*.json ./
RUN npm ci --ignore-scripts

COPY api-backend-ts/tsconfig.json .
COPY api-backend-ts/src ./src

RUN npm run build

# Stage 2: Production
FROM node:20-alpine AS production

WORKDIR /app

# Non-root user for security (OWASP A05)
RUN addgroup -g 1001 -S aigis && adduser -u 1001 -S aigis -G aigis

COPY api-backend-ts/package*.json ./
RUN npm ci --only-production --ignore-scripts && npm cache clean --force

COPY --from=builder /app/dist ./dist
COPY api-backend-ts/migrations ./migrations

USER aigis

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
  CMD node -e "fetch('http://localhost:3000/health').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"

CMD ["node", "dist/server.js"]