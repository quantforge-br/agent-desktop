"""The hyprland tier's selection, session layout and oracle parsing."""
import unittest
from unittest.mock import patch

from oracle import hyprland
from runner import legs, session
from runner.main import DEFAULT_TIERS, TIER_CHOICES
from runner.session_base import session_base, session_bases

CLIENT = {
    "address": "0x1", "pid": 7, "class": "app", "title": "t", "at": [1, 2], "size": [30, 40],
    "workspace": {"id": 1, "name": "1"}, "floating": False, "fullscreen": 0, "hidden": False, "mapped": True,
}


class HyprlandTierTests(unittest.TestCase):
    def test_tier_is_selectable_but_not_a_default(self):
        self.assertIn("hyprland", TIER_CHOICES)
        self.assertNotIn("hyprland", DEFAULT_TIERS)

    def test_session_wrapper_runs_standalone_without_the_x11_session_script(self):
        argv = session._tier_argv("hyprland")
        self.assertEqual(argv[:3], ["bash", str(session.SCRIPT_HYPRLAND_SHELL), "--"])
        self.assertNotIn(str(session.SCRIPT_PRIVATE_SESSION), argv)

    def test_hyprland_base_is_short_enough_for_the_compositor_socket(self):
        base = session_base("hyprland", {"XDG_RUNTIME_DIR": "/run/user/1000/long/path/for/runtime"})
        self.assertLessEqual(len(str(base)) + len("/rt.XXXXXX/hypr/") + 62 + len("/.socket2.sock"), 107)
        self.assertEqual(str(session_base("hyprland", {"AGENT_DESKTOP_SESSION_BASE": "/tmp/x"})), "/tmp/x")

    def test_bases_are_deduplicated_per_tier_family(self):
        bases = session_bases(("x11-openbox", "x11-mutter", "hyprland"), {"XDG_RUNTIME_DIR": "/run/user/1"})
        self.assertEqual(len(bases), len(set(bases)))
        self.assertEqual(len(bases), 2)

    def test_hyprland_legs_declare_no_x11_or_gnome_oracle(self):
        # The dux-shell legs read the notification signals on the private session bus; every other
        # Hyprland leg has no D-Bus oracle, because the only notification daemon there is dux-shell.
        for leg in legs.REGISTRY:
            if "hyprland" in leg.tiers:
                forbidden = {"x11", "gsettings"} | (set() if leg.id.startswith("hyprland.dux_") else {"dbus"})
                with self.subTest(leg=leg.id):
                    self.assertFalse(forbidden & set(leg.oracles))

    def test_window_view_reads_the_compositor_record(self):
        with patch.object(hyprland, "query", return_value=[CLIENT]):
            (view,) = hyprland.windows(object())
        self.assertEqual((view.address, view.pid, view.geometry(), view.workspace), ("0x1", 7, (1, 2, 30, 40), "1"))

    def test_window_of_requires_exactly_one_mapped_window(self):
        other = dict(CLIENT, address="0x2")
        with patch.object(hyprland, "query", return_value=[CLIENT, other]):
            with self.assertRaises(hyprland.HyprlandError):
                hyprland.window_of(object(), 7)
        with patch.object(hyprland, "query", return_value=[CLIENT, dict(other, pid=8)]):
            self.assertEqual(hyprland.window_of(object(), 7).address, "0x1")


if __name__ == "__main__":
    unittest.main()
