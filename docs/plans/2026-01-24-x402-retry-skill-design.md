# x402-retry Skill Design

## Overview

A reactive skill that teaches Claude to automatically retry failed HTTP requests with x402curl when a 402 Payment Required response is detected from an x402-compatible endpoint.

## Goals

- Enable seamless interaction with x402-gated APIs
- Inform users when payment is required, then handle it automatically
- Provide clear guidance when setup is incomplete or errors occur

## Non-Goals

- Initial x402curl installation (skill only checks and guides)
- Wallet funding management
- Spending history or budget tracking
- Auto-activation for non-x402 402 responses

## Design Decisions

### Trigger: Reactive on 402 Detection

The skill activates when Claude observes a 402 response with x402 markers (X-Payment header or x402Version in body). This is not user-invoked.

**Rationale:** Users shouldn't need to know which APIs are x402-gated. Claude discovers this naturally and adapts.

### Behavior: Inform Then Retry

When 402 is detected, Claude tells the user "This endpoint requires payment via x402. Retrying with x402curl..." then proceeds automatically.

**Rationale:** Balance between transparency and automation. User knows payment happened but isn't blocked by confirmation dialogs.

### Prerequisites: Check First, Guide If Missing

Before retrying, verify x402curl is installed and .env has X402_PRIVATE_KEY. If either is missing, provide setup instructions instead of failing cryptically.

**Rationale:** Better UX than letting x402curl fail with unclear errors.

### Error Handling: Actionable Guidance

Each error type (insufficient funds, invalid key, network failure, rejection) has specific guidance.

**Rationale:** Users can fix issues without researching error codes.

## Detection Criteria

An x402-compatible 402 response has:
- HTTP status 402
- X-Payment header present, OR
- Body contains "x402Version" in JSON

Regular 402s without these markers are not retried.

## Command Transformation

curl flags preserved: `-X`, `-H`, `-d`, `--data`, `-o`, `-F`
curl flags dropped: `-v`, `--verbose`, `-s`, `--silent`

## File Location

```
claude/skills/x402/SKILL.md
openclaw/skills/x402/SKILL.md
```
