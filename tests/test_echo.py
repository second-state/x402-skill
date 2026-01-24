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
