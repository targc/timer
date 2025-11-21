# Timer/Callback Scheduling Platform

A high-performance timer platform built with Rust that executes HTTP callbacks at scheduled times. Features a hybrid storage architecture with PostgreSQL persistence and in-memory caching for sub-second execution precision.

## Features

- **Timer Registration**: RESTful API for creating, updating, and canceling timers
- **Scheduled Callbacks**: Automatic HTTP POST callbacks to external services at specified times
- **Hybrid Storage**: PostgreSQL for persistence + in-memory cache for performance (97% reduction in DB load)
- **Auto-Recovery**: Automatically executes overdue timers (up to 5 minutes past due)
- **Simple Authentication**: API key-based authentication
- **Docker Support**: Full containerization with docker-compose

## Quick Start

### Prerequisites

- Rust 1.75 or later
- PostgreSQL 15+
- Docker and Docker Compose (for containerized deployment)

### Local Development

1. **Clone the repository**
```bash
cd timer
```

2. **Set up environment variables**
```bash
cp .env.example .env
# Edit .env with your configuration
```

3. **Start PostgreSQL** (using Docker)
```bash
docker run -d \
  --name timer-postgres \
  -e POSTGRES_USER=timer \
  -e POSTGRES_PASSWORD=timer123 \
  -e POSTGRES_DB=timerdb \
  -p 5432:5432 \
  postgres:15-alpine
```

4. **Run the application**
```bash
cargo run
```

The service will be available at `http://localhost:8080`

### Docker Deployment

```bash
# Start all services (PostgreSQL + Timer Platform)
docker-compose up -d

# View logs
docker-compose logs -f timer

# Stop services
docker-compose down
```

## API Documentation

### Authentication

All API endpoints (except `/healthz`) require the `X-API-Key` header:

```bash
X-API-Key: your-api-key-here
```

### Endpoints

#### Create Timer
```bash
POST /timers
Content-Type: application/json
X-API-Key: your-api-key

{
  "execute_at": "2025-10-28T16:00:00Z",
  "callback_url": "https://api.example.com/webhook",
  "callback_headers": {
    "Authorization": "Bearer token123"
  },
  "callback_payload": {
    "event": "timer_triggered",
    "user_id": "user123"
  },
  "metadata": {
    "client_ref": "order-456"
  }
}
```

Response (201 Created):
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "created_at": "2025-10-28T10:30:00Z",
    "execute_at": "2025-10-28T16:00:00Z",
    "callback_url": "https://api.example.com/webhook",
    "status": "pending",
    "executed_at": null
  }
}
```

#### Get Timer
```bash
GET /timers/{id}
X-API-Key: your-api-key
```

#### List Timers
```bash
GET /timers?status=pending&limit=50&offset=0&sort=created_at&order=desc
X-API-Key: your-api-key
```

#### Update Timer
```bash
PUT /timers/{id}
Content-Type: application/json
X-API-Key: your-api-key

{
  "execute_at": "2025-10-28T17:00:00Z"
}
```

#### Cancel Timer
```bash
DELETE /timers/{id}
X-API-Key: your-api-key
```

#### Health Check
```bash
GET /healthz
```

## Configuration

Environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `API_KEY` | Yes | - | API authentication key (min 32 chars) |
| `PG_HOST` | Yes* | - | PostgreSQL server hostname |
| `PG_PORT` | No | 5432 | PostgreSQL server port |
| `PG_USER` | Yes* | - | PostgreSQL username |
| `PG_PASSWORD` | Yes* | - | PostgreSQL password |
| `PG_DB_NAME` | Yes* | - | PostgreSQL database name |
| `DATABASE_URL` | Yes* | - | Direct PostgreSQL URL (alternative to component config) |
| `PORT` | No | 8080 | HTTP server port |
| `RUST_LOG` | No | info | Logging level (trace, debug, info, warn, error) |
| `NATS_HOST` | No | - | NATS server hostname (enables NATS callbacks) |
| `NATS_PORT` | No | 4222 | NATS server port |
| `NATS_USER` | No | - | NATS username for authentication |
| `NATS_PASSWORD` | No | - | NATS password for authentication |
| `NATS_URL` | No | - | Direct NATS URL (alternative to component config) |

*Either `DATABASE_URL` OR the `PG_*` variables are required (not both)

## Architecture

### Hybrid Storage System

The platform uses a two-tier storage architecture:

1. **PostgreSQL (Persistent Layer)**: Stores all timers permanently
2. **In-Memory Cache (Hot Layer)**: Stores near-term timers (execute within 5 minutes)

This design reduces database queries by 97% while maintaining sub-second execution precision.

### Scheduler Tasks

Two independent background tasks run concurrently:

- **Memory Loader** (30s interval): Loads timers from PostgreSQL into cache
- **Execution Task** (1s interval): Scans cache and executes due timers

### Callback Execution

- HTTP POST requests to external services
- 30-second timeout
- Single execution attempt (no retries)
- 2xx = success, 4xx/5xx/timeout = failure

## Response Format

All API responses follow this structure:

```json
{
  "code": 0,
  "message": "success",
  "data": { ... }
}
```

Response codes:
- `0`: Success
- `1`: Internal error (database, network)
- `2`: Validation error
- `3`: Not found
- `4`: Unauthorized

HTTP status codes:
- `200`: OK
- `201`: Created
- `400`: Bad Request
- `401`: Unauthorized
- `404`: Not Found
- `500`: Internal Server Error

## Development

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Run Tests

```bash
cargo test
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Check without building
cargo check
```

### Database Operations

```bash
# Create database
sqlx database create

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Monitoring

### Health Check

```bash
curl http://localhost:8080/healthz
```

Successful response:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "status": "up",
    "database": "connected",
    "timestamp": "2025-10-28T10:30:00Z"
  }
}
```

### Logs

The application uses structured logging with the following levels:

- `info`: Startup events, timer execution, scheduler activity
- `warn`: Callback failures, database connection issues
- `error`: Critical errors requiring attention

## Limitations (MVP)

- One-shot timers only (no recurring timers)
- Single shared API key (no per-client authentication)
- No callback response storage
- No retry logic for failed callbacks
- No rate limiting
- Single scheduler instance (no distributed locking)
- All times must be in UTC

## Troubleshooting

### Timer not executing

1. Check timer status: `GET /timers/{id}`
2. Verify `execute_at` is in the past
3. Check scheduler logs: `docker-compose logs -f timer`
4. Ensure timer is in cache (eventual consistency: up to 30s delay)

### Database connection errors

```bash
# Check PostgreSQL is running
docker-compose ps postgres

# Test connection
docker-compose exec postgres psql -U timer -d timerdb -c "SELECT 1"
```

### Callback failures

Check timer's `last_error` field for error details:
```bash
curl -H "X-API-Key: your-api-key" http://localhost:8080/timers/{id}
```

## Production Considerations

- Use strong API keys (32+ characters, cryptographically random)
- Enable TLS/HTTPS for callback URLs
- Monitor database connection pool utilization
- Set up log aggregation and alerting
- Consider horizontal scaling with distributed locks (Redis)
- Implement per-client API keys and rate limiting

## License

MIT

## Support

For issues and questions, please open an issue on GitHub.
