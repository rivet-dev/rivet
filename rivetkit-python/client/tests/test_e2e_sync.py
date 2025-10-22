from rivetkit_client import Client as ActorClient
from common import start_mock_server, logger

def test_e2e_sync():
    (addr, stop_mock_server) = start_mock_server()

    client = ActorClient(addr)

    handle = client.get("counter")
    logger.info("Actor handle: " + str(handle))

    logger.info("Listening to newCount")
    handle.on_event("newCount", lambda msg: print("Received msg:", msg))

    logger.info("Sending action")
    assert 1 == handle.action("increment", 1)

    handle.disconnect()
