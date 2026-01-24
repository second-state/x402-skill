# x402 Echo Server

A minimal FastAPI server with x402 payment-gated echo endpoint for testing x402curl.

## Setup

```bash
# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -e .

# Configure wallet address
cp .env.example .env
# Edit .env with your Base Sepolia wallet address
```

## Run

```bash
uvicorn echo_server.server:app --reload --port 8000
```

## Test with x402curl

```bash
# Without payment - returns 402
curl -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'

# With x402curl - handles payment automatically
x402curl -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'

# Dry-run to preview cost
x402curl --x402-dry-run -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'
```

## Configuration

| Variable | Description |
|----------|-------------|
| `WALLET_ADDRESS` | Your Base Sepolia wallet to receive payments |

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/echo` | POST | Payment-gated echo (requires $0.01 USDC) |
