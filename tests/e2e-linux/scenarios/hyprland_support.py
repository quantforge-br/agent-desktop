from __future__ import annotations

import contextlib
import shutil
import subprocess
from dataclasses import asdict

from oracle import atspi_raw as raw, hyprland
from runner.helper import managed, remove, require
from scenarios.toolkit_qt import log_state

TEXT = "org.a11y.atspi.Text"
EDITABLE = "org.a11y.atspi.EditableText"
ROLE_BUTTON = 43
ROLE_ENTRY = 61
ROLE_COMBO = 11
ROLE_MENU = 33
ROLE_MENU_ITEM = 35
ROLE_TOGGLE = 62


def additions(ctx, baseline: set) -> dict:
    """The Qt window's log and the session processes started since `baseline`.

    A clipboard server that served the session before can exit once a leg replaces the clipboard, so only what
    the Qt session added counts, never what left.
    """
    state = log_state(ctx)
    added = sorted(set(map(tuple, state["members"])) - baseline)
    return {"log": state["log"], "added": [list(member) for member in added]}


@contextlib.contextmanager
def qt_session(ctx):
    """The Qt window with its activation log and added processes read before, after and once removed again."""
    path = ctx.tier.runtime_dir / "qt-window.log"
    remove(path)
    baseline = set(map(tuple, log_state(ctx)["members"]))
    before = additions(ctx, baseline)
    ctx.before("applog", before)
    try:
        with managed(ctx, "Qt") as app:
            try:
                yield app
            finally:
                ctx.after("applog", additions(ctx, baseline))
    finally:
        remove(path)
        raw.poll(lambda: additions(ctx, baseline) == before, 3.0)
        restored = additions(ctx, baseline)
        require(restored == before, f"the Qt app left its log or processes behind: {restored}")
        ctx.restored("applog", restored)


def window(ctx, app) -> hyprland.WindowView:
    return hyprland.window_of(ctx.tier, app.pid)


def window_state(ctx, app) -> dict:
    return asdict(window(ctx, app))


def center(ctx, app, node) -> tuple[int, int]:
    """Layout coordinates of a raw node's centre: the compositor's window origin plus its window-relative extents."""
    x, y, width, height = node.extents(window_coords=True)
    origin = window(ctx, app).at
    return origin[0] + x + width // 2, origin[1] + y + height // 2


def selection(node) -> list[tuple[int, int]]:
    count = node.invoke(TEXT, "GetNSelections")[0]
    return [tuple(node.invoke(TEXT, "GetSelection", "(i)", (index,))) for index in range(count)]


def set_text(node, text: str) -> None:
    require(node.invoke(EDITABLE, "SetTextContents", "(s)", (text,))[0], "raw text write refused")


def keys_from_outside(tier, *keys: str) -> None:
    """Key presses from a foreign virtual keyboard client, used only to put the app back where it was."""
    require(shutil.which("wtype"), "wtype is required to restore keyboard state independently")
    argv = ["wtype"]
    for key in keys:
        argv += ["-k", key]
    process = subprocess.run(argv, env={**tier.env, "WAYLAND_DISPLAY": tier.wayland_display},
                             capture_output=True, text=True, timeout=5, check=False)
    require(process.returncode == 0, f"wtype failed: {process.stderr.strip()}")


def grab(tier, region: tuple[int, int, int, int]):
    """Pixels of a layout-coordinate region as grim reads them from the nested compositor, without the cursor."""
    from oracle import pixels
    require(shutil.which("grim"), "grim is required to read compositor pixels independently")
    x, y, width, height = region
    process = subprocess.run(["grim", "-g", f"{x},{y} {width}x{height}", "-"],
                             env={**tier.env, "WAYLAND_DISPLAY": tier.wayland_display},
                             capture_output=True, timeout=10, check=False)
    require(process.returncode == 0, f"grim failed: {process.stderr.decode(errors='replace').strip()}")
    return pixels.decode_png(process.stdout)


@contextlib.contextmanager
def input_state(ctx):
    """Records the pointer and the keyboard-focused window around a leg, and puts both back."""
    tier = ctx.tier
    cursor, active = hyprland.cursor(tier), hyprland.active_address(tier)
    ctx.before("hyprland.cursor", list(cursor))
    ctx.before("hyprland.active", active)
    try:
        yield (cursor, active)
    finally:
        ctx.after("hyprland.cursor", list(hyprland.cursor(tier)))
        ctx.after("hyprland.active", hyprland.active_address(tier))
        # Focusing a window warps the cursor to its centre, so the pointer goes back after the focus.
        if active and any(view.address == active for view in hyprland.windows(tier)):
            hyprland.focus(tier, active)
        hyprland.move_cursor(tier, *cursor)
        require(raw.poll(lambda: hyprland.active_address(tier) == active, 3.0),
                "the keyboard-focused window was not restored")
        ctx.restored("hyprland.cursor", list(hyprland.cursor(tier)))
        ctx.restored("hyprland.active", hyprland.active_address(tier))


@contextlib.contextmanager
def undisturbed(ctx):
    """A headless product command leaves the pointer and the focused window where they were."""
    tier = ctx.tier
    before = hyprland.cursor(tier), hyprland.active_address(tier)
    yield
    after = hyprland.cursor(tier), hyprland.active_address(tier)
    require(before == after, f"headless command moved the pointer or the focus: {before} -> {after}")


def raw_names(tree) -> list[str]:
    return sorted({node.name for node in tree.walk() if node.name})


def entry_of(tree):
    entries = [node for node in tree.find(role=ROLE_ENTRY) if node.name == "Text entry"]
    require(len(entries) == 1, f"expected one Text entry, found {len(entries)}")
    return entries[0]
