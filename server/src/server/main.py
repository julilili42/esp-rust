import json
import queue
import threading
from dataclasses import dataclass

from server.api import data_q, start_server
from server.plot import plot


@dataclass
class AccData:
    x: float
    y: float
    z: float


def parse_data(s: str) -> list[AccData]:
    batch = []
    data = json.loads(s)
    for entry in data:
        batch.append(AccData(**entry))

    return batch


def main():
    server_thread = threading.Thread(target=start_server, daemon=True)
    server_thread.start()
    print("Server started")

    try:
        batch = []
        while plot(batch):
            try:
                s = data_q.get(timeout=0.05)
                batch = parse_data(s)
            except queue.Empty:
                batch = []

    except KeyboardInterrupt:
        print("Stopped server.")
    return 0


if __name__ == "__main__":
    main()
