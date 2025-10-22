from rivetkit_client import SimpleClient as ActorClient
from common import start_mock_server, logger


def do_oneoff_increment(client):
    handle = client.get("counter")
    logger.info("Created new handle: " + str(handle))

    logger.info("Sending increment action")
    res = handle.action("increment", 1)
    # First increment on mock server, so it should be 1
    assert res == 1

def test_e2e_simple_sync():
    (addr, stop_mock_server) = start_mock_server()

    client = ActorClient(addr)
    handle = client.get("counter")
    logger.info("Actor handle: " + str(handle))

    logger.info("Subscribing to newCount")
    handle.subscribe("newCount")

    do_oneoff_increment(client)

    logger.info("Waiting for newCount event")
    logger.info(handle.receive(1))

    handle.disconnect()

    stop_mock_server()
