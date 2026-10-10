from math import sqrt

from server.types import Ema, RawData


class MovementDetector:
    def __init__(self):
        self.baseline: Ema | None = None
        self.samples = 0
        self.energy = 0.0
        self.threshold: float | None = None
        self.active_windows = 0
        self.delta = Ema(0.0, 0.0, 0.0)
        self.window_energy: float | None = None

    def update(self, input: RawData | Ema) -> bool:
        if self.baseline is None:
            self.baseline = Ema(input.x, input.y, input.z)
        else:
            self.baseline = Ema(
                input.x * 0.05 + self.baseline.x * 0.95,
                input.y * 0.05 + self.baseline.y * 0.95,
                input.z * 0.05 + self.baseline.z * 0.95,
            )
        self.delta = Ema(
            input.x - self.baseline.x,
            input.y - self.baseline.y,
            input.z - self.baseline.z,
        )
        delta = sqrt(self.delta.x**2 + self.delta.y**2 + self.delta.z**2)
        self.energy += delta * delta
        self.samples += 1

        window = 100 if self.threshold is None else 10
        if self.samples < window:
            return False
        self.window_energy = self.energy / self.samples
        self.energy = 0.0
        self.samples = 0

        if self.threshold is None:
            self.threshold = max(1.51814 * self.window_energy, 9.375_026e-5)
            return False
        self.active_windows = (
            min(self.active_windows + 1, 255)
            if self.window_energy > self.threshold
            else 0
        )
        return self.active_windows >= 3
