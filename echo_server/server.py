from fastapi import FastAPI

app = FastAPI(
    title="x402 Echo Server",
    description="Minimal payment-gated echo endpoint for testing x402curl",
    version="0.1.0",
)


@app.get("/health")
async def health():
    return {"status": "ok"}
