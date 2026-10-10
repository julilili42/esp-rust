import argparse

from server.plot import plot_acceleration, plot_movement
from server.runtime import run


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--plot", choices=("standard", "movement"), default="standard",
        help="Live plot to display (default: standard)",
    )
    parser.add_argument(
        "--save", action="store_true",
        help="Save timestamped JSONL recordings in server/measurements/",
    )
    args = parser.parse_args()
    plotter = plot_movement if args.plot == "movement" else plot_acceleration
    return run(plotter, save=args.save)


if __name__ == "__main__":
    raise SystemExit(main())
