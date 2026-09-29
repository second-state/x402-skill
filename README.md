# AI agent skills for making payments

This project provides the tooling and agentic skill for AI agents to seamlessly pay for x402-gated services. If you are looking for the agentic skill to request and receive payments, checkout https://money.flows.network

## Quick start

Copy and paste the following sentence to give your agent the ability to make payments, and purchase products or services on its own.

For OpenClaw:

> Read https://raw.githubusercontent.com/second-state/x402-skill/refs/heads/master/openclaw/skills/x402/install.md and follow the instructions to set up the skill to make payments to x402-gated web or agent services.

For Claude Code:

> Read https://raw.githubusercontent.com/second-state/x402-skill/refs/heads/master/claude/skills/x402/install.md and follow the instructions to set up the skill to make payments to x402-gated web or agent services.

## Components

| Component | Language | Description |
|-----------|----------|-------------|
| **x402curl** | Rust | Drop-in `curl` replacement with automatic x402 payment handling |
| **Echo Server** | Python | Demo FastAPI server with an x402 payment-gated endpoint |
| **x402-retry** | Skill | AI agent skill that detects HTTP 402 responses and retries with x402curl |

## Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  AI agent       │     │   x402curl   │     │  Paid API       │
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

x402curl supports two wallet formats: raw private key or Keystore v3 (JSON wallet file). If both are configured, the private key takes priority.

**Option A: Private key**

```bash
echo 'X402_PRIVATE_KEY=your_64_hex_char_key' >> .env
```

**Option B: Keystore v3 wallet**

```bash
echo 'X402_WALLET=/path/to/wallet.json' >> .env
echo 'X402_WALLET_PASSWORD=your_password' >> .env
```

Credentials are resolved in this order:
1. `--x402-key` CLI flag
2. `X402_PRIVATE_KEY` environment variable / `.env` file
3. `--x402-wallet` + `--x402-wallet-password` CLI flags
4. `X402_WALLET` + `X402_WALLET_PASSWORD` environment variables / `.env` file
5. `~/.x402/config` global TOML config file

## x402curl Usage

x402curl is a drop-in replacement for `curl` that automatically detects 402 responses, signs a payment, and retries the request.

```bash
# Basic request - payment handled automatically
x402curl -X POST https://api.example.com/endpoint \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}'

# Check wallet USDC balance (default: Base mainnet)
x402curl --x402-balance

# Check balance on Base Sepolia testnet
x402curl --x402-balance --x402-rpc-url https://sepolia.base.org

# Check any ERC-20 token balance (decimals auto-detected)
x402curl --x402-balance \
  --x402-rpc-url https://eth.llamarpc.com \
  --x402-token 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48

# Preview payment requirements without paying
x402curl --x402-dry-run -X POST https://api.example.com/endpoint

# Prompt for confirmation before paying
x402curl --confirm -X POST https://api.example.com/endpoint

# Refuse any single payment above 10000 atomic token units
x402curl --x402-max-amount 10000 https://api.example.com/endpoint

# File upload via multipart form
x402curl -X POST https://api.example.com/upload -F "file=@document.pdf"

# Use a Keystore v3 wallet file instead of a raw private key
x402curl --x402-wallet wallet.json --x402-wallet-password mypassword \
  -X POST https://api.example.com/endpoint

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
| `--x402-wallet` | Path to Keystore v3 wallet (JSON) file |
| `--x402-wallet-password` | Password for the keystore wallet file |
| `--x402-dry-run` | Show payment requirements without paying |
| `--x402-max-amount` | Maximum payment amount in atomic token units for this request |
| `--x402-balance` | Query wallet USDC balance |
| `--x402-rpc-url` | Override RPC endpoint URL (default: Base mainnet) |
| `--x402-token` | Override ERC-20 token contract address (decimals and symbol auto-detected) |
| `--confirm` | Prompt before making payment |
| `--x402-before-sign` | Program run after the amount pin and `--confirm`, before any signature |

`--x402-max-amount` is a per-request raw threshold applied to each advertised
payment option's integer amount. It is not a human-readable or fiat value, a
cumulative budget, or a constraint on the asset, network, payee, or resource.
For example, `10000` means `0.01 USDC` only for a six-decimal USDC asset. A
challenge with no supported payment option at or below the threshold is
rejected before signing and before any paid retry; this currently exits through
the middleware error path with code 2.

`--x402-before-sign PROGRAM` is off unless you set it. The program runs after
that amount pin and after `--confirm`, and before any signature. JSON terms for
the chosen accept are written to its stdin (`payTo`, `amount`, `asset`,
`network`, `scheme`, `resource`, `url`). `resource` is the 402 resource when
the challenge carries one. Exit 0 to pay. Any other exit refuses with code 2
and nothing is signed. After exit 0, the signed accept must still match that
`payTo`, `amount`, `asset`, and `network`. Another network with the same payee
and amount is refused with code 2 and nothing is signed. `resource` is not part
of that bind. Leave the flag unset and the loop is unchanged.

The check is a `twzrd.payment_decision.v1` for this challenge. Issuance of that
record is $0.001. Verifying it does not call `GET /v1/intel/trust`. Continue
only on an unexpired `allow` or `warn`.

```js
import { verifyPaymentDecisionRecord } from 'twzrd-x402-gate/payment-decision' // @0.9.9

const terms = JSON.parse(await new Response(process.stdin).text())
// `record` is the $0.001 payment_decision for this challenge, already in hand.
const verified = await verifyPaymentDecisionRecord(record, {
  publicKeyPem: process.env.TWZRD_DECISION_PUBKEY,
  challenge: {
    payTo: terms.payTo,
    amount: terms.amount,
    resource: terms.resource,
    network: terms.network,
    scheme: terms.scheme,
  },
})
if (!verified.ok || (verified.decision !== 'allow' && verified.decision !== 'warn')) {
  process.exit(1)
}
```

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid args, config) |
| 2 | Network or x402 middleware error (including no acceptable payment option) |
| 3 | Payment error (insufficient funds) |
| 4 | HTTP error (4xx/5xx with `-f`) |
| 5 | Configuration error (no key found) |
| 6 | RPC error (balance query failed) |

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

The skill checks that `x402curl` is installed and `X402_PRIVATE_KEY` is configured, then transparently retries the failed request. See `claude/skills/x402/SKILL.md` (for Claude Code) or `openclaw/skills/x402/SKILL.md` (for OpenClaw) for full details.

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
