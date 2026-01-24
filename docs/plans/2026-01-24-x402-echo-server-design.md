# x402 Echo Server Design

## Overview

A minimal FastAPI server with one endpoint (`POST /echo`) that requires x402 payment and echoes back the JSON body. This serves as a demo for testing x402curl end-to-end.

## Requirements

- Python 3.14 with FastAPI
- Uses [second-state/x402-payment-service](https://github.com/second-state/x402-payment-service) SDK
- Local only (localhost for testing)
- Base Sepolia testnet (test USDC)
- Price: $0.01 per request

## Project Structure

```
x402-echo-server/
├── pyproject.toml      # Dependencies: fastapi, uvicorn, x402-payment-service
├── .env.example        # Template for wallet address config
├── .env                # Your receiving wallet address (gitignored)
├── src/
│   └── server.py       # The FastAPI app (~35 lines)
└── README.md           # How to run and test with x402curl
```

## Dependencies

- `fastapi>=0.115.0` - Web framework
- `uvicorn>=0.32.0` - ASGI server
- `x402-payment-service` - Payment protocol SDK (from GitHub)
- `python-dotenv>=1.0.0` - Environment variable loading

## Implementation

### server.py

```python
from fastapi import FastAPI, Request, Response
from x402_payment_service import PaymentService
import os

app = FastAPI()

NETWORK = "base-sepolia"
FACILITATOR_URL = "https://x402f1.secondstate.io"

@app.post("/echo")
async def echo(request: Request):
    body = await request.json()

    payment_service = PaymentService(
        app_name="Echo Service",
        app_logo="",
        headers=dict(request.headers),
        resource_url=str(request.url),
        price=0.01,
        description="Echo your JSON back",
        network=NETWORK,
        pay_to_address=os.getenv("WALLET_ADDRESS"),
        facilitator_url=FACILITATOR_URL,
        max_timeout_seconds=60,
    )

    # Step 1: Parse payment header
    if not payment_service.parse():
        return payment_service.response()  # Returns 402 with requirements

    # Step 2: Verify payment
    if not await payment_service.verify():
        return Response(status_code=402, content="Invalid payment")

    # Step 3: Settle on-chain
    await payment_service.settle()

    return {"echo": body, "paid": True}
```

### pyproject.toml

```toml
[project]
name = "x402-echo-server"
version = "0.1.0"
requires-python = ">=3.14"
dependencies = [
    "fastapi>=0.115.0",
    "uvicorn>=0.32.0",
    "x402-payment-service @ git+https://github.com/second-state/x402-payment-service.git",
    "python-dotenv>=1.0.0",
]

[project.scripts]
serve = "server:main"
```

### .env.example

```bash
# Your wallet address to receive payments (Base Sepolia)
WALLET_ADDRESS=0x...
```

## Running the Server

```bash
# Create venv and install
python3 -m venv .venv
source .venv/bin/activate
pip install -e .

# Configure wallet
cp .env.example .env
# Edit .env with your Base Sepolia wallet address

# Start server
uvicorn src.server:app --reload --port 8000
```

## Testing with x402curl

```bash
# 1. First request without payment - should get 402
curl -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'
# Returns: 402 Payment Required + payment requirements header

# 2. Using x402curl - handles payment automatically
x402curl -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'
# Returns: {"echo": {"hello": "world"}, "paid": true}

# 3. Dry-run to preview cost
x402curl --x402-dry-run -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'
# Shows: Payment required: $0.01 USDC on Base Sepolia

# 4. With confirmation prompt
x402curl --confirm -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'
# Prompts before paying
```

## Prerequisites for Testing

- x402curl configured with a private key that has Base Sepolia test USDC
- Echo server running on localhost:8000

## Out of Scope

- No Docker/deployment configs
- No multiple endpoints
- No rate limiting or auth
- No logging/metrics
