from collections import deque
from math import nan

import matplotlib.pyplot as plt

from server.movement import MovementDetector
from server.types import AccData

acceleration_history = deque(maxlen=500)
acceleration_figure = None


def plot_acceleration(batch: list[AccData]):
    global acceleration_figure
    if acceleration_figure is None:
        plt.ion()
        acceleration_figure, ax = plt.subplots()
        for axis in ("x", "y", "z"):
            ax.plot([], [], label=f"raw {axis}")
        for axis in ("x", "y", "z"):
            ax.plot([], [], label=f"ema {axis}")
        ax.plot([], [], label="rms")

        legend = ax.legend(loc="upper right")
        targets = {}
        for line, handle, label in zip(ax.lines, legend.get_lines(), legend.get_texts()):
            handle.set_picker(5)
            label.set_picker(True)
            targets[handle] = targets[label] = (line, handle, label)

        def toggle_line(event):
            target = targets.get(event.artist)
            if target is None:
                return
            line, handle, label = target
            visible = not line.get_visible()
            line.set_visible(visible)
            handle.set_alpha(1.0 if visible else 0.2)
            label.set_alpha(1.0 if visible else 0.2)
            ax.relim(visible_only=True)
            ax.autoscale_view()
            acceleration_figure.canvas.draw_idle()

        acceleration_figure.canvas.mpl_connect("pick_event", toggle_line)
        ax.grid(True)

    if not plt.fignum_exists(acceleration_figure.number):
        return False

    if batch:
        acceleration_history.extend(batch)
        ax = acceleration_figure.axes[0]

        y_values = [
            [getattr(getattr(entry, kind), axis) for entry in acceleration_history]
            for kind in ("raw", "ema")
            for axis in ("x", "y", "z")
        ]

        y_values.append([entry.rms.value for entry in acceleration_history])

        for line, y_value in zip(ax.lines, y_values):
            line.set_data(range(len(acceleration_history)), y_value)

        ax.relim(visible_only=True)
        ax.autoscale_view()
        acceleration_figure.canvas.draw_idle()
    plt.pause(0.001)
    return True


detector = MovementDetector()
movement_history = deque(maxlen=500)
samples_seen = 0
detection_count = 0
movement_figure = None


def plot_movement(batch: list[AccData]):
    global movement_figure, samples_seen, detection_count
    if movement_figure is None:
        plt.ion()
        movement_figure, axes = plt.subplots(4, 1, sharex=True, figsize=(11, 10))
        for ax, label in zip(axes[:2], ("Raw acceleration [g]", "Raw − baseline [g]")):
            for axis in ("x", "y", "z"):
                ax.plot([], [], label=axis)
            ax.set_ylabel(label)
        axes[2].step([], [], where="post", label="Window energy")
        axes[2].step([], [], where="post", linestyle="--", label="Threshold")
        axes[2].plot([], [], "ro", label="Motion detected", markersize=5)
        axes[2].set_ylabel("Mean energy [g²]")
        axes[3].step([], [], where="post", color="red")
        axes[3].set_ylabel("Motion detected")
        axes[3].set_yticks([0, 1], labels=["No", "Yes"])
        axes[3].set_ylim(-0.1, 1.1)
        axes[3].set_xlabel("Received sample")
        axes[3].text(
            0.02, 0.9, "Detections: 0", transform=axes[3].transAxes,
            va="top", bbox={"facecolor": "white", "edgecolor": "none", "alpha": 0.8},
        )
        for ax in axes[:3]:
            ax.legend(loc="upper right")
        for ax in axes:
            ax.grid(True)
        movement_figure.tight_layout()

    if not plt.fignum_exists(movement_figure.number):
        return False

    for entry in batch:
        detected = detector.update(entry.raw)
        samples_seen += 1
        detection_count += detected
        movement_history.append(
            (
                samples_seen,
                entry.raw,
                detector.delta,
                detector.window_energy if detector.window_energy is not None else nan,
                detector.threshold if detector.threshold is not None else nan,
                detected,
            )
        )

    if batch:
        indices = [row[0] for row in movement_history]
        for column, ax in enumerate(movement_figure.axes[:2], start=1):
            for axis, line in zip(("x", "y", "z"), ax.lines):
                line.set_data(indices, [getattr(row[column], axis) for row in movement_history])
        energy_ax = movement_figure.axes[2]
        for column, line in zip((3, 4), energy_ax.lines[:2]):
            line.set_data(indices, [row[column] for row in movement_history])
        detections = [row for row in movement_history if row[5]]
        energy_ax.lines[2].set_data(
            [row[0] for row in detections], [row[3] for row in detections]
        )
        motion_ax = movement_figure.axes[3]
        motion_ax.lines[0].set_data(indices, [row[5] for row in movement_history])
        motion_ax.texts[0].set_text(f"Detections: {detection_count}")
        for ax in movement_figure.axes:
            ax.relim()
            ax.autoscale_view()
        movement_figure.canvas.draw_idle()

    plt.pause(0.001)
    return True
