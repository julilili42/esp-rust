from dataclasses import dataclass


@dataclass
class RawData:
    x: float
    y: float
    z: float


@dataclass
class Ema:
    x: float
    y: float
    z: float


@dataclass
class Rms:
    value: float


@dataclass
class AccData:
    raw: RawData
    ema: Ema
    rms: Rms
