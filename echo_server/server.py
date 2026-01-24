import os

from dotenv import load_dotenv
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

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
        # Use x402 SDK directly to create 402 response
        from x402 import (
            FacilitatorConfig,
            PaymentRequired,
            PaymentRequirements,
            ResourceConfig,
            X402_VERSION,
            x402ResourceServer,
        )
        from x402.http import HTTPFacilitatorClient

        # Create facilitator client
        facilitator_config = FacilitatorConfig(url=FACILITATOR_URL)
        facilitator = HTTPFacilitatorClient(facilitator_config)

        # Create server instance
        server = x402ResourceServer(facilitator)

        # Get wallet address from environment
        wallet_address = os.getenv(
            "WALLET_ADDRESS", "0x0000000000000000000000000000000000000000"
        )

        # Build payment requirements manually
        # For base-sepolia, USDC has 6 decimals, price $0.01 = 10000 units
        payment_requirements = [
            PaymentRequirements(
                scheme="exact",
                network=NETWORK,
                asset="0x036CbD53842c5426634e7929541eC2318f3dCF7e",  # USDC on base-sepolia
                amount="10000",  # $0.01 in USDC (6 decimals)
                pay_to=wallet_address,
                max_timeout_seconds=60,
                extra={},
            )
        ]

        # Create 402 response
        payment_required = PaymentRequired(
            x402_version=X402_VERSION,
            error="Payment required",
            resource=None,
            accepts=payment_requirements,
            extensions=None,
        )

        return JSONResponse(
            status_code=402,
            content=payment_required.model_dump(by_alias=True),
        )

    # For now, if payment header exists, echo back
    # Full verification will be added next
    return {"echo": body, "paid": True}
