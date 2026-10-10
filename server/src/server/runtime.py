import queue
import threading
from datetime import datetime
from pathlib import Path

from server.api import app, data_q, start_server
from server.parse import parse_data

RECORDING_DIR = Path(__file__).resolve().parents[2] / "measurements"


def run(plotter, save=False):
    app.state.recording_path = None
    if save:
        RECORDING_DIR.mkdir(parents=True, exist_ok=True)
        recording_path = RECORDING_DIR / f"measurements_{datetime.now():%Y-%m-%d_%H-%M-%S_%f}.jsonl"
        recording_path.touch(exist_ok=False)
        app.state.recording_path = recording_path
        print(f"Recording: {recording_path}")

    server_thread = threading.Thread(target=start_server, daemon=True)
    server_thread.start()
    print("Server started")

    try:
        batch = []
        while plotter(batch):
            batch = []
            try:
                batch.extend(parse_data(data_q.get(timeout=0.05)))
                while True:
                    batch.extend(parse_data(data_q.get_nowait()))
            except queue.Empty:
                pass

    except KeyboardInterrupt:
        print("Stopped server.")
    return 0
