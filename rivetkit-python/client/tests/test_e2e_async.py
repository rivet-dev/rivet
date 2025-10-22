import asyncio
import pytest
from rivetkit_client import AsyncClient as ActorClient
from common import start_mock_server, get_free_port, logger

@pytest.mark.asyncio
async def test_e2e_async():
    (addr, stop_mock_server) = start_mock_server()

    client = ActorClient(addr)

    handle = await client.get("counter", tags={"tag": "valu3"})
    logger.info("Actor handle: " + str(handle))

    logger.info("Subscribed to newCount")
    handle.on_event("newCount", lambda msg: logger.info("Received msg:", msg))

    logger.info("Sending action")
    assert 1 == await handle.action("increment", 1)

    await handle.disconnect()

    stop_mock_server()
