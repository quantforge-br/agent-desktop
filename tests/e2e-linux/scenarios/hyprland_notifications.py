"""Notification legs against the Dux Shell, the private session's notification server on the hyprland tier.

The shell's contract, which these legs hold the product to: while the notification center is closed,
`list-notifications` and `wait --notification` answer POLICY_DENIED because reading it means showing it, which
takes the keyboard focus; `--headed` allows that. The mutations (`dismiss-notification`, `notification-action`,
`dismiss-all-notifications`) are POLICY_DENIED headless in every state. Notifications are sent and observed with
gdbus and dbus-monitor on the private bus, never through the product.
"""
from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw, notify
from runner import dux
from runner.helper import delayed, nodes, require
from runner.legs import implementation
from runner.verdict import LegContext

CENTER = "Notification center"
CENTER_MARKER = "Do not disturb"
CLOSED_BY_USER = 2


@contextlib.contextmanager
def observing(ctx):
    """The shell is running, its bus signals are recorded, and every notification is closed again on the way out."""
    with dux.running(ctx.tier), notify.observing(ctx) as sender:
        yield sender


def listed(ctx, *extra, headed=True):
    envelope = ctx.product(["list-notifications", *extra], headed=headed)
    require(envelope.ok, envelope.raw)
    return envelope.data["notifications"]


def awaited(ctx, count, *extra):
    def read():
        entries = listed(ctx, *extra)
        return entries if len(entries) == count else None

    result = raw.poll(read, 10)
    require(result, f"the shell never listed {count} notifications for {extra}")
    return result


def shown(entry) -> tuple[str, str, str]:
    return entry["app_name"], entry["title"], entry["body"]


def refused(envelope, code):
    error = envelope.error or {}
    require(not envelope.ok and error.get("code") == code, envelope.raw)
    require(error.get("disposition", {}).get("delivery") == "not_delivered", envelope.raw)
    require(envelope.exit_code == 1, envelope.raw)


def untouched(sender, expected_active):
    require(not sender.signals(), f"the bus saw notification signals: {sender.signals()}")
    require(sender.active() == expected_active, "the notifications changed")


@implementation("hyprland.dux_list_notifications")
def dux_list_notifications(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        first = sender.send("HarnessFirst", "First title", "First body")
        second = sender.send("HarnessSecond", "Second title", "Second body", actions=(("open", "Open"),))
        entries = awaited(ctx, 2)
        require(sorted(shown(entry) for entry in entries) == [
            ("HarnessFirst", "First title", "First body"), ("HarnessSecond", "Second title", "Second body")],
            f"listed {entries}, sent {sender.records}")
        require(sorted(entry["index"] for entry in entries) == [1, 2], f"indices {entries}")
        by_app = {entry["app_name"]: entry for entry in entries}
        require(by_app["HarnessSecond"].get("actions") == ["Open"], f"actions of the second: {by_app}")
        require(not by_app["HarnessFirst"].get("actions"), f"actions of the first: {by_app}")
        only = listed(ctx, "--app", "NESSSEC")
        require([shown(entry) for entry in only] == [("HarnessSecond", "Second title", "Second body")],
                f"the case-insensitive app filter gave {only}")
        text = listed(ctx, "--text", "first body")
        require([shown(entry) for entry in text] == [("HarnessFirst", "First title", "First body")],
                f"the text filter gave {text}")
        require(len(listed(ctx, "--limit", "1")) == 1, "--limit 1 did not cap the list")
        require({first, second} == {record["id"] for record in sender.active()}, "listing closed something")
        require(not sender.signals(), f"listing produced bus signals: {sender.signals()}")
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_center_closed_policy_denied")
def dux_center_closed_policy_denied(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        sender.send("HarnessClosed", "Closed title", "Closed body")
        expected = sender.active()
        refused(ctx.product(["list-notifications"], headed=False), "POLICY_DENIED")
        refused(ctx.product(["wait", "--notification", "--text", "Closed body", "--timeout", "1500"],
                            headed=False), "POLICY_DENIED")
        untouched(sender, expected)
        # The control: the same notification is there, and headed is what lets the product read it.
        require([shown(entry) for entry in awaited(ctx, 1)] == [("HarnessClosed", "Closed title", "Closed body")],
                "the refused read hid a notification that exists")
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_wait_notification_passes")
def dux_wait_notification_passes(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        marker = "HarnessWaitMarker"
        trigger = delayed(1.5, lambda: sender.send("HarnessWait", marker, "Wait body"))
        try:
            envelope = ctx.product(["wait", "--notification", "--text", marker, "--timeout", "10000"])
        finally:
            trigger.join()
        require(envelope.ok and envelope.data.get("matched") is True, envelope.raw)
        entry = envelope.data["notification"]
        require((entry["app_name"], entry["title"]) == ("HarnessWait", marker), envelope.raw)
        require([(record["app"], record["summary"]) for record in sender.active()] == [("HarnessWait", marker)],
                "the bus does not show exactly the notification the wait matched")
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_wait_notification_times_out")
def dux_wait_notification_times_out(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        refused(ctx.product(["wait", "--notification", "--text", "HarnessAbsentMarker", "--timeout", "1500"]),
                "TIMEOUT")
        untouched(sender, [])
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_headless_mutations_policy_denied")
def dux_headless_mutations_policy_denied(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        sender.send("HarnessMutate", "Mutation title", "Mutation body", actions=(("open", "Open"),))
        expected = sender.active()
        awaited(ctx, 1)
        for argv in (["dismiss-notification", "1", "--expected-app", "HarnessMutate"],
                     ["notification-action", "1", "Open", "--expected-app", "HarnessMutate"],
                     ["dismiss-all-notifications"]):
            refused(ctx.product(argv, headed=False), "POLICY_DENIED")
        untouched(sender, expected)
        require(len(awaited(ctx, 1)) == 1, "a refused mutation changed the list")
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_wrong_app_not_found")
def dux_wrong_app_not_found(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        sender.send("HarnessRight", "Right title", "Right body")
        expected = sender.active()
        before = awaited(ctx, 1)
        for argv in (["dismiss-notification", "1", "--expected-app", "HarnessWrong"],
                     ["dismiss-notification", "1", "--expected-app", "HarnessRight", "--expected-title", "Wrong"]):
            refused(ctx.product(argv), "NOTIFICATION_NOT_FOUND")
        untouched(sender, expected)
        require(listed(ctx) == before, "a refused dismissal changed the list")
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_dismiss_notification_closes_only_target")
def dux_dismiss_notification_closes_only_target(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        target = sender.send("HarnessTarget", "Target title", "Target body")
        bystander = sender.send("HarnessBystander", "Bystander title", "Bystander body")
        entry = awaited(ctx, 1, "--app", "HarnessTarget")[0]
        require(shown(entry) == ("HarnessTarget", "Target title", "Target body"), str(entry))
        envelope = ctx.product(["dismiss-notification", str(entry["index"]),
                                "--expected-app", "HarnessTarget", "--expected-title", "Target title"])
        require(envelope.ok, envelope.raw)
        require(sender.wait_closed({target}) == [{"id": target, "reason": CLOSED_BY_USER}],
                f"the daemon did not report the target dismissed by the user: {sender.signals()}")
        require([(record["id"], record["app"]) for record in sender.active()] == [(bystander, "HarnessBystander")],
                f"bystander lost: {sender.signals()}")
        require(sender.signals() == [("NotificationClosed", {"id": target, "reason": CLOSED_BY_USER})],
                f"unexpected bus signals: {sender.signals()}")
        remaining = awaited(ctx, 1)
        require([shown(item) for item in remaining] == [("HarnessBystander", "Bystander title", "Bystander body")],
                f"the shell lists {remaining}")
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_notification_action_invoked")
def dux_notification_action_invoked(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        identifier = sender.send("HarnessAction", "Action title", "Action body", actions=(("open", "Open"),))
        entry = awaited(ctx, 1, "--app", "HarnessAction")[0]
        require(entry.get("actions") == ["Open"], str(entry))
        envelope = ctx.product(["notification-action", str(entry["index"]), "open",
                                "--expected-app", "HarnessAction", "--expected-title", "Action title"])
        require(envelope.ok, envelope.raw)
        require(raw.poll(lambda: ("ActionInvoked", {"id": identifier, "action": "open"}) in sender.signals(), 5),
                f"the daemon never emitted ActionInvoked for {identifier}: {sender.signals()}")
        ctx.after("dbus.notifications", sender.active())


@implementation("hyprland.dux_dismiss_all_closes_scoped_then_every_notification")
def dux_dismiss_all_closes_scoped_then_every_notification(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        scoped = {sender.send("HarnessAllA", "All A1", "A1 body"), sender.send("HarnessAllA", "All A2", "A2 body")}
        other = sender.send("HarnessAllB", "All B", "B body")
        awaited(ctx, 3)
        envelope = ctx.product(["dismiss-all-notifications", "--app", "HarnessAllA"])
        require(envelope.ok, envelope.raw)
        closed = sender.wait_closed(scoped)
        require(sorted((item["id"], item["reason"]) for item in closed)
                == [(identifier, CLOSED_BY_USER) for identifier in sorted(scoped)], str(closed))
        require([record["id"] for record in sender.active()] == [other],
                f"the app-scoped dismissal did not leave exactly the other app's notification: {sender.signals()}")
        require([shown(entry) for entry in awaited(ctx, 1)] == [("HarnessAllB", "All B", "B body")],
                "the shell lists something other than the other app's notification")
        envelope = ctx.product(["dismiss-all-notifications"])
        require(envelope.ok, envelope.raw)
        require(sender.wait_closed({other}) == [{"id": other, "reason": CLOSED_BY_USER}], str(sender.signals()))
        require(not sender.active(), str(sender.active()))
        require(not listed(ctx), "the shell still lists notifications")
        ctx.after("dbus.notifications", sender.active())


def center_frames(tier):
    """Showing frames of the shell that hold the notification center's controls, read from the raw AT-SPI tree."""
    found = []
    for application in raw.registry(tier):
        with contextlib.suppress(Exception):
            for frame in application.children():
                if "SHOWING" in frame.states and any(node.name.startswith(CENTER_MARKER) for node in frame.walk()):
                    found.append(frame)
    return found


def raw_node_of(tier, window_id: str):
    """The AT-SPI node a product window id (w-<bus digits>-<object path with dashes>) names."""
    unique, path = window_id[2:].split("-", 1)
    return raw.RawNode(raw.connect(tier.a11y_bus_address), ":" + unique, "/" + path.replace("-", "/"))


@implementation("hyprland.dux_notification_center_surface")
def dux_notification_center_surface(ctx: LegContext) -> None:
    with observing(ctx) as sender:
        sender.send("HarnessSurface", "Surface title", "Surface body")
        ctx.before("atspi", len(center_frames(ctx.tier)))
        require(not center_frames(ctx.tier), "the notification center was open before the leg")
        try:
            envelope = ctx.product(["open-system-surface", "--surface", "notification-center"])
            require(envelope.ok, envelope.raw)
            window = envelope.data["window"]
            require(window["title"] == CENTER and window["visible"], envelope.raw)
            node = raw_node_of(ctx.tier, window["id"])
            bounds = window["bounds"]
            require("SHOWING" in node.states and node.extents() == tuple(int(bounds[key]) for key in ("x", "y", "width", "height")),
                    f"the raw tree disagrees about the window: {sorted(node.states)} {node.extents()} for {window}")
            require(any(item.name.startswith(CENTER_MARKER) for item in node.walk()),
                    "the raw window does not hold the notification center's controls")
            snapshot = ctx.product(["snapshot", "--surface", "notification-center", "--window-id", window["id"], "-i"])
            require(snapshot.ok and snapshot.data["complete"] is True, snapshot.raw)
            names = {item.name for item in node.walk()}
            seen = [item["name"] for item in nodes(snapshot.data["tree"]) if item.get("name")]
            missing = [name for name in seen if name not in names]
            require(seen and not missing, f"snapshot names absent from the raw tree: {missing}")
            require(any("Surface title" in name or "Surface body" in name for name in names),
                    "the raw tree of the center does not carry the seeded notification")
            ctx.after("atspi", len(center_frames(ctx.tier)))
        finally:
            dux.ipc(ctx.tier, "notifications", "close")
            require(raw.poll(lambda: not center_frames(ctx.tier), 5), "the notification center did not close")
            ctx.restored("atspi", len(center_frames(ctx.tier)))
        ctx.after("dbus.notifications", sender.active())
