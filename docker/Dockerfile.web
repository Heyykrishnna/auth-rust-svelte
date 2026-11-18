FROM node:24-alpine AS builder

WORKDIR /app

COPY package.json package-lock.json ./
COPY packages/shared-types ./packages/shared-types
COPY vendor/picomatch ./vendor/picomatch
COPY apps/web/package.json ./apps/web/package.json

RUN npm ci --ignore-scripts

COPY apps/web ./apps/web
RUN npm run build

FROM node:24-alpine AS runtime

RUN apk upgrade --no-cache

WORKDIR /app

COPY package.json package-lock.json ./
COPY packages/shared-types ./packages/shared-types
COPY vendor/picomatch ./vendor/picomatch
COPY apps/web/package.json ./apps/web/package.json
RUN npm ci --omit=dev --ignore-scripts && \
    npm cache clean --force && \
    rm -rf /usr/local/lib/node_modules/npm /opt/yarn* /app/vendor && \
    rm -f /usr/local/bin/npm /usr/local/bin/npx /usr/local/bin/yarn /usr/local/bin/yarnpkg

COPY --from=builder /app/apps/web/build ./build

RUN addgroup -g 1001 -S appgroup && adduser -u 1001 -S appuser -G appgroup
USER appuser

EXPOSE 3000

ENV NODE_ENV=production
ENV PORT=3000
ENV HOST=0.0.0.0

CMD ["node", "build"]
