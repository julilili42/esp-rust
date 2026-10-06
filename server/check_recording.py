import asyncio
import os
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
from unittest.mock import AsyncMock

from fastapi.websockets import WebSocketDisconnect

from server.api import data_q, websocket_endpoint


async def check():
    recording = Path("measurements.jsonl")
    messages = ['[{"raw":{"x":1,"y":2,"z":3}}]', '[{"rms":{"value":0.5}}]']
    for index, message in enumerate(messages):

        async def receive():
            yield message
            assert (
                recording.read_text(encoding="utf-8").splitlines()
                == messages[: index + 1]
            )
            raise WebSocketDisconnect(code=1000)

        socket = SimpleNamespace(
            accept=AsyncMock(),
            receive_text=receive().__anext__,
        )
        await websocket_endpoint(socket)
        assert data_q.get_nowait() == message
    assert data_q.empty()


if __name__ == "__main__":
    original_directory = Path.cwd()
    with TemporaryDirectory() as directory:
        try:
            os.chdir(directory)
            asyncio.run(check())
        finally:
            os.chdir(original_directory)
    print("Recording, append after reconnect and immediate writes passed")
