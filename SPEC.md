# x402 Skills Monetization Spec

## Overview

This project enables skill creators to monetize their Claude Code skills by gating API endpoints with x402 payments. It consists of two main components:

1. **x402curl** - A Rust CLI tool that handles x402 payment flows automatically
2. **Skill Integration Pattern** - Guidelines for skill authors to integrate x402 payments

## Goals

- Allow skill creators to monetize their skills on a per-call basis
- Eliminate the need for users to manually sign up for API keys
- Enable agents to pay for services automatically using a pre-configured wallet

---

## Component 1: x402curl

A Rust CLI wrapper for `curl` that transparently handles x402 payment responses.

### Functionality

1. **Make HTTP requests** - Works like standard `curl` for regular API calls
2. **Detect x402 responses** - Recognizes HTTP 402 Payment Required responses with x402 payment details
3. **Automatic payment** - Reads private key from `.env` file and submits payment via x402 SDK
4. **Request retry** - After successful payment, automatically re-submits the original request with payment proof

### Configuration

```bash
# .env file
X402_PRIVATE_KEY=<user's private key>
```

### Usage

```bash
# Basic usage (drop-in replacement for curl)
x402curl https://api.example.com/ocr -X POST -d @document.pdf

# The tool will:
# 1. Send the request
# 2. If 402 response received, parse x402 payment requirements
# 3. Sign and submit payment using private key from .env
# 4. Re-send original request with payment proof
# 5. Return the final response to stdout
```

### Implementation Notes

- Written in Rust for performance and easy distribution
- Should support common curl flags (-X, -H, -d, -o, etc.)
- Error handling for insufficient funds, network issues, invalid keys
- Optional verbose mode to show payment details

---

## Component 2: Skill Integration Pattern

### For Skill Authors (API Providers)

1. **Gate your API with x402** - Use the x402 SDK to add payment requirements to your API endpoints

   ```
   Your existing API endpoint:
   POST https://api.example.com/ocr

   After x402 integration:
   - Returns 402 Payment Required with x402 payment details
   - After payment verification, processes the request normally
   ```

2. **Update your SKILL.md** - Document the x402 requirement and x402curl usage

### SKILL.md Template

```markdown
# My Paid Skill

## Description
[What your skill does]

## Requirements

This skill requires `x402curl` for API access. Install it with:

```bash
cargo install x402curl
```

Configure your wallet by adding your private key to `.env`:

```bash
echo "X402_PRIVATE_KEY=your_private_key_here" >> .env
```

## Usage

When making API calls in this skill, always use `x402curl` instead of `curl` or `wget`:

```bash
# Correct
x402curl https://api.example.com/endpoint

# Incorrect
curl https://api.example.com/endpoint
```

## Pricing

- OCR endpoint: $0.01 per page
- Translation endpoint: $0.005 per 1000 characters
```

---

## User Flow

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Claude Code    │     │   x402curl   │     │  Paid API       │
│  (runs skill)   │     │              │     │  (x402 gated)   │
└────────┬────────┘     └──────┬───────┘     └────────┬────────┘
         │                     │                      │
         │ Execute skill       │                      │
         │ (uses x402curl)     │                      │
         │────────────────────>│                      │
         │                     │                      │
         │                     │  HTTP Request        │
         │                     │─────────────────────>│
         │                     │                      │
         │                     │  402 + Payment Info  │
         │                     │<─────────────────────│
         │                     │                      │
         │                     │  [Auto-pay with      │
         │                     │   private key]       │
         │                     │                      │
         │                     │  Request + Payment   │
         │                     │─────────────────────>│
         │                     │                      │
         │                     │  200 + Response      │
         │                     │<─────────────────────│
         │                     │                      │
         │  Response           │                      │
         │<────────────────────│                      │
         │                     │                      │
```

---

## Deliverables

1. **x402curl CLI tool** (Rust)
   - Source code repository
   - Binary releases for major platforms (Linux, macOS, Windows)
   - Installation via `cargo install`

2. **Documentation**
   - x402curl usage guide
   - Skill author integration guide
   - Example skills with x402 integration

3. **Example/Demo**
   - Sample paid API endpoint using x402 SDK
   - Sample skill that uses x402curl to access the paid API

---

## Future Considerations

- Support for multiple payment methods/tokens
- Budget limits and spending alerts
- Payment history/receipts logging
- Integration with wallet management tools
