---
name: x402-retry
description: Automatically retry 402 responses with x402curl for payment-gated APIs
trigger: reactive
---

# x402 Retry Skill

When an HTTP request returns 402 Payment Required with x402 payment info, automatically retry using x402curl.

## Detecting x402-Compatible 402 Responses

Not all 402 responses are x402-compatible. Only retry when you see:

- HTTP status: `402`
- Header: `X-Payment` present, OR
- Body contains `"x402Version"` in JSON

If a 402 lacks these markers, inform the user it requires payment but don't retry - it's not an x402 endpoint.

## Prerequisites Check

Before retrying, verify setup (once per session):

**Check 1: x402curl installed**
```bash
which x402curl
```

If missing, tell the user:
> x402curl is not installed. Install with: `cargo install x402curl`

**Check 2: Wallet configured**
```bash
grep -q "X402_PRIVATE_KEY" .env 2>/dev/null
```

If missing, tell the user:
> Add your private key to .env:
> `echo 'X402_PRIVATE_KEY=your_key' >> .env`

If either check fails, stop and guide setup. Do not attempt retry.

## Retry Workflow

**Step 1:** Inform the user (one line):
> This endpoint requires payment via x402. Retrying with x402curl...

**Step 2:** Transform curl → x402curl

Preserve these flags: `-X`, `-H`, `-d`, `--data`, `-o`, `-F`
Drop these flags: `-v`, `--verbose`, `-s`, `--silent`

```bash
# Original
curl -X POST https://api.example.com/endpoint -H "Content-Type: application/json" -d '{"key": "value"}'

# Transformed
x402curl -X POST https://api.example.com/endpoint -H "Content-Type: application/json" -d '{"key": "value"}'
```

**Step 3:** Execute and return result. On success, show the response without mentioning payment again.

## Error Handling

When x402curl fails, provide specific guidance:

**Insufficient funds:**
> Payment failed: insufficient funds.
> Your wallet needs USDC on Base network.
> Testnet: get funds from https://www.alchemy.com/faucets/base-sepolia
> Mainnet: transfer USDC to your wallet on Base.

**Invalid private key:**
> Payment failed: invalid private key in .env.
> Ensure X402_PRIVATE_KEY is 64 hex characters without '0x' prefix.

**Network failure:**
> Payment failed: could not reach payment network.
> Check your internet connection and try again.

**Payment rejected:**
> Payment submitted but rejected by server.
> The API may have changed requirements. Contact the provider.

## Examples

### Example 1: Simple POST

User: "Fetch data from https://paid-api.com/data"

```bash
curl https://paid-api.com/data
# Returns 402 with X-Payment header
```

You say: "This endpoint requires payment via x402. Retrying with x402curl..."

```bash
x402curl https://paid-api.com/data
# Returns 200 with data
```

Return the data to user.

### Example 2: POST with body

User: "Send this JSON to the translation API"

```bash
curl -X POST https://translate.example.com/v1 \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello", "target": "es"}'
# Returns 402 with x402Version in body
```

You say: "This endpoint requires payment via x402. Retrying with x402curl..."

```bash
x402curl -X POST https://translate.example.com/v1 \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello", "target": "es"}'
# Returns 200 with translation
```

Return: "Hola"

### Example 3: File upload

User: "Extract text from document.pdf using the OCR service"

```bash
curl -X POST https://ocr.example.com/extract -F "file=@document.pdf"
# Returns 402
```

You say: "This endpoint requires payment via x402. Retrying with x402curl..."

```bash
x402curl -X POST https://ocr.example.com/extract -F "file=@document.pdf"
```

Return the extracted text.
