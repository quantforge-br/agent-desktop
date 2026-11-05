"""The Dux Shell (a DankMaterialShell fork on quickshell) as the private session's notification server.

The shell is started per leg and removed again, so the owned-process set a leg is judged on is the same
before and after it. Its state lives in a throwaway home under the private runtime dir.
"""
from __future__ import annotations

import contextlib
import os
import shutil
import signal
import subprocess
import time
from pathlib import Path

from oracle import atspi_raw as raw, proc

NOTIFICATIONS = "org.freedesktop.Notifications"
START_TIMEOUT_S = 60.0
STOP_TIMEOUT_S = 8.0


def _environment(tier, home: Path) -> dict[str, str]:
    env = dict(tier.env)
    for key in ("QT_QPA_PLATFORM", "HYPRLAND_CMD", "AT_SPI_BUS_ADDRESS"):
        env.pop(key, None)
    # tier.env names the host compositor; the shell belongs on the nested Hyprland.
    env.update(WAYLAND_DISPLAY=tier.wayland_display, HOME=str(home), XDG_CONFIG_HOME=str(home / "config"), XDG_DATA_HOME=str(home / "data"),
               XDG_CACHE_HOME=str(home / "cache"), XDG_STATE_HOME=str(home / "state"),
               QT_LINUX_ACCESSIBILITY_ALWAYS_ON="1",
               # dms defaults this to gtk3 when unset, and that theme loads GTK's own AT-SPI bridge: a second registry application.
               QT_QPA_PLATFORMTHEME="qt6ct", QS_DISABLE_CRASH_HANDLER="1")
    return env


def _owner(connection) -> str | None:
    try:
        return raw.call(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus",
                        "GetNameOwner", "(s)", (NOTIFICATIONS,))[0]
    except Exception as error:
        if "NameHasNoOwner" in str(error):
            return None
        raise


def _registered(tier, baseline: set) -> bool:
    """Whether a process the shell started is an application of the private AT-SPI registry."""
    new = {pid for pid, _, _ in _members(tier) - baseline}
    for application in raw.registry(tier):
        try:
            if raw.owner_pid(application) in new and application.children():
                return True
        except Exception:
            continue
    return False


def _members(tier) -> set[tuple[int, int, str]]:
    return set(proc.members(tier.runtime_dir))


def _signal_group(group: int, signum: int) -> None:
    try:
        os.killpg(group, signum)
    except ProcessLookupError:
        pass


def _stop(tier, child: subprocess.Popen, baseline: set, state: Path) -> None:
    for signum in (signal.SIGTERM, signal.SIGKILL):
        _signal_group(child.pid, signum)
        for pid, _, _ in _members(tier) - baseline:
            with contextlib.suppress(ProcessLookupError):
                os.kill(pid, signum)
        deadline = time.monotonic() + STOP_TIMEOUT_S
        while time.monotonic() < deadline and _members(tier) - baseline:
            time.sleep(0.1)
        if not _members(tier) - baseline:
            break
    child.wait(timeout=STOP_TIMEOUT_S)
    # quickshell extracts its QML read-only below the runtime dir, which rmtree cannot remove as it is.
    shell_cache = tier.runtime_dir / "dms-shell"
    for path in (shell_cache, state):
        for entry in [path, *path.rglob("*")] if path.exists() else []:
            with contextlib.suppress(OSError):
                entry.chmod(0o700 if entry.is_dir() else 0o600)
        shutil.rmtree(path, ignore_errors=True)
    left = _members(tier) - baseline
    if left:
        raise RuntimeError(f"dux shell left processes behind: {sorted(left)}")


@contextlib.contextmanager
def running(tier):
    """Runs `dms run` against the private session until the notification server answers on its bus."""
    if not shutil.which("dms"):
        raise RuntimeError("dms is required for the dux-shell legs")
    connection = raw.connect(tier.bus_address)
    if _owner(connection):
        raise RuntimeError(f"{NOTIFICATIONS} is already owned in the private session")
    baseline = _members(tier)
    home = tier.runtime_dir / "dux-home"
    for sub in ("config", "data", "cache", "state"):
        (home / sub).mkdir(parents=True, exist_ok=True)
    log = tier.runtime_dir / "dux.log"
    with log.open("ab") as stream:
        child = subprocess.Popen(["dms", "run", "--log-file", str(log)], env=_environment(tier, home),
                                 stdin=subprocess.DEVNULL, stdout=stream, stderr=stream, start_new_session=True)
    try:
        if not raw.poll(lambda: child.poll() is None and _owner(connection), START_TIMEOUT_S):
            raise RuntimeError(f"dux shell never took {NOTIFICATIONS}; exit={child.poll()}; log={log}")
        if not raw.poll(lambda: _registered(tier, baseline), START_TIMEOUT_S):
            raise RuntimeError(f"dux shell never registered with the AT-SPI registry; log={log}")
        yield child
    finally:
        _stop(tier, child, baseline, home)


def ipc(tier, target: str, function: str) -> str:
    """`dms ipc call` against the shell of the private session: the shell's own control, used to put its state back."""
    home = tier.runtime_dir / "dux-home"
    process = subprocess.run(["dms", "ipc", "call", target, function], env=_environment(tier, home),
                             capture_output=True, text=True, timeout=15, check=False)
    if process.returncode != 0:
        raise RuntimeError(f"dms ipc call {target} {function} failed: {process.stderr.strip()}")
    return process.stdout.strip()
