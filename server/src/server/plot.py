from collections import deque

import matplotlib.pyplot as plt

from server.types import AccData

history = deque(maxlen=500)
figure = None


def plot(batch: list[AccData]):
    global figure
    if figure is None:
        plt.ion()
        figure, ax = plt.subplots()
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
            figure.canvas.draw_idle()

        figure.canvas.mpl_connect("pick_event", toggle_line)
        ax.grid(True)

    if not plt.fignum_exists(figure.number):
        return False

    if batch:
        history.extend(batch)
        ax = figure.axes[0]

        y_values = [
            [getattr(getattr(entry, kind), axis) for entry in history]
            for kind in ("raw", "ema")
            for axis in ("x", "y", "z")
        ]

        y_values.append([entry.rms.value for entry in history])

        for line, y_value in zip(ax.lines, y_values):
            line.set_data(range(len(history)), y_value)

        ax.relim(visible_only=True)
        ax.autoscale_view()
        figure.canvas.draw_idle()
    plt.pause(0.001)
    return True
