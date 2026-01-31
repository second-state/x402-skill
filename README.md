# x402-skills

Monetize Claude Code skills with x402 micropayments. This project provides the tooling for skill creators to gate API endpoints behind automatic payments, and for users to pay for them seamlessly.

## Components

| Component | Language | Description |
|-----------|----------|-------------|
| **x402curl** | Rust | Drop-in `curl` replacement with automatic x402 payment handling |
| **Echo Server** | Python | Demo FastAPI server with a payment-gated endpoint |
| **x402-retry** | Skill | Claude Code skill that detects 402 responses and retries with x402curl |

## Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Claude Code    │     │   x402curl   │     │  Paid API       │
│  (runs skill)   │     │              │     │  (x402 gated)   │
└────────┬────────┘     └──────┬───────┘     └────────┬────────┘
         │                     │                      │
         │ Execute skill       │                      │
         │ (uses x402curl)     │                      │
         │────────────────────>│                      │
         │                     │  HTTP Request        │
         │                     │─────────────────────>│
         │                     │  402 + Payment Info  │
         │                     │<─────────────────────│
         │                     │  [Auto-pay with      │
         │                     │   private key]       │
         │                     │  Request + Payment   │
         │                     │─────────────────────>│
         │                     │  200 + Response      │
         │                     │<─────────────────────│
         │  Response           │                      │
         │<────────────────────│                      │
```

## Getting Started

### Prerequisites

- Rust toolchain (for x402curl)
- Python 3.14+ (for echo server)
- A Base Sepolia wallet with testnet USDC

### Install x402curl

```bash
cargo install --path .
```

### Configure wallet

```bash
# For paying (client side) - add private key to .env
echo 'X402_PRIVATE_KEY=your_64_hex_char_key' >> .env
```

The private key is resolved in this order:
1. `--x402-key` CLI flag
2. `X402_PRIVATE_KEY` environment variable
3. `./.env` file in current directory
4. `~/.x402/config` global TOML config file

## x402curl Usage

x402curl is a drop-in replacement for `curl` that automatically detects 402 responses, signs a payment, and retries the request.

```bash
# Basic request - payment handled automatically
x402curl -X POST https://api.example.com/endpoint \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}'

# Preview payment requirements without paying
x402curl --x402-dry-run -X POST https://api.example.com/endpoint

# Prompt for confirmation before paying
x402curl --confirm -X POST https://api.example.com/endpoint

# File upload via multipart form
x402curl -X POST https://api.example.com/upload -F "file=@document.pdf"

# Verbose mode - shows signing address, payment flow, headers
x402curl -v -X POST https://api.example.com/endpoint
```

### Supported curl flags

| Flag | Description |
|------|-------------|
| `-X` | HTTP method |
| `-H` | Request header (repeatable) |
| `-d` | Request body (`@filename` to read from file) |
| `--data-binary` | Raw binary data |
| `-F` | Multipart form field (repeatable) |
| `-o` | Write output to file |
| `-u` | Basic auth (`user:password`) |
| `-L` | Follow redirects |
| `-f` | Fail silently on HTTP errors |
| `-s` | Silent mode |
| `-v` | Verbose mode |

### x402-specific flags

| Flag | Description |
|------|-------------|
| `--x402-key` | Override private key for this request |
| `--x402-dry-run` | Show payment requirements without paying |
| `--confirm` | Prompt before making payment |

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid args, config) |
| 2 | Network error (connection failed) |
| 3 | Payment error (insufficient funds) |
| 4 | HTTP error (4xx/5xx with `-f`) |
| 5 | Configuration error (no key found) |

## Echo Server

A minimal FastAPI server for testing x402curl. The `/echo` endpoint requires $0.01 USDC on Base Sepolia.

### Setup

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -e .

cp .env.example .env
# Edit .env with your Base Sepolia wallet address
```

### Run

```bash
uvicorn echo_server.server:app --reload --port 8000
```

### Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/health` | GET | None | Health check |
| `/echo` | POST | x402 ($0.01 USDC) | Echoes JSON body after payment verification and settlement |

### Test

```bash
# Returns 402 - no payment
curl -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'

# Pays and returns echoed body
x402curl -X POST http://localhost:8000/echo \
  -H "Content-Type: application/json" \
  -d '{"hello": "world"}'
```

## x402-retry Skill

A reactive Claude Code skill that automatically detects x402-compatible 402 responses and retries with x402curl. It triggers when a response has:

- HTTP status `402` **and** either:
  - `X-Payment` header present, or
  - Response body contains `"x402Version"`

The skill checks that `x402curl` is installed and `X402_PRIVATE_KEY` is configured, then transparently retries the failed request. See `skills/x402-retry/SKILL.md` for full details.

## Running Tests

```bash
# Rust tests
cargo test

# Python tests
source .venv/bin/activate
pip install -e ".[dev]"
pytest
```

## License

MIT
