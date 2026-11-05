from __future__ import annotations

from oracle import atspi_raw as raw
from runner.helper import changed, command, one, ref, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_observe import launched
from scenarios.hyprland_support import (ROLE_BUTTON, ROLE_TOGGLE, TEXT, keys_from_outside, set_text,
                                        undisturbed)

TYPED = "agent desktop on hyprland"


def urlbar_nodes(tree):
    # The bar reads "Search or enter address" until a search engine loads, then names the engine.
    return [node for node in tree.walk() if node.name.startswith("Search") and
            node.name.endswith("enter address") and TEXT in node.interfaces()]


def urlbar_node(tree):
    # The chrome loads after the window maps, so the bar appears some time after the app is ready.
    found = raw.poll(lambda: len(urlbar_nodes(tree)) >= 1 and urlbar_nodes(tree), 30.0) or []
    require(len(found) == 1, f"expected one editable address bar, found {len(found)}")
    return found[0]


def refused_without_focus(response) -> bool:
    """A bar that is not EDITABLE takes text only through focus and keys, which a headless type may not use."""
    error = response.error or {}
    checks = (error.get("details") or {}).get("checks", ())
    return error.get("code") == "POLICY_DENIED" and \
        error.get("disposition", {}).get("delivery") == "not_delivered" and \
        any(check.get("check") == "supported_action" and check.get("status") == "fail" for check in checks)


@implementation("hyprland.firefox_type_into_address_bar")
def firefox_type_into_address_bar(ctx: LegContext) -> None:
    with launched(ctx, "Firefox", 60.0) as app:
        tree = root(ctx, app)
        bar = urlbar_node(tree)
        before = bar.text()
        require(before == "", f"about:blank should leave the address bar empty, read {before!r}")
        ctx.before("atspi", before)
        target = ref(ctx, app, role="combobox", name=bar.name)
        try:
            with undisturbed(ctx):
                reply = ctx.product(["type", target, TYPED])
            if not reply.ok:
                require(refused_without_focus(reply), f"headless type failed otherwise: {reply.raw}")
                require(bar.text() == before, "the refused headless type changed the address bar")
                reply = command(ctx, ["type", target, TYPED], app, headed=True)
            require(reply.data["disposition"]["delivery"] == "delivered_verified",
                    f"type was not verified: {reply.raw}")
            changed(bar.text, TYPED, seconds=5)
            ctx.after("atspi", bar.text())
        finally:
            if "EDITABLE" in bar.states:
                set_text(bar, before)
            else:
                # Escape closes the results and then reverts the bar to the page address.
                keys_from_outside(ctx.tier, "Escape", "Escape")
            ctx.restored("atspi", changed(bar.text, before))


SIDE_BAR_SWITCH = "Toggle Primary Side Bar (Ctrl+B)"
SIDE_BAR_MARKER = "Timeline Section"


def side_bar_state(tree) -> tuple[bool, bool]:
    """(the switch reads pressed, the side bar's Timeline section exists), both from the raw tree."""
    def read():
        try:
            switches = tree.find(role=ROLE_TOGGLE, name=SIDE_BAR_SWITCH)
            return (len(switches), "PRESSED" in switches[0].states if switches else False,
                    bool(tree.find(role=ROLE_BUTTON, name=SIDE_BAR_MARKER)))
        except Exception as error:
            # Chromium replaces the nodes of its toolbar while the layout changes; a vanished object means read again.
            if "UnknownObject" in str(error) or "does not exist" in str(error):
                return None
            raise

    found = raw.poll(read, 10)
    require(found, "the raw tree kept changing under the read")
    require(found[0] == 1, f"expected one {SIDE_BAR_SWITCH!r} switch, found {found[0]}")
    return found[1], found[2]


@implementation("hyprland.code_toggle_primary_side_bar")
def code_toggle_primary_side_bar(ctx: LegContext) -> None:
    with launched(ctx, "Code", 60.0) as app:
        tree = root(ctx, app)
        require(raw.poll(lambda: tree.find(role=ROLE_TOGGLE, name=SIDE_BAR_SWITCH), 40.0),
                "VS Code never exposed the side bar switch")
        initial = side_bar_state(tree)
        require(initial[0] == initial[1], f"the switch and the side bar disagree before the leg: {initial}")
        ctx.before("atspi", initial)
        try:
            # The File menu item advertises no Click verb, so a headless click is refused before any input.
            menu = ref(ctx, app, role="menuitem", name="File")
            refusal = command(ctx, ["click", menu], app, code="POLICY_DENIED")
            checks = (refusal.error.get("details") or {}).get("checks", ())
            require(any(check.get("check") == "supported_action" and check.get("status") == "fail"
                        for check in checks), f"the refusal does not name supported_action: {refusal.raw}")
            require(refusal.error.get("disposition", {}).get("delivery") == "not_delivered", refusal.raw)
            require(side_bar_state(tree) == initial, "the refused click changed VS Code")
            target = ref(ctx, app, role="switch", name=SIDE_BAR_SWITCH)
            with undisturbed(ctx):
                reply = command(ctx, ["toggle", target], app)
            require(reply.data["disposition"]["delivery"] == "delivered_verified",
                    f"toggle was not verified: {reply.raw}")
            flipped = (not initial[0], not initial[1])
            require(raw.poll(lambda: side_bar_state(tree) == flipped, 10.0),
                    f"the side bar is {side_bar_state(tree)}, not {flipped}")
            ctx.after("atspi", side_bar_state(tree))
        finally:
            if side_bar_state(tree) != initial:
                # activate() only knows click/activate/press, which this switch's action is not named; its first action toggles it.
                require(one(tree, role=ROLE_TOGGLE, name=SIDE_BAR_SWITCH).invoke(
                    "org.a11y.atspi.Action", "DoAction", "(i)", (0,))[0], "raw side-bar toggle refused")
            ctx.restored("atspi", changed(lambda: side_bar_state(tree), initial, seconds=10))
