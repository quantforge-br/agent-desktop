from __future__ import annotations

import os
from pathlib import Path

HYPRLAND_BASE_PREFIX = "/tmp/adh-"


def session_base(tier_name: str | None = None, environ=None) -> Path:
    """Directory holding a tier's private runtime directories.

    Hyprland's IPC sockets sit under the runtime directory and a Unix socket path holds
    at most 107 bytes, so the Hyprland tier defaults to a base short enough for them.
    """
    environ = os.environ if environ is None else environ
    explicit = environ.get("AGENT_DESKTOP_SESSION_BASE")
    if explicit:
        return Path(explicit)
    if tier_name == "hyprland":
        return Path(f"{HYPRLAND_BASE_PREFIX}{os.getuid()}")
    if environ.get("XDG_RUNTIME_DIR"):
        return Path(environ["XDG_RUNTIME_DIR"]) / "agent-desktop-sessions"
    return Path(f"/tmp/agent-desktop-sessions-{os.getuid()}")


def session_bases(tier_names, environ=None) -> list[Path]:
    seen: list[Path] = []
    for name in tier_names:
        base = session_base(name, environ)
        if base not in seen:
            seen.append(base)
    return seen
