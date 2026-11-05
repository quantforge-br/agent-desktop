from __future__ import annotations

import contextlib
import importlib.util
import os
import shutil
import signal
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

from oracle import proc as process
from runner.bounded import load_bounded_process as _load_bounded_process
from runner.session_a11y import _a11y_address, _a11y_warm
from runner.session_base import session_base as _session_base
from runner.session_environment import _PAYLOAD, _ENV_KEYS, HYPRLAND_ENV_KEYS, hyprland_env
HARNESS_ROOT = Path(__file__).resolve().parent.parent
REPO_ROOT = HARNESS_ROOT.parent.parent
SCRIPTS_DIR = REPO_ROOT / "scripts"

SCRIPT_PRIVATE_SESSION = SCRIPTS_DIR / "private-session.sh"
SCRIPT_MUTTER_SHELL = SCRIPTS_DIR / "private-mutter-shell.sh"
SCRIPT_WAYLAND_SHELL = SCRIPTS_DIR / "private-wayland-shell.sh"
SCRIPT_HYPRLAND_SHELL = SCRIPTS_DIR / "private-hyprland-shell.sh"

_INNER_WRAPPERS = {
    "x11-openbox": (),
    "x11-mutter": ("SCRIPT_MUTTER_SHELL",),
    "wayland": ("SCRIPT_WAYLAND_SHELL",),
    "hyprland": ("SCRIPT_HYPRLAND_SHELL",),
}
_STANDALONE_TIERS = frozenset({"hyprland"})
WAYLAND_TIERS = frozenset({"wayland", "hyprland"})

ENV_FILE_VAR = "AD_TIER_ENV_FILE"
RELEASE_FILE_VAR = "AD_TIER_RELEASE_FILE"
RUNNER_PID_VAR = "AD_RUNNER_PID"
RUNNER_START_VAR = "AD_RUNNER_START"
OWNER_MARKER = "harness-owner"


@dataclass
class Tier:
    name: str
    runtime_dir: Path
    display: str | None
    wayland_display: str | None
    bus_address: str
    a11y_bus_address: str
    state_root: Path
    keyfile: Path
    env: dict[str, str]


class TierUnavailable(Exception):
    pass


class TeardownError(Exception):
    pass


def _tier_argv(name: str) -> list[str]:
    if name not in _INNER_WRAPPERS:
        raise ValueError(f"unknown tier {name}")
    if name in _STANDALONE_TIERS:
        script = globals()[_INNER_WRAPPERS[name][0]]
        return ["bash", str(script), "--", "bash", "-c", _PAYLOAD]
    argv = ["bash", str(SCRIPT_PRIVATE_SESSION)]
    if name == "x11-openbox":
        argv.append("--wm")
    argv.append("--")
    for attr in _INNER_WRAPPERS[name]:
        argv += [str(globals()[attr]), "--"]
    argv += ["bash", "-c", _PAYLOAD]
    return argv


def _parse_env_file(text: str) -> dict[str, str]:
    env = {}
    for line in text.splitlines():
        key, sep, value = line.partition("=")
        if sep and key in _ENV_KEYS + HYPRLAND_ENV_KEYS:
            env[key] = value
    return env


def _proc_members(runtime_dir: Path, proc_root: Path = Path("/proc")) -> list[tuple[int, int]]:
    members = []
    for directory in proc_root.glob("[0-9]*"):
        try:
            pid = int(directory.name)
            start, _, _, env = process.facts(pid, proc_root)
        except (OSError, ValueError, IndexError):
            continue
        if env.get("XDG_RUNTIME_DIR") == str(runtime_dir):
            members.append((pid, start))
    return members


def _process_start_time(pid_dir: Path) -> int | None:
    try:
        stat = (pid_dir / "stat").read_text()
    except OSError:
        return None
    fields = stat[stat.rfind(")") + 2 :].split()
    try:
        return int(fields[19])
    except (IndexError, ValueError):
        return None


def _kill(pid: int, sig: int, start: int, runtime_dir: Path, proc_root: Path) -> None:
    try:
        current, _, _, env = process.facts(pid, proc_root)
        if current == start and env.get("XDG_RUNTIME_DIR") == str(runtime_dir):
            os.kill(pid, sig)
    except (OSError, ValueError, IndexError):
        pass


def _sweep_members(runtime_dir: Path, proc_root: Path = Path("/proc")) -> list[tuple[int, int]]:
    for pid, start in _proc_members(runtime_dir, proc_root):
        _kill(pid, signal.SIGTERM, start, runtime_dir, proc_root)
    time.sleep(1.0)
    for pid, start in _proc_members(runtime_dir, proc_root):
        _kill(pid, signal.SIGKILL, start, runtime_dir, proc_root)
    deadline = time.monotonic() + 5.0
    while time.monotonic() < deadline:
        remaining = _proc_members(runtime_dir, proc_root)
        if not remaining:
            return []
        time.sleep(0.1)
    return _proc_members(runtime_dir, proc_root)


def _remove_tree(path: Path) -> bool:
    if not path.exists():
        return True
    try:
        shutil.rmtree(path)
    except OSError:
        return False
    return not path.exists()


def _own_start_time() -> int:
    return _process_start_time(Path("/proc") / "self") or 0


def _signal_process(pid: int, sig: int) -> None:
    try:
        os.kill(pid, sig)
    except (ProcessLookupError, PermissionError, OSError):
        pass


def _build_tier(name: str, env_values: dict[str, str]) -> Tier:
    runtime_dir = Path(env_values["AGENT_DESKTOP_PRIVATE_SESSION"])
    state_root = Path(env_values["AGENT_DESKTOP_HOME"])
    state_root.chmod(0o700)
    env = {
        "HOME": os.environ.get("HOME", str(Path.home())),
        "PATH": os.environ.get("PATH", os.defpath),
        "LANG": os.environ.get("LANG", "C.UTF-8"),
    }
    for key in _ENV_KEYS:
        if env_values.get(key):
            env[key] = env_values[key]
    libraries = env.get("LD_LIBRARY_PATH", os.environ.get("LD_LIBRARY_PATH", "")).split(os.pathsep)
    libraries = list(dict.fromkeys(path for path in libraries if path))
    if libraries:
        env["LD_LIBRARY_PATH"] = os.pathsep.join(libraries)
    if name in WAYLAND_TIERS:
        env.pop("DISPLAY", None)
        env.update(GDK_BACKEND="wayland", XDG_SESSION_TYPE="wayland")
    if name == "hyprland":
        env.update(hyprland_env(env_values))
    return Tier(
        name=name,
        runtime_dir=runtime_dir,
        display=None if name in WAYLAND_TIERS else env_values.get("DISPLAY") or None,
        wayland_display=env_values.get("WAYLAND_DISPLAY") or None,
        bus_address=env_values["DBUS_SESSION_BUS_ADDRESS"],
        a11y_bus_address=env_values.get("AT_SPI_BUS_ADDRESS", ""),
        state_root=state_root,
        keyfile=runtime_dir / "cfg" / "glib-2.0" / "settings" / "keyfile",
        env=env,
    )


def sweep_strays(session_base: Path, proc_root: Path = Path("/proc")) -> list[Path]:
    swept = []
    for marker in sorted(session_base.glob("rt.*/" + OWNER_MARKER)):
        runtime_dir = marker.parent
        try:
            runner_pid, runner_start = marker.read_text().split()[:2]
        except (OSError, ValueError):
            continue
        pid_dir = proc_root / runner_pid
        if pid_dir.exists() and _process_start_time(pid_dir) == int(runner_start):
            continue
        _sweep_members(runtime_dir, proc_root)
        _remove_tree(runtime_dir)
        swept.append(runtime_dir)
    return swept


def _wait_for_health(
    proc: subprocess.Popen, name: str, env_file: Path, deadline: float
) -> dict[str, str]:
    while time.monotonic() < deadline:
        if env_file.exists():
            values = _parse_env_file(env_file.read_text())
            if values.get("XDG_RUNTIME_DIR") and values.get("DBUS_SESSION_BUS_ADDRESS"):
                return values
        rc = proc.poll()
        if rc is not None:
            raise TierUnavailable(
                f"{name}: the session wrapper exited {rc}: {_stderr_tail(proc)}"
            )
        time.sleep(0.2)
    _signal_process(proc.pid, signal.SIGTERM)
    raise TierUnavailable(f"{name}: the health gate did not pass in {int(deadline)}s")


def _stderr_tail(proc: subprocess.Popen) -> str:
    if proc.stderr is None:
        return ""
    try:
        os.set_blocking(proc.stderr.fileno(), False)
        data = proc.stderr.read() or b""
    except (OSError, ValueError):
        return ""
    return data[-800:].decode("utf-8", "replace").strip()


def _env_runtime_dir(env_file: Path, base: Path, existing: set[Path]) -> Path | None:
    owner = f"{os.getpid()} {_own_start_time()}"
    for runtime in set(base.glob("rt.*")) - existing:
        try:
            if (runtime / OWNER_MARKER).read_text().strip() == owner:
                return runtime
        except OSError:
            continue
    if not env_file.exists():
        return None
    value = _parse_env_file(env_file.read_text()).get("AGENT_DESKTOP_PRIVATE_SESSION", "")
    return Path(value) if value else None


def _teardown(
    proc: subprocess.Popen,
    runtime_dir: Path,
    release_file: Path,
    proc_root: Path = Path("/proc"),
) -> None:
    errors = []
    try:
        release_file.touch()
    except OSError as error:
        errors.append(f"release marker could not be written: {error}")
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        _signal_process(proc.pid, signal.SIGTERM)
        try:
            proc.wait(timeout=4)
        except subprocess.TimeoutExpired:
            remaining = _sweep_members(runtime_dir, proc_root)
            if remaining:
                errors.append(f"members survived before wrapper escalation: {remaining}")
            _signal_process(proc.pid, signal.SIGKILL)
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                errors.append("the session wrapper could not be reaped")
    remaining = _sweep_members(runtime_dir, proc_root)
    if remaining:
        errors.append(f"members survived the sweep: {remaining}")
    if runtime_dir.exists() and not _remove_tree(runtime_dir):
        errors.append(f"the runtime directory could not be removed: {runtime_dir}")
    if proc.stderr is not None:
        proc.stderr.close()
    if errors:
        raise TeardownError("; ".join(errors))


@contextlib.contextmanager
def start_tier(name: str, deadline: float):
    argv = _tier_argv(name)
    work_dir = Path(os.environ.get("AGENT_DESKTOP_E2E_SUITE") or tempfile.mkdtemp(prefix="ad-tier-"))
    work_dir.mkdir(parents=True, exist_ok=True)
    env_file = work_dir / f"{name}.env"
    release_file = work_dir / f"{name}.release"
    for marker in (env_file, Path(f"{env_file}.tmp"), release_file):
        marker.unlink(missing_ok=True)
    base = _session_base(name)
    existing = set(base.glob("rt.*"))
    child_env = dict(os.environ)
    child_env.update({
        ENV_FILE_VAR: str(env_file),
        RELEASE_FILE_VAR: str(release_file),
        RUNNER_PID_VAR: str(os.getpid()),
        RUNNER_START_VAR: str(_own_start_time()),
    })
    proc = subprocess.Popen(
        argv,
        env=child_env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        start_new_session=True,
        preexec_fn=lambda: signal.signal(signal.SIGINT, signal.SIG_DFL),
    )
    monotonic_deadline = time.monotonic() + deadline
    tier = None
    try:
        values = _wait_for_health(proc, name, env_file, monotonic_deadline)
        tier = _build_tier(name, values)
        address = _a11y_address(tier.env, monotonic_deadline)
        if address is None:
            raise TierUnavailable(f"{name}: the accessibility bus did not answer")
        tier.a11y_bus_address = address
        tier.env["AT_SPI_BUS_ADDRESS"] = address
        if not _a11y_warm(tier.env, monotonic_deadline):
            raise TierUnavailable(f"{name}: the accessibility registry did not come up")
        from runner import cancel
        cancel.publish(work_dir, tier)
        yield tier
    finally:
        if proc.poll() is None:
            _signal_process(proc.pid, signal.SIGTERM)
        runtime_dir = tier.runtime_dir if tier is not None else _env_runtime_dir(env_file, base, existing)
        if runtime_dir is not None:
            _teardown(proc, runtime_dir, release_file)
        else:
            if proc.poll() is None:
                _signal_process(proc.pid, signal.SIGKILL)
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
            if proc.stderr is not None:
                proc.stderr.close()
