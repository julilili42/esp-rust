import queue
from contextlib import nullcontext

import uvicorn
from fastapi import FastAPI, WebSocket
from fastapi.websockets import WebSocketDisconnect

app = FastAPI()
app.state.recording_path = None
data_q = queue.Queue()


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    try:
        path = app.state.recording_path
        with (
            path.open("a", encoding="utf-8", buffering=1)
            if path is not None else nullcontext()
        ) as recording:
            while True:
                data = await websocket.receive_text()
                if recording is not None:
                    recording.write(data + "\n")
                data_q.put(data)
    except WebSocketDisconnect as exc:
        print(f"Client disconnected: {exc.code}", flush=True)


def start_server():
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="warning")
