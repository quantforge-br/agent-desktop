from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw
from runner.helper import activate, changed, command, display, managed, one, ref, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_support import entry_of, input_state, qt_session, selection, set_text

SEED = "hello world"
COMPONENT = "org.a11y.atspi.Component"


def entry_state(entry):
    return [entry.text(), selection(entry)]


@contextlib.contextmanager
def qt_entry(ctx, seed):
    """The Qt window with its text entry holding SEED and keyboard focus; text and app log are put back."""
    with qt_session(ctx) as app, input_state(ctx):
        tree = root(ctx, app)
        entry = entry_of(tree)
        ctx.before("atspi.entry", entry_state(entry))
        set_text(entry, seed)
        require(entry.invoke(COMPONENT, "GrabFocus")[0], "the entry refused keyboard focus")
        require(raw.poll(lambda: "FOCUSED" in entry.states, 3.0), "the entry never became focused")
        try:
            yield app, tree, entry
        finally:
            set_text(entry, "")
            ctx.restored("atspi.entry", entry_state(entry))


@implementation("hyprland.press_digits_calculator")
def press_digits_calculator(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app, input_state(ctx):
        tree = root(ctx, app)
        ctx.before("atspi", display(tree))
        try:
            command(ctx, ["press", "7"], app)
            command(ctx, ["press", "8"], app)
            changed(lambda: display(tree), "78")
            ctx.after("atspi", display(tree))
        finally:
            # Typed digits never enter the calculator's undo history, so Undo stays disabled; C clears the display.
            activate(one(tree, role=43, name="C"))
            ctx.restored("atspi", changed(lambda: display(tree), ""))


@implementation("hyprland.press_ctrl_a_selects_all")
def press_ctrl_a_selects_all(ctx: LegContext) -> None:
    with qt_entry(ctx, SEED) as (app, _, entry):
        command(ctx, ["press", "ctrl+a"], app)
        require(raw.poll(lambda: selection(entry) == [(0, len(SEED))], 3.0),
                f"ctrl+a left the selection at {selection(entry)}")
        ctx.after("atspi.entry", entry_state(entry))


@implementation("hyprland.press_named_keys_edit_text")
def press_named_keys_edit_text(ctx: LegContext) -> None:
    with qt_entry(ctx, "abc") as (app, _, entry):
        # The product's `delete` is the backspace key; the forward Delete key is `forwarddelete`.
        for key in ("backspace", "home", "forwarddelete"):
            command(ctx, ["press", key], app)
        changed(entry.text, "b")
        ctx.after("atspi.entry", entry_state(entry))


@implementation("hyprland.type_keystrokes_with_unicode")
def type_keystrokes_with_unicode(ctx: LegContext) -> None:
    text = "Héllo wörld 123 ✓"
    with qt_entry(ctx, "") as (app, _, entry):
        target = ref(ctx, app, role="textfield", name="Text entry")
        command(ctx, ["type", target, text, "--interval-ms", "15"], app)
        changed(entry.text, text, seconds=5)
        ctx.after("atspi.entry", entry_state(entry))


@implementation("hyprland.modifiers_released_after_combo")
def modifiers_released_after_combo(ctx: LegContext) -> None:
    with qt_entry(ctx, "") as (app, _, entry):
        command(ctx, ["press", "ctrl+shift+alt+a"], app)
        command(ctx, ["press", "x"], app)
        changed(entry.text, "x")
        ctx.after("atspi.entry", entry_state(entry))
