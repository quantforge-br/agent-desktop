from __future__ import annotations

import contextlib
from dataclasses import asdict

from oracle import atspi_raw as raw, hyprland
from runner.helper import command, managed, require
from runner.legs import implementation
from runner.verdict import LegContext

from scenarios.hyprland_support import qt_session, window

TARGET = (100, 120)
SIZE = (640, 420)
SLACK_PX = 2


@contextlib.contextmanager
def target_window(ctx):
    """The Qt window beside Calculator; its compositor state is read first and put back through hyprctl."""
    with managed(ctx, "Calculator") as calculator, qt_session(ctx) as qt:
        before = window(ctx, qt)
        ctx.before("hyprland.window", asdict(before))
        try:
            yield qt, calculator, before
        finally:
            ctx.after("hyprland.window", asdict(hyprland.view_at(ctx.tier, before.address)))
            ctx.restored("hyprland.window", asdict(hyprland.restore(ctx.tier, before)))


def op(ctx, name, app, *extra):
    return command(ctx, [name, "--window-id", app.windows[0], *extra])


def near(actual, wanted):
    return all(abs(a - w) <= SLACK_PX for a, w in zip(actual, wanted))


@implementation("hyprland.focus_window")
def focus_window(ctx: LegContext) -> None:
    with target_window(ctx) as (qt, calculator, before):
        other = window(ctx, calculator).address
        hyprland.focus(ctx.tier, other)
        require(raw.poll(lambda: hyprland.active_address(ctx.tier) == other, 3.0), "could not stage the other window")
        ctx.before("hyprland.active", hyprland.active_address(ctx.tier))
        try:
            op(ctx, "focus-window", qt)
            require(raw.poll(lambda: hyprland.active_address(ctx.tier) == before.address, 4.0),
                    f"the compositor's active window is {hyprland.active_address(ctx.tier)}, not the target")
            ctx.after("hyprland.active", hyprland.active_address(ctx.tier))
        finally:
            hyprland.focus(ctx.tier, other)
            raw.poll(lambda: hyprland.active_address(ctx.tier) == other, 3.0)
            ctx.restored("hyprland.active", hyprland.active_address(ctx.tier))


@implementation("hyprland.move_window")
def move_window(ctx: LegContext) -> None:
    with target_window(ctx) as (qt, _, before):
        op(ctx, "move-window", qt, "--x", str(TARGET[0]), "--y", str(TARGET[1]))
        moved = hyprland.wait_view(ctx.tier, before.address, lambda view: near(view.at, TARGET), 6.0)
        require(moved is not None, f"the window is at {hyprland.view_at(ctx.tier, before.address).at}, not {TARGET}")
        require(moved.floating, "a moved window must leave the tiling layout")
        require(moved.size == before.size or moved.floating, "the move changed the size")


@implementation("hyprland.resize_window")
def resize_window(ctx: LegContext) -> None:
    with target_window(ctx) as (qt, _, before):
        op(ctx, "resize-window", qt, "--width", str(SIZE[0]), "--height", str(SIZE[1]))
        resized = hyprland.wait_view(ctx.tier, before.address, lambda view: near(view.size, SIZE), 6.0)
        require(resized is not None,
                f"the window is {hyprland.view_at(ctx.tier, before.address).size}, not {SIZE}")
        require(resized.floating, "a resized window must leave the tiling layout")


def covers_monitor(ctx, view) -> bool:
    monitor = hyprland.monitors(ctx.tier)[0]
    return view.size[0] >= monitor["width"] - 2 * SLACK_PX and view.size[1] >= monitor["height"] - 2 * SLACK_PX


@implementation("hyprland.maximize_restore")
def maximize_restore(ctx: LegContext) -> None:
    with target_window(ctx) as (qt, _, before):
        op(ctx, "maximize", qt)
        grown = hyprland.wait_view(ctx.tier, before.address, lambda view: view.fullscreen in (1, 3), 6.0)
        require(grown is not None, "the compositor does not report the window as maximised")
        require(covers_monitor(ctx, grown), f"the maximised window is only {grown.size}")
        op(ctx, "restore", qt)
        back = hyprland.wait_view(ctx.tier, before.address, lambda view: view.fullscreen == 0, 6.0)
        require(back is not None, "restore left the window maximised")
        require(hyprland.settle(ctx.tier, before.address, before).geometry() == before.geometry(),
                f"restore did not return the geometry {before.geometry()}")


@implementation("hyprland.minimize_restore")
def minimize_restore(ctx: LegContext) -> None:
    with target_window(ctx) as (qt, _, before):
        op(ctx, "minimize", qt)
        gone = hyprland.wait_view(
            ctx.tier, before.address,
            lambda view: view.workspace != before.workspace or view.hidden or not view.mapped, 6.0)
        require(gone is not None, "the window is still shown on its workspace")
        listed = [item for item in command(ctx, ["list-windows"]).data if item["pid"] == qt.pid]
        require(listed and listed[0]["minimized"] is True, f"list-windows does not report it minimised: {listed}")
        op(ctx, "restore", qt)
        back = hyprland.wait_view(
            ctx.tier, before.address,
            lambda view: view.workspace == before.workspace and view.mapped and not view.hidden, 6.0)
        require(back is not None, "restore did not bring the window back to its workspace")
        listed = [item for item in command(ctx, ["list-windows"]).data if item["pid"] == qt.pid]
        require(listed and listed[0]["minimized"] is False, f"list-windows still reports it minimised: {listed}")


def bystanders(ctx, skip_pid):
    return [(view.address, view.pid, view.workspace, view.title, view.mapped)
            for view in hyprland.windows(ctx.tier) if view.pid != skip_pid]


def close_leg(ctx, *extra):
    with qt_session(ctx) as qt, managed(ctx, "Calculator") as victim:
        views = hyprland.windows_of(ctx.tier, victim.pid)
        require(len(views) == 1, "the victim has no single compositor window")
        ctx.before("hyprland.bystanders", bystanders(ctx, victim.pid))
        command(ctx, ["close-app", "gnome-calculator", *extra])
        require(raw.poll(lambda: not hyprland.windows_of(ctx.tier, victim.pid), 8.0),
                "the compositor still holds the closed app's window")
        require(raw.poll(lambda: not _alive(victim.pid), 8.0), "the closed app's process is still running")
        ctx.after("hyprland.bystanders", bystanders(ctx, victim.pid))
        require(any(view.pid == qt.pid and view.mapped for view in hyprland.windows(ctx.tier)),
                "closing one app removed another app's window")
        ctx.restored("hyprland.bystanders", bystanders(ctx, victim.pid))


def _alive(pid):
    from pathlib import Path
    try:
        stat = (Path("/proc") / str(pid) / "stat").read_text()
    except FileNotFoundError:
        return False
    return stat[stat.rindex(")") + 2:].split()[0] != "Z"


@implementation("hyprland.close_app_graceful")
def close_app_graceful(ctx: LegContext) -> None:
    close_leg(ctx)


@implementation("hyprland.close_app_force")
def close_app_force(ctx: LegContext) -> None:
    close_leg(ctx, "--force")
