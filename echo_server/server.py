import os

import httpx
from dotenv import load_dotenv
from fastapi import FastAPI, Request
from x402.fastapi.middleware import require_payment

load_dotenv()

# Workaround: x402 Python SDK's FacilitatorClient doesn't set httpx timeout,
# causing settle() to timeout on Base network (block confirmation: 10-28s).
# See: https://github.com/coinbase/x402/issues/1062
_original_httpx_init = httpx.AsyncClient.__init__


def _patched_httpx_init(self: httpx.AsyncClient, *args: object, **kwargs: object) -> None:
    if "timeout" not in kwargs:
        kwargs["timeout"] = 60.0  # 60 seconds for blockchain transactions
    _original_httpx_init(self, *args, **kwargs)


httpx.AsyncClient.__init__ = _patched_httpx_init  # type: ignore[method-assign]

app = FastAPI(
    title="x402 Echo Server",
    description="Minimal payment-gated echo endpoint for testing x402curl",
    version="0.2.0",
)

# Get wallet address from environment
WALLET_ADDRESS = os.getenv(
    "WALLET_ADDRESS", "0x0000000000000000000000000000000000000000"
)

# Network and facilitator configuration
NETWORK = "base-sepolia"
FACILITATOR_URL = "https://x402.org/facilitator"

# Create payment middleware for /echo endpoint
# Price: $0.01 USD (will be converted to USDC automatically)
payment_middleware = require_payment(
    price="$0.01",
    pay_to_address=WALLET_ADDRESS,
    path="/echo",
    description="Echo endpoint - returns your JSON payload",
    mime_type="application/json",
    max_deadline_seconds=60,
    network=NETWORK,
    facilitator_config={"url": FACILITATOR_URL},
)

# Register the payment middleware
app.middleware("http")(payment_middleware)


@app.get("/health")
async def health():
    """Health check endpoint (no payment required)."""
    return {"status": "ok"}


@app.post("/echo")
async def echo(request: Request):
    """
    Echo endpoint - returns the JSON body sent to it.

    This endpoint requires x402 payment. The payment is handled
    automatically by the require_payment middleware.
    """
    body = await request.json()

    # Access payment details from request state (set by middleware)
    payment_details = getattr(request.state, "payment_details", None)
    verify_response = getattr(request.state, "verify_response", None)

    response = {
        "echo": body,
        "paid": True,
    }

    # Include payer info if available
    if verify_response and verify_response.payer:
        response["payer"] = verify_response.payer

    return response
