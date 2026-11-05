from __future__ import annotations

import contextlib
from uuid import uuid4

from oracle import atspi_raw as raw
from runner.helper import activate, changed, command, display, managed, one, ref, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_keys import entry_state
from scenarios.hyprland_support import ROLE_COMBO, entry_of, input_state, qt_session, set_text, undisturbed
from scenarios.ref_contract import matching_event, reset, since_reset
from scenarios.toolkit_qt import events


@contextlib.contextmanager
def qt_semantic(ctx):
    """The Qt window for headless actions: the text entry and the app log are read around the leg."""
    with qt_session(ctx) as app:
        tree = root(ctx, app)
        entry = entry_of(tree)
        ctx.before("atspi.entry", entry_state(entry))
        try:
            yield app, tree, entry
        finally:
            set_text(entry, "")
            ctx.restored("atspi.entry", entry_state(entry))


@implementation("hyprland.semantic_click_calculator")
def semantic_click_calculator(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        ctx.before("atspi", display(tree))
        seven = ref(ctx, app, role="button", name="7")
        try:
            with undisturbed(ctx):
                command(ctx, ["click", seven], app)
                changed(lambda: display(tree), "7")
            ctx.after("atspi", display(tree))
        finally:
            activate(one(tree, role=43, name="Undo"))
            ctx.restored("atspi", changed(lambda: display(tree), ""))


@implementation("hyprland.semantic_text_actions_qt")
def semantic_text_actions_qt(ctx: LegContext) -> None:
    with qt_semantic(ctx) as (app, _, entry):
        target = ref(ctx, app, role="textfield", name="Text entry")
        with undisturbed(ctx):
            command(ctx, ["type", target, "typed text"], app)
            changed(entry.text, "typed text")
            command(ctx, ["set-value", target, "set value"], app)
            changed(entry.text, "set value")
            command(ctx, ["clear", target], app)
            changed(entry.text, "")
            command(ctx, ["type", target, "again"], app)
            changed(entry.text, "again")
        ctx.after("atspi.entry", entry_state(entry))


@implementation("hyprland.semantic_select_combo_qt")
def semantic_select_combo_qt(ctx: LegContext) -> None:
    """A Qt combobox select needs the pointer path: headless is POLICY_DENIED, --headed picks the item."""
    def choice():
        return sorted(node.name for node in tree.find(role=ROLE_COMBO))

    with qt_session(ctx) as app, input_state(ctx):
        tree = root(ctx, app)
        path = ctx.tier.runtime_dir / "qt-window.log"
        before = events(path)
        combo = ref(ctx, app, role="combobox", name="Alpha")
        refusal = ctx.product(["select", combo, "Charlie"], headed=False)
        error = refusal.error or {}
        require(not refusal.ok and error.get("code") == "POLICY_DENIED"
                and (error.get("details") or {}).get("kind") == "pointer_required"
                and (error.get("disposition") or {}).get("delivery") == "not_delivered",
                f"headless combobox select is not a pointer_required refusal: {refusal.raw}")
        require(choice() == ["Alpha"] and events(path) == before, "the refused select changed the combo")
        command(ctx, ["select", combo, "Charlie"], app, headed=True)
        changed(choice, ["Charlie"])
        require(raw.poll(lambda: events(path)[len(before):][-1:] == ["choice:Charlie"], 3.0),
                f"selecting Charlie logged {events(path)[len(before):]}")


@implementation("hyprland.semantic_click_qt_button_leaves_input_alone")
def semantic_click_qt_button_leaves_input_alone(ctx: LegContext) -> None:
    with qt_semantic(ctx) as (app, _, entry):
        path = ctx.tier.runtime_dir / "qt-window.log"
        before = events(path)
        button = ref(ctx, app, role="button", name="Press me")
        with undisturbed(ctx):
            command(ctx, ["click", button], app)
            require(raw.poll(lambda: events(path)[len(before):].count("click") == 1, 3.0),
                    f"the button logged {events(path)[len(before):]}, not one click")
        ctx.after("atspi.entry", entry_state(entry))


@implementation("hyprland.check_gtk4_checkbox_without_actions")
def check_gtk4_checkbox_without_actions(ctx: LegContext) -> None:
    """A GTK 4 check button advertises no action: check is refused headless and clicked headed."""
    path = ctx.tier.runtime_dir / f"check-gtk4-{uuid4().hex}.log"
    with managed(ctx, "Zoo GTK4", (str(path),)) as app:
        reset(path)
        box = one(root(ctx, app), role=7, name="zoo-check")
        checked = lambda: "CHECKED" in box.states
        require(not checked(), "zoo-check must start unchecked")
        ctx.before("atspi", checked())
        target = ref(ctx, app, role="checkbox", name="zoo-check")
        refused = ctx.product(["check", target], headed=False)
        require(not refused.ok and (refused.error or {}).get("code") == "POLICY_DENIED", refused.raw)
        require(not checked() and not since_reset(path), "a refused check reached the box")
        try:
            response = ctx.product(["check", target], headed=True)
            require(response.ok, response.raw)
            require((response.data or {}).get("disposition", {}).get("delivery") == "delivered_verified", response.raw)
            changed(checked, True)
            require(matching_event(since_reset(path), "zoo-check", "toggled", True), "the app logged no check")
            ctx.after("atspi", checked())
        finally:
            ctx.product(["uncheck", target], headed=True)
            ctx.restored("atspi", changed(checked, False))


@implementation("hyprland.scroll_wheel_moves_scroll_area")
def scroll_wheel_moves_scroll_area(ctx: LegContext) -> None:
    """scroll wheels the virtual pointer over a scroll area; its scrollbar's raw value is the effect."""
    path = ctx.tier.runtime_dir / f"scroll-gtk4-{uuid4().hex}.log"
    with managed(ctx, "Zoo GTK4", (str(path),)) as app:
        reset(path)
        bar = one(root(ctx, app), role=48, name="zoo-scroll-vscroll")

        def position() -> float:
            return round(bar.value(), 3)

        require(position() == 0.0, f"the scroll area must start at the top, not {bar.value()}")
        ctx.before("atspi", position())
        target = ref(ctx, app, role="scrollarea")
        refused = ctx.product(["scroll", target, "--direction", "down"], headed=False)
        require(not refused.ok and (refused.error or {}).get("code") == "POLICY_DENIED", refused.raw)
        require(position() == 0.0, "a refused scroll moved the scroll area")
        try:
            response = ctx.product(["scroll", target, "--direction", "down", "--amount", "3"], headed=True)
            require(response.ok, response.raw)
            require(raw.poll(lambda: position() > 0.0, 3.0), "the wheel did not move the scroll area")
            ctx.after("atspi", position())
        finally:
            ctx.product(["scroll", target, "--direction", "up", "--amount", "10"], headed=True)
            ctx.restored("atspi", changed(position, 0.0))
