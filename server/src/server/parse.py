import json

from server.types import AccData, Ema, RawData, Rms


def parse_data(s: str) -> list[AccData]:
    batch = []
    data = json.loads(s)
    for entry in data:
        batch.append(
            AccData(
                raw=RawData(**entry["raw"]),
                ema=Ema(**entry["ema"]),
                rms=Rms(**entry["rms"]),
            )
        )

    return batch
