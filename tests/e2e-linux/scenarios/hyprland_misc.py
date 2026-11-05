from __future__ import annotations

import subprocess

from oracle import atspi_raw as raw, hyprland
from runner import apps
from runner.helper import changed, command, delayed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_keys import entry_state, qt_entry
from scenarios.hyprland_support import qt_session

WAIT_BUDGET_MS = "15000"
PASTE_TEXT = "pasted from the product clipboard"


def event_kind(envelope) -> str:
    return str(((envelope.data or {}).get("event") or {}).get("kind", "")).replace("_", "-")


def addresses(ctx):
    return sorted(view.address for view in hyprland.windows(ctx.tier))


@implementation("hyprland.wait_event_window_opened")
def wait_event_window_opened(ctx: LegContext) -> None:
    ctx.before("hyprland.windows", addresses(ctx))
    spawned = []
    trigger = delayed(1.5, lambda: spawned.append(apps.launch_qt(ctx.tier)))
    try:
        reply = command(ctx, ["wait", "--event", "window-opened", "--timeout", WAIT_BUDGET_MS])
        trigger.join()
        require(event_kind(reply) == "window-opened", f"wait reported {reply.raw}")
        require(len(spawned) == 1 and hyprland.windows_of(ctx.tier, spawned[0].pid),
                "the compositor shows no new window although the wait succeeded")
        ctx.after("hyprland.windows", addresses(ctx))
    finally:
        trigger.cancel()
        for app in spawned:
            apps.stop(app)
        ctx.restored("hyprland.windows", changed(lambda: addresses(ctx), []))


@implementation("hyprland.wait_event_window_closed")
def wait_event_window_closed(ctx: LegContext) -> None:
    ctx.before("hyprland.windows", addresses(ctx))
    app = apps.launch_qt(ctx.tier)
    trigger = None
    try:
        require(hyprland.windows_of(ctx.tier, app.pid), "the staged window is not on the compositor")
        trigger = delayed(1.5, lambda: apps.stop(app))
        reply = command(ctx, ["wait", "--event", "window-closed", "--timeout", WAIT_BUDGET_MS])
        trigger.join()
        require(event_kind(reply) == "window-closed", f"wait reported {reply.raw}")
        require(not hyprland.windows_of(ctx.tier, app.pid), "the compositor still shows the closed window")
        ctx.after("hyprland.windows", addresses(ctx))
    finally:
        if trigger is not None:
            trigger.cancel()
        apps.stop(app)
        ctx.restored("hyprland.windows", changed(lambda: addresses(ctx), []))


@implementation("hyprland.wait_event_quiet_times_out")
def wait_event_quiet_times_out(ctx: LegContext) -> None:
    with qt_session(ctx) as app:
        ctx.before("hyprland.windows", addresses(ctx))
        reply = ctx.product(["wait", "--event", "window-opened", "--timeout", "1500"])
        require(not reply.ok and (reply.error or {}).get("code") == "TIMEOUT",
                f"a quiet wait should time out: {reply.raw}")
        require(hyprland.windows_of(ctx.tier, app.pid), "the quiet wait changed the window")
        ctx.after("hyprland.windows", addresses(ctx))
        ctx.restored("hyprland.windows", addresses(ctx))


def paste_from_outside(tier) -> None:
    """Ctrl+V from a foreign virtual keyboard client: the receiving app reads the selection itself."""
    process = subprocess.run(["wtype", "-M", "ctrl", "-k", "v", "-m", "ctrl"],
                             env={**tier.env, "WAYLAND_DISPLAY": tier.wayland_display},
                             capture_output=True, text=True, timeout=5, check=False)
    require(process.returncode == 0, f"wtype failed: {process.stderr.strip()}")


@implementation("hyprland.clipboard_set_reaches_focused_app")
def clipboard_set_reaches_focused_app(ctx: LegContext) -> None:
    with qt_entry(ctx, "") as (_, _, entry):
        command(ctx, ["clipboard-set", PASTE_TEXT])
        try:
            paste_from_outside(ctx.tier)
            changed(entry.text, PASTE_TEXT, seconds=4)
            ctx.after("atspi.entry", entry_state(entry))
        finally:
            command(ctx, ["clipboard-clear"])


@implementation("hyprland.clipboard_get_reads_app_selection")
def clipboard_get_reads_app_selection(ctx: LegContext) -> None:
    with qt_entry(ctx, PASTE_TEXT) as (_, _, entry):
        subprocess.run(["wtype", "-M", "ctrl", "-k", "a", "-k", "c", "-m", "ctrl"],
                       env={**ctx.tier.env, "WAYLAND_DISPLAY": ctx.tier.wayland_display},
                       capture_output=True, timeout=5, check=True)
        try:
            got = raw.poll(lambda: (command(ctx, ["clipboard-get"]).data or {}).get("text") or None, 4.0)
            require(got == PASTE_TEXT, f"clipboard-get returned {got!r}, the app copied {PASTE_TEXT!r}")
            ctx.after("atspi.entry", entry_state(entry))
        finally:
            command(ctx, ["clipboard-clear"])
