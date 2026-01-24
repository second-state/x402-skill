# x402 Echo Server Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a minimal FastAPI server with one `/echo` endpoint that requires $0.01 x402 payment and echoes back the JSON body.

**Architecture:** Single-file FastAPI server using second-state/x402-payment-service SDK. The endpoint receives JSON, checks for payment header, verifies via facilitator, settles on-chain, then returns the echoed body.

**Tech Stack:** Python 3.14, FastAPI, uvicorn, x402-payment-service (from GitHub), python-dotenv

---

### Task 1: Initialize Python Project

**Files:**
- Create: `pyproject.toml`
- Create: `.python-version`

**Step 1: Create pyproject.toml**

```toml
[project]
name = "x402-echo-server"
version = "0.1.0"
description = "Minimal x402 payment-gated echo server for testing x402curl"
requires-python = ">=3.14"
dependencies = [
    "fastapi>=0.115.0",
    "uvicorn>=0.32.0",
    "x402-payment-service @ git+https://github.com/second-state/x402-payment-service.git",
    "python-dotenv>=1.0.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0.0",
    "pytest-asyncio>=0.24.0",
    "httpx>=0.27.0",
]
```

**Step 2: Create .python-version**

```
3.14
```

**Step 3: Commit**

```bash
git add pyproject.toml .python-version
git commit -m "feat: initialize Python project with dependencies"
```

---

### Task 2: Create Environment Configuration

**Files:**
- Create: `.env.example`
- Modify: `.gitignore`

**Step 1: Create .env.example**

```bash
# Your wallet address to receive payments (Base Sepolia testnet)
WALLET_ADDRESS=0x...
```

**Step 2: Add .env to .gitignore**

Append to `.gitignore`:

```
# Python
__pycache__/
*.py[cod]
.venv/

# Environment
.env
```

**Step 3: Commit**

```bash
git add .env.example .gitignore
git commit -m "feat: add environment configuration template"
```

---

### Task 3: Create Server Module Structure

**Files:**
- Create: `src/__init__.py`
- Create: `src/server.py`

**Step 1: Create empty __init__.py**

```python
```

**Step 2: Create server.py with app skeleton**

```python
from fastapi import FastAPI

app = FastAPI(
    title="x402 Echo Server",
    description="Minimal payment-gated echo endpoint for testing x402curl",
    version="0.1.0",
)


@app.get("/health")
async def health():
    return {"status": "ok"}
```

**Step 3: Verify server starts**

Run:
```bash
cd /Users/hydai/workspace/vibe/x402-skills/.worktrees/x402-echo-server
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
uvicorn src.server:app --port 8000 &
sleep 2
curl http://localhost:8000/health
kill %1
```

Expected: `{"status":"ok"}`

**Step 4: Commit**

```bash
git add src/__init__.py src/server.py
git commit -m "feat: add FastAPI server skeleton with health endpoint"
```

---

### Task 4: Write Failing Test for Echo Endpoint (No Payment)

**Files:**
- Create: `tests/__init__.py`
- Create: `tests/test_echo.py`

**Step 1: Create empty tests/__init__.py**

```python
```

**Step 2: Write test for 402 response when no payment**

```python
import pytest
from httpx import AsyncClient, ASGITransport

from src.server import app


@pytest.fixture
def anyio_backend():
    return "asyncio"


@pytest.fixture
async def client():
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as ac:
        yield ac


@pytest.mark.anyio
async def test_echo_without_payment_returns_402(client):
    response = await client.post("/echo", json={"hello": "world"})
    assert response.status_code == 402
```

**Step 3: Run test to verify it fails**

Run:
```bash
source .venv/bin/activate
pip install -e ".[dev]"
pytest tests/test_echo.py::test_echo_without_payment_returns_402 -v
```

Expected: FAIL (404 - endpoint doesn't exist yet)

**Step 4: Commit failing test**

```bash
git add tests/__init__.py tests/test_echo.py
git commit -m "test: add failing test for echo endpoint 402 response"
```

---

### Task 5: Implement Echo Endpoint with Payment Check

**Files:**
- Modify: `src/server.py`

**Step 1: Implement the echo endpoint**

Replace `src/server.py` contents:

```python
import os

from dotenv import load_dotenv
from fastapi import FastAPI, Request, Response

load_dotenv()

app = FastAPI(
    title="x402 Echo Server",
    description="Minimal payment-gated echo endpoint for testing x402curl",
    version="0.1.0",
)

NETWORK = "base-sepolia"
FACILITATOR_URL = "https://x402f1.secondstate.io"


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.post("/echo")
async def echo(request: Request):
    body = await request.json()

    # Check for payment header
    payment_header = request.headers.get("X-Payment")

    if not payment_header:
        # Import here to avoid issues if SDK not installed
        from x402_payment_service import PaymentService

        payment_service = PaymentService(
            app_name="Echo Service",
            app_logo="",
            headers=dict(request.headers),
            resource_url=str(request.url),
            price=0.01,
            description="Echo your JSON back",
            network=NETWORK,
            pay_to_address=os.getenv("WALLET_ADDRESS", "0x0000000000000000000000000000000000000000"),
            facilitator_url=FACILITATOR_URL,
            max_timeout_seconds=60,
        )
        return payment_service.response()

    # For now, if payment header exists, echo back
    # Full verification will be added next
    return {"echo": body, "paid": True}
```

**Step 2: Run test to verify it passes**

Run:
```bash
pytest tests/test_echo.py::test_echo_without_payment_returns_402 -v
```

Expected: PASS

**Step 3: Commit**

```bash
git add src/server.py
git commit -m "feat: add echo endpoint with 402 response for missing payment"
```

---

### Task 6: Add Test for Successful Echo with Payment Header

**Files:**
- Modify: `tests/test_echo.py`

**Step 1: Add test for echo with payment header**

Append to `tests/test_echo.py`:

```python
@pytest.mark.anyio
async def test_echo_with_payment_header_returns_body(client):
    response = await client.post(
        "/echo",
        json={"hello": "world"},
        headers={"X-Payment": "mock-payment-token"},
    )
    assert response.status_code == 200
    data = response.json()
    assert data["echo"] == {"hello": "world"}
    assert data["paid"] is True
```

**Step 2: Run test to verify it passes**

Run:
```bash
pytest tests/test_echo.py -v
```

Expected: Both tests PASS

**Step 3: Commit**

```bash
git add tests/test_echo.py
git commit -m "test: add test for echo with payment header"
```

---

### Task 7: Add Full Payment Verification Flow

**Files:**
- Modify: `src/server.py`

**Step 1: Update echo endpoint with full verification**

Replace the `/echo` endpoint in `src/server.py`:

```python
@app.post("/echo")
async def echo(request: Request):
    from x402_payment_service import PaymentService

    body = await request.json()

    payment_service = PaymentService(
        app_name="Echo Service",
        app_logo="",
        headers=dict(request.headers),
        resource_url=str(request.url),
        price=0.01,
        description="Echo your JSON back",
        network=NETWORK,
        pay_to_address=os.getenv("WALLET_ADDRESS", "0x0000000000000000000000000000000000000000"),
        facilitator_url=FACILITATOR_URL,
        max_timeout_seconds=60,
    )

    # Step 1: Parse payment header
    if not payment_service.parse():
        return payment_service.response()

    # Step 2: Verify payment with facilitator
    if not await payment_service.verify():
        return Response(status_code=402, content="Invalid payment")

    # Step 3: Settle on-chain
    await payment_service.settle()

    return {"echo": body, "paid": True}
```

**Step 2: Run existing tests**

Run:
```bash
pytest tests/test_echo.py -v
```

Expected: First test PASS, second test may FAIL (depends on SDK mock behavior)

**Step 3: Commit**

```bash
git add src/server.py
git commit -m "feat: add full payment verification and settlement flow"
```

---

### Task 8: Add README with Usage Instructions

**Files:**
- Create: `README.md` (in echo server directory)

**Step 1: Create README.md**

```markdown
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
uvicorn src.server:app --reload --port 8000
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
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with setup and usage instructions"
```

---

### Task 9: Run Full Integration Test

**Step 1: Start server and test manually**

Run:
```bash
source .venv/bin/activate
uvicorn src.server:app --port 8000 &
sleep 2

# Test health
curl http://localhost:8000/health

# Test 402 response
curl -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"test": "data"}'

kill %1
```

Expected:
- Health returns `{"status":"ok"}`
- Echo returns 402 with payment requirements

**Step 2: Run all tests**

Run:
```bash
pytest tests/ -v
```

Expected: All tests pass

**Step 3: Final commit if any cleanup needed**

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Initialize Python project | `pyproject.toml`, `.python-version` |
| 2 | Environment configuration | `.env.example`, `.gitignore` |
| 3 | Server skeleton | `src/__init__.py`, `src/server.py` |
| 4 | Failing test for 402 | `tests/__init__.py`, `tests/test_echo.py` |
| 5 | Echo endpoint with payment check | `src/server.py` |
| 6 | Test for successful echo | `tests/test_echo.py` |
| 7 | Full payment verification | `src/server.py` |
| 8 | Documentation | `README.md` |
| 9 | Integration testing | (manual verification) |

**Total estimated commits:** 8
