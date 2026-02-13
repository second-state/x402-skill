# x402 Setup Guide

Follow these steps to install the x402curl binary and configure a wallet so the x402 skill can handle payment-gated API requests.

## Step 1: Install x402curl

Choose **one** of the following options.

### Option A: Download pre-built binary (recommended)

Detect the platform and download the matching release:

```bash
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}_${ARCH}" in
  Linux_x86_64)   TARGET="x86_64-unknown-linux-musl" ;;
  Linux_aarch64)  TARGET="aarch64-unknown-linux-musl" ;;
  Darwin_x86_64)  TARGET="x86_64-apple-darwin" ;;
  Darwin_arm64)   TARGET="aarch64-apple-darwin" ;;
  *) echo "Unsupported platform: ${OS} ${ARCH}" && exit 1 ;;
esac

TAG=$(gh api repos/second-state/x402-skill/releases/latest --jq '.tag_name')
URL="https://github.com/second-state/x402-skill/releases/download/${TAG}/x402curl-${TARGET}.tar.gz"

mkdir -p ~/.claude/skills/x402/scripts
curl -fsSL "$URL" | tar xz -C ~/.claude/skills/x402/scripts
chmod +x ~/.claude/skills/x402/scripts/x402curl
```

Verify the installation:

```bash
~/.claude/skills/x402/scripts/x402curl --version
```

| Platform       | Architecture | Target triple                  |
|----------------|-------------|--------------------------------|
| Linux          | x86_64      | `x86_64-unknown-linux-musl`    |
| Linux          | aarch64     | `aarch64-unknown-linux-musl`   |
| macOS          | x86_64      | `x86_64-apple-darwin`          |
| macOS          | arm64       | `aarch64-apple-darwin`         |

### Option B: Build from source

Requires a Rust toolchain (`cargo`).

```bash
cargo build --release
mkdir -p ~/.claude/skills/x402/scripts
cp target/release/x402curl ~/.claude/skills/x402/scripts/x402curl
```

Verify:

```bash
~/.claude/skills/x402/scripts/x402curl --version
```

## Step 2: Set up a wallet

x402curl needs a private key to sign payments. Choose **one** of the following options.

### Option A: Generate with openssl (quickest)

```bash
KEY=$(openssl rand -hex 32)
echo "X402_PRIVATE_KEY=${KEY}" >> .env
```

### Option B: Generate with cast (Foundry)

```bash
cast wallet new
```

Copy the private key from the output (without the `0x` prefix) and add to `.env`:

```bash
echo "X402_PRIVATE_KEY=<paste-key-here>" >> .env
```

### Option C: Use a keystore (Keystore v3 JSON)

If you have an existing Ethereum keystore file:

```bash
echo "X402_WALLET=/path/to/keystore.json" >> .env
echo "X402_WALLET_PASSWORD=your-password" >> .env
```

### Important notes on key format

- The private key must be **64 hex characters** with **no `0x` prefix**.
- x402curl resolves wallet credentials in this priority order:
  1. `--x402-key` CLI flag
  2. `X402_PRIVATE_KEY` environment variable
  3. `X402_PRIVATE_KEY` from `.env` file
  4. `--x402-wallet` + `--x402-wallet-password` CLI flags
  5. `X402_WALLET` + `X402_WALLET_PASSWORD` env vars (including `.env`)
  6. `~/.x402/config` (TOML file)

## Step 3: Fund the wallet

The wallet needs USDC on Base network to make payments.

**Testnet (Base Sepolia):**
Get free testnet tokens from the [Alchemy Base Sepolia faucet](https://www.alchemy.com/faucets/base-sepolia).

**Mainnet (Base):**
Transfer USDC to your wallet address on Base network. To find your address:

```bash
~/.claude/skills/x402/scripts/x402curl --x402-dry-run https://any-x402-endpoint.example.com
```

The `--x402-dry-run` flag shows payment requirements (including your wallet address) without actually sending a transaction.

## Troubleshooting

**"No wallet credentials found"**
Ensure `.env` exists in the project root and contains `X402_PRIVATE_KEY=...` (64 hex chars, no `0x` prefix).

**"Invalid private key"**
The key must be exactly 64 hexadecimal characters. Remove any `0x` prefix or surrounding quotes.

**"Wallet keystore file not found"**
Check that `X402_WALLET` in `.env` points to an existing file path and `X402_WALLET_PASSWORD` is set.

**Binary not found or permission denied**
Ensure `~/.claude/skills/x402/scripts/x402curl` exists and is executable (`chmod +x`).

**Platform not supported**
Pre-built binaries are available for Linux (x86_64, aarch64) and macOS (x86_64, arm64). For other platforms, build from source (Option B in Step 1).
