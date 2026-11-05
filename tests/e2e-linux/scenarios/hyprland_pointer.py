from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw, hyprland, pixels
from runner.helper import changed, command, ref, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_support import (ROLE_BUTTON, ROLE_COMBO, center, entry_of, grab, input_state,
                                        keys_from_outside, qt_session, selection, set_text)
from scenarios.toolkit_qt import events

SEED = "alpha beta gamma"
STATUS_TIP = "Pointer over the button"
TEXT_INSET_PX = 10
DRAG_INSET_PX = 4
MENU_REGION = (260, 150)
MENU_CHANGED = 0.3


def at(point):
    return f"{point[0]},{point[1]}"


@contextlib.contextmanager
def qt_pointer(ctx):
    """The Qt window under pointer restoration; the activation log is read after and then removed."""
    with qt_session(ctx) as app, input_state(ctx):
        yield app, root(ctx, app)


def one_named(tree, role, name):
    found = [node for node in tree.find(role=role, name=name)]
    require(len(found) == 1, f"expected one {name!r}, found {len(found)}")
    return found[0]


def text_point(ctx, app, entry, offset=TEXT_INSET_PX):
    x, y = center(ctx, app, entry)
    left, _, width, _ = entry.extents(window_coords=True)
    return x - width // 2 + offset, y


def cursor_at(ctx, point):
    return raw.poll(lambda: hyprland.cursor(ctx.tier) == tuple(point), 3.0)


@implementation("hyprland.mouse_move_reaches_cursor")
def mouse_move_reaches_cursor(ctx: LegContext) -> None:
    with qt_pointer(ctx):
        for point in ((700, 300), (50, 60)):
            command(ctx, ["mouse-move", "--xy", at(point)])
            require(cursor_at(ctx, point), f"the compositor's cursor is at {hyprland.cursor(ctx.tier)}, not {point}")


@implementation("hyprland.mouse_click_presses_button")
def mouse_click_presses_button(ctx: LegContext) -> None:
    with qt_pointer(ctx) as (app, tree):
        path = ctx.tier.runtime_dir / "qt-window.log"
        before = events(path)
        point = center(ctx, app, one_named(tree, ROLE_BUTTON, "Press me"))
        command(ctx, ["mouse-click", "--xy", at(point)])
        require(raw.poll(lambda: events(path)[len(before):].count("click") == 1, 3.0),
                f"the button logged {events(path)[len(before):]}, not one click")
        require(events(path)[len(before):].count("click") == 1, "more than one activation")
        require(cursor_at(ctx, point), "the click did not leave the cursor on its target")


@implementation("hyprland.mouse_double_click_selects_word")
def mouse_double_click_selects_word(ctx: LegContext) -> None:
    with qt_pointer(ctx) as (app, tree):
        entry = entry_of(tree)
        set_text(entry, SEED)
        point = text_point(ctx, app, entry)
        command(ctx, ["mouse-click", "--xy", at(point), "--count", "2"])
        require(raw.poll(lambda: selection(entry) == [(0, 5)], 3.0),
                f"a double click selected {selection(entry)}, not the first word")
        set_text(entry, "")


@implementation("hyprland.mouse_right_click_opens_context_menu")
def mouse_right_click_opens_context_menu(ctx: LegContext) -> None:
    # Qt's context menu is a popup that its AT-SPI tree never lists, so the compositor's pixels are the oracle.
    def menu_region(point):
        # Starts below the entry so its focus ring, which stays after the menu closes, is not in the region.
        return grab(ctx.tier, (point[0], point[1] + 15, MENU_REGION[0], MENU_REGION[1]))

    with qt_pointer(ctx) as (app, tree):
        entry = entry_of(tree)
        point = center(ctx, app, entry)
        before = menu_region(point)

        def menu_shown() -> bool:
            return pixels.mismatch_fraction(before, menu_region(point)) > MENU_CHANGED

        ctx.before("pixels.menu_shown", menu_shown())
        command(ctx, ["mouse-click", "--xy", at(point), "--button", "right"])
        require(raw.poll(menu_shown, 3.0), "the pixels under the right click never changed into a menu")
        ctx.after("pixels.menu_shown", menu_shown())
        keys_from_outside(ctx.tier, "Escape")
        # About 0.4% of the pixels stay repainted once the menu is gone; a menu differs in over 80%.
        require(raw.poll(lambda: not menu_shown(), 3.0), "the menu is still on screen after Escape")
        ctx.restored("pixels.menu_shown", menu_shown())


@implementation("hyprland.mouse_wheel_scrolls_combo")
def mouse_wheel_scrolls_combo(ctx: LegContext) -> None:
    with qt_pointer(ctx) as (app, tree):
        path = ctx.tier.runtime_dir / "qt-window.log"
        before = events(path)
        x, y = center(ctx, app, one_named(tree, ROLE_COMBO, "Alpha"))
        command(ctx, ["mouse-wheel", f"--x={x}", f"--y={y}", "--dy=-1"])
        require(raw.poll(lambda: events(path)[len(before):] == ["choice:Bravo"], 3.0),
                f"the wheel produced {events(path)[len(before):]}, not a step to Bravo")


@implementation("hyprland.hover_xy_exposes_status_tip")
def hover_xy_exposes_status_tip(ctx: LegContext) -> None:
    with qt_pointer(ctx) as (app, tree):
        ctx.before("atspi", tip_hidden(ctx, tree))
        point = center(ctx, app, one_named(tree, ROLE_BUTTON, "Press me"))
        command(ctx, ["hover", "--xy", at(point)])
        require(raw.poll(lambda: status_tip_shown(tree), 3.0), "the status tip never appeared under the pointer")
        require(cursor_at(ctx, point), "the cursor is not on the hover target")
        ctx.after("atspi", status_tip_shown(tree))
        hyprland.move_cursor(ctx.tier, 5, 5)
        ctx.restored("atspi", changed(lambda: status_tip_shown(tree), False))


@implementation("hyprland.hover_ref_exposes_status_tip")
def hover_ref_exposes_status_tip(ctx: LegContext) -> None:
    with qt_pointer(ctx) as (app, tree):
        ctx.before("atspi", tip_hidden(ctx, tree))
        target = ref(ctx, app, role="button", name="Press me")
        command(ctx, ["hover", target], app)
        require(raw.poll(lambda: status_tip_shown(tree), 3.0), "the status tip never appeared under the pointer")
        expected = center(ctx, app, one_named(tree, ROLE_BUTTON, "Press me"))
        landed = hyprland.cursor(ctx.tier)
        require(abs(landed[0] - expected[0]) <= 3 and abs(landed[1] - expected[1]) <= 3,
                f"the pointer is at {landed}, the element centre is {expected}")
        ctx.after("atspi", status_tip_shown(tree))
        hyprland.move_cursor(ctx.tier, 5, 5)
        ctx.restored("atspi", changed(lambda: status_tip_shown(tree), False))


def tip_hidden(ctx, tree) -> bool:
    """Parks the cursor in the corner, where an earlier leg's resting pointer cannot show the tip."""
    hyprland.move_cursor(ctx.tier, 5, 5)
    require(raw.poll(lambda: not status_tip_shown(tree), 3.0), "the status tip stays shown with the cursor parked")
    return status_tip_shown(tree)


def status_tip_shown(tree) -> bool:
    return any(STATUS_TIP in (node.name or "") or STATUS_TIP in (node.text() or "")
               for node in tree.walk() if "SHOWING" in node.states)


@implementation("hyprland.drag_selects_text")
def drag_selects_text(ctx: LegContext) -> None:
    with qt_pointer(ctx) as (app, tree):
        entry = entry_of(tree)
        set_text(entry, SEED)
        start = text_point(ctx, app, entry, DRAG_INSET_PX)
        end = (start[0] + 70, start[1])
        command(ctx, ["drag", "--from-xy", at(start), "--to-xy", at(end)])
        require(raw.poll(lambda: bool(selection(entry)) and selection(entry)[0][0] == 0
                         and selection(entry)[0][1] >= 5, 3.0),
                f"dragging across the text selected {selection(entry)}")
        set_text(entry, "")


@implementation("hyprland.pointer_commands_refused_headless")
def pointer_commands_refused_headless(ctx: LegContext) -> None:
    with qt_pointer(ctx) as (app, tree):
        path = ctx.tier.runtime_dir / "qt-window.log"
        before = events(path)
        point = center(ctx, app, one_named(tree, ROLE_BUTTON, "Press me"))
        cursor = hyprland.cursor(ctx.tier)
        for argv in (["mouse-click", "--xy", at(point)], ["mouse-move", "--xy", at(point)],
                     ["hover", "--xy", at(point)], ["mouse-wheel", f"--x={point[0]}", f"--y={point[1]}", "--dy=-1"]):
            reply = ctx.product(argv, headed=False)
            require(not reply.ok, f"{argv[0]} ran without --headed: {reply.raw}")
            require((reply.error or {}).get("disposition", {}).get("delivery") == "not_delivered",
                    f"{argv[0]} refusal does not say not_delivered: {reply.raw}")
        require(hyprland.cursor(ctx.tier) == cursor, "a refused pointer command moved the cursor")
        require(events(path) == before, "a refused pointer command reached the app")
