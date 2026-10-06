import queue

import uvicorn
from fastapi import FastAPI, WebSocket
from fastapi.websockets import WebSocketDisconnect

app = FastAPI()
data_q = queue.Queue()


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    try:
        with open("measurements.jsonl", "a", encoding="utf-8", buffering=1) as recording:
            while True:
                data = await websocket.receive_text()
                recording.write(data + "\n")
                data_q.put(data)
    except WebSocketDisconnect as exc:
        print(f"Client disconnected: {exc.code}", flush=True)


def start_server():
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="warning")
