# Nuxt Minimal Starter

Look at the [Nuxt documentation](https://nuxt.com/docs/getting-started/introduction) to learn more.

## Setup

Make sure to install dependencies:

```bash
# npm
npm install

# pnpm
pnpm install

# yarn
yarn install

# bun
bun install
```

## Development Server

Start the development server on `http://localhost:3000`:

```bash
# npm
npm run dev

# pnpm
pnpm dev

# yarn
yarn dev

# bun
bun run dev
```

## Production

Build the application for production:

```bash
# npm
npm run build

# pnpm
pnpm build

# yarn
yarn build

# bun
bun run build
```

Locally preview production build:

```bash
# npm
npm run preview

# pnpm
pnpm preview

# yarn
yarn preview

# bun
bun run preview
```

Check out the [deployment documentation](https://nuxt.com/docs/getting-started/deployment) for more information.

## ScyllaDB with Docker Compose

Create root env file from the template and set credentials:

```bash
cp ../.env.example ../.env
```

Connection env variables used by local setup:

- `SCYLLA_HOST` (default: `127.0.0.1`)
- `SCYLLA_PORT` (default: `9042`)
- `SCYLLA_USERNAME` (default: `scylla_app`)
- `SCYLLA_PASSWORD` (default: `change_me_local`)

Start ScyllaDB in the background:

```bash
docker compose -f ../docker-compose.yml up -d
```

Verify the container is healthy:

```bash
docker compose -f ../docker-compose.yml ps
```

Open a CQL shell in the running ScyllaDB container:

```bash
docker exec -it localbase-scylladb cqlsh -u "$SCYLLA_USERNAME" -p "$SCYLLA_PASSWORD"
```

Stop ScyllaDB:

```bash
docker compose -f ../docker-compose.yml down
```

Stop and remove the persisted data volume:

```bash
docker compose -f ../docker-compose.yml down -v
```
