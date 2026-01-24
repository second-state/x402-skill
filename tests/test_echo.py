import pytest
from httpx import AsyncClient, ASGITransport

from echo_server.server import app


@pytest.fixture
def anyio_backend():
    return "asyncio"


@pytest.fixture
async def client():
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as ac:
        yield ac


@pytest.mark.anyio
async def test_echo_without_payment_returns_402(client):
    response = await client.post("/echo", json={"hello": "world"})
    assert response.status_code == 402


@pytest.mark.anyio
async def test_echo_with_invalid_payment_header_returns_400(client):
    """Test that an invalid (non-base64) payment header returns 400."""
    response = await client.post(
        "/echo",
        json={"hello": "world"},
        headers={"X-Payment": "mock-payment-token"},
    )
    assert response.status_code == 400
    data = response.json()
    assert "error" in data
    assert "Invalid payment header" in data["error"]
