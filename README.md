# x402curl

A curl-like CLI tool with automatic x402 payment handling.

## Installation

```bash
cargo install x402curl
```

## Configuration

Set your private key using one of these methods (in priority order):

1. **CLI flag**: `--x402-key 0x...`
2. **Environment variable**: `export X402_PRIVATE_KEY=0x...`
3. **Local .env file**: Add `X402_PRIVATE_KEY=0x...` to `./.env`
4. **Global config**: Create `~/.x402/config`:
   ```toml
   private_key = "0x..."
   ```

## Usage

```bash
# Basic GET request
x402curl https://api.example.com/resource

# POST with JSON body
x402curl -X POST -H "Content-Type: application/json" -d '{"key": "value"}' https://api.example.com

# Upload file
x402curl -F "file=@document.pdf" https://api.example.com/upload

# Preview payment requirements without paying
x402curl --x402-dry-run https://api.example.com/paid-endpoint

# Require confirmation before payment
x402curl --confirm https://api.example.com/paid-endpoint

# Verbose mode (shows payment details)
x402curl -v https://api.example.com/paid-endpoint
```

## Supported Flags

| Flag | Description |
|------|-------------|
| `-X, --request` | HTTP method |
| `-H, --header` | Add header |
| `-d, --data` | Request body |
| `-o, --output` | Output file |
| `-f, --fail` | Fail on HTTP errors |
| `-s, --silent` | Silent mode |
| `-v, --verbose` | Verbose output |
| `-F, --form` | Form field |
| `-u, --user` | Basic auth |
| `-L, --location` | Follow redirects |
| `--x402-dry-run` | Preview payment |
| `--confirm` | Confirm before paying |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Network error |
| 3 | Payment error |
| 4 | HTTP error |
| 5 | Config error |

## License

MIT
