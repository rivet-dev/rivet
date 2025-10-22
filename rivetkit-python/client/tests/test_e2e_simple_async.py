import asyncio
import pytest
from rivetkit_client import AsyncSimpleClient as ActorClient
from common import start_mock_server, logger

async def do_oneoff_increment(client):
    handle = await client.get("counter")
    logger.info("Created new handle: " + str(handle))

    logger.info("Sending increment action")
    res = await handle.action("increment", 1)
    # First increment on mock server, so it should be 1
    assert res == 1

@pytest.mark.asyncio
async def test_e2e_simple_async():
    (addr, stop_mock_server) = start_mock_server()

    client = ActorClient(addr)

    handle = await client.get("counter")
    logger.info("Actor handle: " + str(handle))

    logger.info("Subscribing to newCount")
    await handle.subscribe("newCount")

    await do_oneoff_increment(client)

    logger.info("Receiving msgs")
    logger.info(await handle.receive(1))

    await handle.disconnect()

    stop_mock_server()
