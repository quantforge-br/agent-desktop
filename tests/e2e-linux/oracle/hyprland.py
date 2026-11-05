from __future__ import annotations

import json
import os
import subprocess
import time
from dataclasses import dataclass

from oracle import atspi_raw as raw
from runner.session import Tier

HYPRCTL_TIMEOUT_S = 5.0


class HyprlandError(RuntimeError):
    pass


@dataclass(frozen=True)
class WindowView:
    address: str
    pid: int
    app_class: str
    title: str
    at: tuple[int, int]
    size: tuple[int, int]
    workspace: str
    floating: bool
    fullscreen: int
    hidden: bool
    mapped: bool

    def geometry(self) -> tuple[int, int, int, int]:
        return (*self.at, *self.size)


def _environment(tier: Tier) -> dict[str, str]:
    return {
        "PATH": tier.env.get("PATH", os.defpath),
        "HOME": tier.env.get("HOME", "/"),
        "XDG_RUNTIME_DIR": str(tier.runtime_dir),
        "HYPRLAND_INSTANCE_SIGNATURE": tier.env["HYPRLAND_INSTANCE_SIGNATURE"],
    }


def hyprctl(tier: Tier, *args: str) -> str:
    """Runs the nested compositor's own hyprctl; the owner's instance is never addressed."""
    process = subprocess.run(["hyprctl", *args], capture_output=True, text=True, timeout=HYPRCTL_TIMEOUT_S,
                             env=_environment(tier), check=False)
    if process.returncode != 0:
        raise HyprlandError(f"hyprctl {' '.join(args)} exited {process.returncode}: {process.stderr.strip()}")
    return process.stdout


def query(tier: Tier, name: str):
    text = hyprctl(tier, "-j", name)
    try:
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise HyprlandError(f"hyprctl -j {name} is not JSON: {text[:200]!r}") from error


def dispatch(tier: Tier, call: str) -> None:
    reply = hyprctl(tier, "dispatch", call).strip()
    if reply != "ok":
        raise HyprlandError(f"dispatch {call} answered {reply!r}")


def _view(client: dict) -> WindowView:
    return WindowView(
        address=client["address"], pid=client["pid"], app_class=client["class"], title=client["title"],
        at=tuple(client["at"]), size=tuple(client["size"]), workspace=client["workspace"]["name"],
        floating=client["floating"], fullscreen=client["fullscreen"], hidden=client["hidden"],
        mapped=client["mapped"],
    )


def windows(tier: Tier) -> list[WindowView]:
    return sorted((_view(client) for client in query(tier, "clients")), key=lambda view: view.address)


def windows_of(tier: Tier, pid: int) -> list[WindowView]:
    return [view for view in windows(tier) if view.pid == pid and view.mapped]


def window_of(tier: Tier, pid: int) -> WindowView:
    found = windows_of(tier, pid)
    if len(found) != 1:
        raise HyprlandError(f"expected one mapped window for pid {pid}, found {len(found)}")
    return found[0]


def view_at(tier: Tier, address: str) -> WindowView:
    for view in windows(tier):
        if view.address == address:
            return view
    raise HyprlandError(f"no window at {address}")


def active_address(tier: Tier) -> str | None:
    text = hyprctl(tier, "-j", "activewindow").strip()
    if not text.startswith("{"):
        return None
    return json.loads(text).get("address") or None


def cursor(tier: Tier) -> tuple[int, int]:
    position = query(tier, "cursorpos")
    return position["x"], position["y"]


def monitors(tier: Tier) -> list[dict]:
    return sorted(query(tier, "monitors"), key=lambda monitor: monitor["id"])


def _target(address: str) -> str:
    return f'window = "address:{address}"'


def focus(tier: Tier, address: str) -> None:
    dispatch(tier, f"hl.dsp.focus({{ {_target(address)} }})")


def layout_box(tier: Tier) -> tuple[int, int, int, int]:
    """The bounding box of every monitor in layout pixels, which absolute motion spans."""
    boxes = [(m["x"], m["y"], round(m["width"] / m["scale"]), round(m["height"] / m["scale"]))
             for m in monitors(tier)]
    left, top = min(b[0] for b in boxes), min(b[1] for b in boxes)
    right, bottom = max(b[0] + b[2] for b in boxes), max(b[1] + b[3] for b in boxes)
    return left, top, right - left, bottom - top


def move_cursor(tier: Tier, x: int, y: int) -> None:
    """Moves the session's seat pointer to (x, y) with real motion, as a hand on the mouse would.

    A compositor warp moves the cursor without a motion event, so the surface under the old
    position never hears that the pointer left it.
    """
    left, top, width, height = layout_box(tier)
    descriptor = os.open(tier.runtime_dir / "seat-pointer.fifo", os.O_WRONLY | os.O_NONBLOCK)
    try:
        os.write(descriptor, f"move {x - left} {y - top} {width} {height}\n".encode())
    finally:
        os.close(descriptor)
    deadline = time.monotonic() + 3
    while cursor(tier) != (x, y):
        if time.monotonic() >= deadline:
            raise AssertionError(f"the seat pointer did not reach {(x, y)}; it is at {cursor(tier)}")
        time.sleep(0.02)


def _float(tier: Tier, address: str, on: bool) -> None:
    action = "set" if on else "unset"
    dispatch(tier, f'hl.dsp.window.float({{ action = "{action}", {_target(address)} }})')


def _fullscreen_off(tier: Tier, address: str) -> None:
    for mode in ("maximized", "fullscreen"):
        dispatch(tier, f'hl.dsp.window.fullscreen({{ mode = "{mode}", action = "unset", {_target(address)} }})')


def restore(tier: Tier, before: WindowView) -> WindowView:
    """Puts one window back as it was read, through the compositor and not through the product."""
    address = before.address
    current = view_at(tier, address)
    if current.fullscreen:
        _fullscreen_off(tier, address)
    if current.workspace != before.workspace:
        dispatch(tier, f'hl.dsp.window.move({{ workspace = "{before.workspace}", follow = false, '
                       f'{_target(address)} }})')
    if before.floating:
        if not view_at(tier, address).floating:
            _float(tier, address, True)
        x, y, width, height = before.geometry()
        dispatch(tier, f"hl.dsp.window.resize({{ x = {width}, y = {height}, {_target(address)} }})")
        dispatch(tier, f"hl.dsp.window.move({{ x = {x}, y = {y}, {_target(address)} }})")
    elif view_at(tier, address).floating:
        _float(tier, address, False)
    return settle(tier, address, before)


def settle(tier: Tier, address: str, expected: WindowView, seconds: float = 4.0) -> WindowView:
    found = raw.poll(lambda: view_at(tier, address) if view_at(tier, address) == expected else None, seconds)
    return found or view_at(tier, address)


def wait_view(tier: Tier, address: str, predicate, seconds: float = 6.0) -> WindowView | None:
    return raw.poll(lambda: view_at(tier, address) if predicate(view_at(tier, address)) else None, seconds)


def healthy(tier: Tier) -> bool:
    return bool(monitors(tier)) and hyprctl(tier, "configerrors").strip() == ""
