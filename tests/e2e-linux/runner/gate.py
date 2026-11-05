from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

from runner import session
from runner.session_base import session_bases

BASE_TOOLS = (
    "dbus-daemon",
    "dbus-update-activation-environment",
    "gdbus",
    "busctl",
    "gsettings",
    "setsid",
    "fusermount",
    "timeout",
)

X11_TOOLS = ("Xvfb", "xprop", "xwininfo", "xwd")
X11_FREE_TIERS = ("hyprland",)
REGISTRYD_PATHS = (
    Path("/usr/lib/at-spi2-registryd"),
    Path("/usr/libexec/at-spi2-registryd"),
    Path("/usr/lib/at-spi2-core/at-spi2-registryd"),
)

TIER_TOOLS = {
    "x11-openbox": ("xdotool", "wmctrl", "xclip"),
    "x11-mutter": ("gnome-shell", "xdotool", "wmctrl", "xclip"),
    "wayland": ("gnome-shell", "wl-copy", "wl-paste"),
    "hyprland": ("Hyprland", "hyprctl", "kwin_wayland", "wl-copy", "wl-paste",
                 "wtype", "gnome-calculator", "code", "firefox"),
}

TIER_SCRIPTS = {
    "x11-openbox": (session.SCRIPT_PRIVATE_SESSION,),
    "x11-mutter": (session.SCRIPT_PRIVATE_SESSION, session.SCRIPT_MUTTER_SHELL),
    "wayland": (session.SCRIPT_PRIVATE_SESSION, session.SCRIPT_WAYLAND_SHELL),
    "hyprland": (session.SCRIPT_HYPRLAND_SHELL,),
}


def _default_gi_probe() -> str | None:
    bounded = session._load_bounded_process()
    result = bounded.run_bounded(
        ["python3", "-c", "import gi"], timeout_seconds=15
    )
    if result.returncode != 0:
        return "python3 cannot import gi (PyGObject)"
    return None


def _openbox_path(env: dict[str, str]) -> Path:
    root = env.get("AGENT_DESKTOP_WM_ROOT") or "/"
    return Path(root) / "usr" / "bin" / "openbox"


def _tool_found(tool: str, extra_paths: tuple[Path, ...], which) -> bool:
    if which(tool) is not None:
        return True
    return any((extra / tool).is_file() and os.access(extra / tool, os.X_OK) for extra in extra_paths)


def check_prerequisites(
    tiers: tuple[str, ...] | None = None,
    *,
    which=shutil.which,
    env=None,
    extra_paths: tuple[Path, ...] | None = None,
    gi_probe=None,
    platform: str | None = None,
) -> list[str]:
    env = dict(os.environ if env is None else env)
    platform = sys.platform if platform is None else platform
    extra_paths = () if extra_paths is None else extra_paths
    gi_probe = _default_gi_probe if gi_probe is None else gi_probe
    selected = tuple(tiers) if tiers else tuple(session._INNER_WRAPPERS)
    missing = []
    if not platform.startswith("linux"):
        return ["a linux host is required for the live harness"]
    for tool in BASE_TOOLS:
        if not _tool_found(tool, extra_paths, which):
            missing.append(f"missing tool: {tool}")
    if any(tier not in X11_FREE_TIERS for tier in selected):
        for tool in X11_TOOLS:
            if not _tool_found(tool, extra_paths, which):
                missing.append(f"missing tool: {tool}")
    for tier in selected:
        for tool in TIER_TOOLS[tier]:
            if not _tool_found(tool, extra_paths, which):
                missing.append(f"missing tool for tier {tier}: {tool}")
        for script in TIER_SCRIPTS[tier]:
            if not script.is_file():
                missing.append(f"missing session script for tier {tier}: {script.name}")
    if "x11-openbox" in selected and not _openbox_path(env).is_file():
        missing.append("missing openbox: set AGENT_DESKTOP_WM_ROOT")
    if "hyprland" in selected and not any(path.is_file() for path in REGISTRYD_PATHS):
        missing.append("missing tool for tier hyprland: at-spi2-registryd")
    reason = gi_probe()
    if reason:
        missing.append(reason)
    binary = env.get("AGENT_DESKTOP_E2E_BIN_SRC") or env.get("AGENT_DESKTOP_E2E_BIN") or str(
        session.HARNESS_ROOT.parent.parent / "target" / "release" / "agent-desktop"
    )
    if not Path(binary).is_file():
        missing.append(f"missing release binary: {binary}")
    for base in session_bases(selected, env):
        try:
            base.mkdir(mode=0o700, parents=True, exist_ok=True)
        except OSError as error:
            missing.append(f"the session base is not writable: {error}")
    return missing
