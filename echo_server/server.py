import os

from dotenv import load_dotenv
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

from x402 import (
    FacilitatorConfig,
    PaymentRequired,
    PaymentRequirements,
    X402_VERSION,
    x402ResourceServer,
)
from x402.http import HTTPFacilitatorClient, decode_payment_signature_header
from x402.mechanisms.evm.exact import ExactEvmServerScheme

load_dotenv()

app = FastAPI(
    title="x402 Echo Server",
    description="Minimal payment-gated echo endpoint for testing x402curl",
    version="0.1.0",
)

NETWORK = "base-sepolia"
FACILITATOR_URL = "https://x402f1.secondstate.io"

# Get wallet address from environment
WALLET_ADDRESS = os.getenv(
    "WALLET_ADDRESS", "0x0000000000000000000000000000000000000000"
)

# USDC contract address on base-sepolia (6 decimals)
USDC_ADDRESS = "0x036CbD53842c5426634e7929541eC2318f3dCF7e"

# Price: $0.01 in USDC (6 decimals) = 10000 units
PRICE_AMOUNT = "10000"

# Create facilitator client
facilitator_config = FacilitatorConfig(url=FACILITATOR_URL)
facilitator = HTTPFacilitatorClient(facilitator_config)

# Create server instance and register EVM scheme
server = x402ResourceServer(facilitator)
server.register(NETWORK, ExactEvmServerScheme())


def build_payment_requirements() -> list[PaymentRequirements]:
    """Build payment requirements for the echo endpoint."""
    return [
        PaymentRequirements(
            scheme="exact",
            network=NETWORK,
            asset=USDC_ADDRESS,
            amount=PRICE_AMOUNT,
            pay_to=WALLET_ADDRESS,
            max_timeout_seconds=60,
            extra={},
        )
    ]


def create_402_response() -> JSONResponse:
    """Create a 402 Payment Required response."""
    payment_requirements = build_payment_requirements()
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


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.post("/echo")
async def echo(request: Request):
    body = await request.json()

    # Check for payment header
    payment_header = request.headers.get("X-Payment")

    if not payment_header:
        return create_402_response()

    # Parse the payment payload from the header
    try:
        payment_payload = decode_payment_signature_header(payment_header)
    except Exception as e:
        return JSONResponse(
            status_code=400,
            content={"error": f"Invalid payment header: {e}"},
        )

    # Get payment requirements
    payment_requirements = build_payment_requirements()

    # Verify the payment with the facilitator
    try:
        verify_response = await server.verify_payment(
            payment_payload, payment_requirements[0]
        )
    except Exception as e:
        return JSONResponse(
            status_code=500,
            content={"error": f"Payment verification failed: {e}"},
        )

    if not verify_response.is_valid:
        # Payment is invalid - return 402
        return JSONResponse(
            status_code=402,
            content={
                "error": "Payment verification failed",
                "reason": verify_response.invalid_reason,
            },
        )

    # Settle the payment on-chain
    try:
        settle_response = await server.settle_payment(
            payment_payload, payment_requirements[0]
        )
    except Exception as e:
        return JSONResponse(
            status_code=500,
            content={"error": f"Payment settlement failed: {e}"},
        )

    if not settle_response.success:
        return JSONResponse(
            status_code=500,
            content={
                "error": "Payment settlement failed",
                "reason": settle_response.error_reason,
            },
        )

    # Payment verified and settled - return echo response
    return {
        "echo": body,
        "paid": True,
        "transaction": settle_response.transaction,
        "network": settle_response.network,
    }
