from collections import deque

import matplotlib.pyplot as plt

history = deque(maxlen=500)
figure = None


def plot(batch):
    global figure
    if figure is None:
        plt.ion()
        figure, ax = plt.subplots()
        for axis in ("x", "y", "z"):
            ax.plot([], [], label=axis)
        ax.legend()
        ax.grid(True)

    if not plt.fignum_exists(figure.number):
        return False

    if batch:
        history.extend(batch)
        ax = figure.axes[0]
        for axis, line in zip(("x", "y", "z"), ax.lines):
            line.set_data(
                range(len(history)), [getattr(entry, axis) for entry in history]
            )
        ax.relim()
        ax.autoscale_view()
        figure.canvas.draw_idle()
    plt.pause(0.001)
    return True
