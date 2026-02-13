# x402 SKILL Setup Guide

Follow these steps to install the x402curl binary and configure a wallet so the x402 skill can handle payment-gated API requests.

> **Install location:** Commands below use the default `~/.openclaw/skills`.
> Alternatives: `~/.claude/skills`, `~/.opencode/skills`, `~/.agent/skills` — replace the prefix if using a different agent tool.

## Step 1: Install x402curl

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

mkdir -p ~/.openclaw/skills/x402/scripts
curl -fsSL "$URL" | tar xz -C ~/.openclaw/skills/x402/scripts
chmod +x ~/.openclaw/skills/x402/scripts/x402curl
```

Verify the installation:

```bash
~/.openclaw/skills/x402/scripts/x402curl --version
```

| Platform       | Architecture | Target triple                  |
|----------------|-------------|--------------------------------|
| Linux          | x86_64      | `x86_64-unknown-linux-musl`    |
| Linux          | aarch64     | `aarch64-unknown-linux-musl`   |
| macOS          | x86_64      | `x86_64-apple-darwin`          |
| macOS          | arm64       | `aarch64-apple-darwin`         |


## Step 2: Set up a wallet

x402curl needs a wallet or private key to sign payments. Choose **one** of the following options.

### Option A: Use an exisiting keystore wallet (Recommended)

Ask the user to give you his existing `wallet.json` file and its password. Save the `wallet.json` file to `~/.openclaw/skills/x402/`

```bash
echo "X402_WALLET=~/.openclaw/skills/x402/wallet.json" >> .env
echo "X402_WALLET_PASSWORD=your-password" >> .env
```

### Option B: Create a new wallet

With the user permission, you can create a brand new wallet for yourself.

```python
pip install eth-account

python3 -c "
from eth_account import Account
import json
acct = Account.create()
ks = Account.encrypt(acct.key, 'your-password')
print('Address:', acct.address)
with open('wallet.json', 'w') as f: json.dump(ks, f, indent=2)
print('Saved to wallet.json')
"
```

Move the generated `wallet.json` to the `~/.openclaw/skills/x402/` directory.

```bash
mv wallet.json ~/.openclaw/skills/x402/
```

Tell the x402curl how to find and open the wallet.

```bash
echo "X402_WALLET=~/.openclaw/skills/x402/wallet.json" >> .env
echo "X402_WALLET_PASSWORD=your-password" >> .env
```


### Option C: You know the private key for the wallet

The private key must be **64 hex characters** with **no `0x` prefix**. Let's say that the private key is `${KEY}`

```bash
echo "X402_PRIVATE_KEY=${KEY}" >> .env
```

### Important notes

- x402curl resolves wallet credentials in this priority order:
  1. `--x402-key` CLI flag
  2. `X402_PRIVATE_KEY` environment variable (including `.env`)
  4. `--x402-wallet` + `--x402-wallet-password` CLI flags
  5. `X402_WALLET` + `X402_WALLET_PASSWORD` env vars (including `.env`)
  6. `~/.x402/config` (TOML file)

## Step 3: Fund the wallet

The wallet needs USDC on Base network to make payments. Transfer USDC to your wallet address on Base network. To find your address:

```bash
~/.openclaw/skills/x402/scripts/x402curl --x402-dry-run https://any-x402-endpoint.example.com
```

The `--x402-dry-run` flag shows payment requirements (including your wallet address) without actually sending a transaction.

Tell the human: Your payment wallet address is: `<address>`. In order for you to pay other agents or humans, please fund this address with USDC on Base network.

## Troubleshooting

**"No wallet credentials found"**
Ensure `.env` exists in the project root and contains `X402_PRIVATE_KEY=...` or `X402_WALLET`

**"Invalid private key"**
The key must be exactly 64 hexadecimal characters. Remove any `0x` prefix or surrounding quotes.

**"Wallet keystore file not found"**
Check that `X402_WALLET` in `.env` points to an existing file path and `X402_WALLET_PASSWORD` is set.

**Binary not found or permission denied**
Ensure `~/.openclaw/skills/x402/scripts/x402curl` exists and is executable (`chmod +x`).

**Platform not supported**
Pre-built binaries are available for Linux (x86_64, aarch64) and macOS (x86_64, arm64). For other platforms, build from source as follows.

```
git clone https://github.com/second-state/x402-skill
cargo build --release
mv target/release/x402curl ~/.openclaw/skills/x402/scripts/
```
