from __future__ import annotations

import contextlib
import os
import signal
import struct
from dataclasses import asdict

from oracle import atspi_raw as raw, hyprland
from oracle.proc import members
from runner import apps
from runner.helper import changed, command, display, managed, nodes, one, ref, require, root, snapshot
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_support import qt_session, raw_names, window

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
CALCULATOR_NAMES = ("7", "Undo")
QT_NAMES = ("Press me", "Text entry", "Delayed activation", "File")
CODE_NAMES = ("File", "Edit", "Selection", "View", "Go", "Run")
FIREFOX_NAMES = ("File", "Edit", "History", "Bookmarks", "Search or enter address")


@contextlib.contextmanager
def launched(ctx, name, settle_s):
    """A slow-starting app that is stopped again and whose helper processes are waited out."""
    baseline = members(ctx.tier.runtime_dir)
    app = apps.launch(ctx.tier, name, settle_s=settle_s)
    try:
        yield app
    finally:
        apps.stop(app)
        reap_helpers(ctx, baseline)
        changed(lambda: members(ctx.tier.runtime_dir), baseline, seconds=20)


def reap_helpers(ctx, baseline):
    """Helper processes an app left in the private session after its main process ended are stopped."""
    for signum in (signal.SIGTERM, signal.SIGKILL):
        extra = [item for item in members(ctx.tier.runtime_dir) if item not in baseline]
        for pid, _, _ in extra:
            try:
                os.kill(pid, signum)
            except ProcessLookupError:
                pass
        raw.poll(lambda: not [item for item in members(ctx.tier.runtime_dir) if item not in baseline], 3.0)


def windows_state(ctx):
    return [asdict(view) for view in hyprland.windows(ctx.tier)]


def products_by_pid(ctx):
    data = command(ctx, ["list-windows"]).data
    return {item["pid"]: item for item in data}


@implementation("hyprland.windows_match_compositor")
def windows_match_compositor(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as calculator, qt_session(ctx) as qt:
        before = windows_state(ctx)
        ctx.before("hyprland.windows", before)
        listed = products_by_pid(ctx)
        active = hyprland.active_address(ctx.tier)
        for app in (calculator, qt):
            view = window(ctx, app)
            item = listed.get(app.pid)
            require(item is not None, f"list-windows omits pid {app.pid}: {sorted(listed)}")
            require(item["title"] == view.title, f"title {item['title']!r} != compositor {view.title!r}")
            require(item["visible"] is (view.mapped and not view.hidden), "visibility differs from the compositor")
            require(item["is_focused"] is (view.address == active), "focus differs from the compositor")
            require(item["minimized"] is False, "an unminimised window is reported minimised")
        require(sum(1 for item in listed.values() if item["is_focused"]) == 1, "not exactly one focused window")
        ctx.after("hyprland.windows", windows_state(ctx))
        ctx.restored("hyprland.windows", windows_state(ctx))


@implementation("hyprland.window_bounds_in_layout_coordinates")
def window_bounds_in_layout_coordinates(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as calculator, qt_session(ctx) as qt:
        ctx.before("hyprland.windows", windows_state(ctx))
        listed = products_by_pid(ctx)
        for app in (calculator, qt):
            view = window(ctx, app)
            bounds = listed[app.pid].get("bounds")
            require(bounds is not None, f"list-windows gives no bounds for pid {app.pid}")
            got = (bounds["x"], bounds["y"], bounds["width"], bounds["height"])
            require(got == tuple(float(value) for value in view.geometry()),
                    f"bounds {got} differ from the compositor geometry {view.geometry()}")
        ctx.after("hyprland.windows", windows_state(ctx))
        ctx.restored("hyprland.windows", windows_state(ctx))


def monitor_state(ctx):
    return [(monitor["name"], monitor["x"], monitor["y"], monitor["width"], monitor["height"], monitor["scale"])
            for monitor in hyprland.monitors(ctx.tier)]


@implementation("hyprland.displays_match_monitors")
def displays_match_monitors(ctx: LegContext) -> None:
    before = monitor_state(ctx)
    ctx.before("hyprland.monitors", before)
    data = command(ctx, ["list-displays"]).data
    expected = {name: (float(x), float(y), float(width), float(height), float(scale))
                for name, x, y, width, height, scale in before}
    got = {item["id"]: (item["bounds"]["x"], item["bounds"]["y"], item["bounds"]["width"],
                        item["bounds"]["height"], item["scale"]) for item in data}
    require(got == expected, f"list-displays {got} differs from the compositor monitors {expected}")
    require(sum(1 for item in data if item["is_primary"]) == 1, "not exactly one primary display")
    ctx.after("hyprland.monitors", monitor_state(ctx))
    ctx.restored("hyprland.monitors", monitor_state(ctx))


def product_names(data):
    return {item["name"] for item in nodes(data) if isinstance(item.get("name"), str) and item["name"]}


def snapshot_names(ctx, app, expected):
    tree = root(ctx, app)

    def stable():
        present = set(raw_names(tree))
        return [name for name in expected if name in present]

    require(raw.poll(lambda: not [name for name in expected if name not in set(raw_names(tree))], 40.0),
            "the app never exposed the expected names in its AT-SPI tree")
    ctx.before("atspi", stable())
    data = snapshot(ctx, app)
    require(data["ref_count"] > 0, "snapshot carries no refs")
    names = product_names(data)
    missing = [name for name in expected if name not in names]
    require(not missing, f"snapshot lacks {missing}; it has {sorted(names)[:40]}")
    invented = sorted(names - set(raw_names(tree)))
    require(not invented, f"snapshot names absent from the AT-SPI tree: {invented[:10]}")
    ctx.after("atspi", stable())
    ctx.restored("atspi", stable())


@implementation("hyprland.snapshot_calculator")
def snapshot_calculator(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        snapshot_names(ctx, app, CALCULATOR_NAMES)


@implementation("hyprland.snapshot_qt")
def snapshot_qt(ctx: LegContext) -> None:
    with qt_session(ctx) as app:
        snapshot_names(ctx, app, QT_NAMES)


@implementation("hyprland.snapshot_code")
def snapshot_code(ctx: LegContext) -> None:
    with launched(ctx, "Code", 60.0) as app:
        snapshot_names(ctx, app, CODE_NAMES)


@implementation("hyprland.snapshot_firefox")
def snapshot_firefox(ctx: LegContext) -> None:
    with launched(ctx, "Firefox", 60.0) as app:
        snapshot_names(ctx, app, FIREFOX_NAMES)


@implementation("hyprland.find_get_is_calculator")
def find_get_is_calculator(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        ctx.before("atspi", display(tree))
        seven = ref(ctx, app, role="button", name="7")
        raw_button = one(tree, role=43, name="7")
        text = command(ctx, ["get", seven, "--property", "text"], app).data
        require(text["value"] == raw_button.name, f"get text {text['value']!r} != raw name {raw_button.name!r}")
        role = command(ctx, ["get", seven, "--property", "role"], app).data
        require(role["value"] == "button", f"get role is {role['value']!r}")
        for prop, raw_state in (("visible", "SHOWING"), ("enabled", "SENSITIVE"), ("focused", "FOCUSED")):
            reply = command(ctx, ["is", seven, "--property", prop], app).data
            require(reply["result"] is (raw_state in raw_button.states),
                    f"is {prop} says {reply['result']} but the raw states are {sorted(raw_button.states)}")
        undo = ref(ctx, app, role="button", name="Undo")
        require(undo != seven, "distinct elements share a ref")
        found = command(ctx, ["find", "--role", "button", "--name", "Undo", "--exact"], app).data
        matches = found["matches"]
        require(len(matches) == 1 and matches[0]["name"] == "Undo" and matches[0]["role"] == "button",
                f"find lists {matches}, not the one Undo button")
        raw_undo = one(tree, role=43, name="Undo")
        require(("disabled" in matches[0]["states"]) == ("SENSITIVE" not in raw_undo.states),
                f"find states {matches[0]['states']} disagree with the raw states {sorted(raw_undo.states)}")
        listed = command(ctx, ["is", matches[0]["ref_id"], "--property", "enabled"], app).data
        require(listed["result"] is ("SENSITIVE" in raw_undo.states),
                "the ref find lists does not resolve to the raw Undo button")
        ctx.after("atspi", display(tree))
        ctx.restored("atspi", display(tree))


def png_size(data: bytes) -> tuple[int, int]:
    require(data[:8] == PNG_SIGNATURE, "screenshot is not a PNG")
    return struct.unpack(">II", data[16:24])


def distinct_pixels(data: bytes) -> int:
    from oracle import pixels
    image = pixels.decode_png(data)
    step = max(1, image.width * image.height // 4000)
    return len({image.pixel(index % image.width, index // image.width)
                for index in range(0, image.width * image.height, step)})


@implementation("hyprland.screenshot_screen_matches_monitor")
def screenshot_screen_matches_monitor(ctx: LegContext) -> None:
    with managed(ctx, "Calculator"):
        ctx.before("hyprland.monitors", monitor_state(ctx))
        target = ctx.tier.runtime_dir / "screen.png"
        command(ctx, ["screenshot", str(target), "--screen", "0"])
        data = target.read_bytes()
        _, _, _, width, height, _ = monitor_state(ctx)[0]
        require(png_size(data) == (width, height), f"screenshot is {png_size(data)}, monitor is {(width, height)}")
        require(distinct_pixels(data) > 1, "the screenshot is a single flat colour")
        target.unlink()
        ctx.after("hyprland.monitors", monitor_state(ctx))
        ctx.restored("hyprland.monitors", monitor_state(ctx))


@implementation("hyprland.screenshot_window_matches_client")
def screenshot_window_matches_client(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        view = window(ctx, app)
        ctx.before("hyprland.windows", windows_state(ctx))
        target = ctx.tier.runtime_dir / "window.png"
        command(ctx, ["screenshot", str(target), "--window-id", app.windows[0]])
        data = target.read_bytes()
        require(png_size(data) == view.size, f"screenshot is {png_size(data)}, the window is {view.size}")
        require(distinct_pixels(data) > 1, "the screenshot is a single flat colour")
        target.unlink()
        ctx.after("hyprland.windows", windows_state(ctx))
        ctx.restored("hyprland.windows", windows_state(ctx))
