from __future__ import annotations

import os
import time

from runner.bounded import load_bounded_process


def _a11y_address(env: dict[str, str], deadline: float) -> str | None:
    bounded = load_bounded_process()
    call_env = {
        "PATH": env.get("PATH", os.defpath),
        "DBUS_SESSION_BUS_ADDRESS": env["DBUS_SESSION_BUS_ADDRESS"],
    }
    while time.monotonic() < deadline:
        result = bounded.run_bounded(
            ["gdbus", "call", "--session", "-d", "org.a11y.Bus", "-o", "/org/a11y/bus",
             "-m", "org.a11y.Bus.GetAddress"],
            timeout_seconds=15,
            env=call_env,
        )
        if result.returncode == 0:
            marker = result.stdout.find("'")
            end = result.stdout.find("'", marker + 1)
            if marker != -1 and end > marker:
                return result.stdout[marker + 1 : end]
        time.sleep(0.5)
    return None


def _a11y_warm(env: dict[str, str], deadline: float) -> bool:
    bounded = load_bounded_process()
    call_env = {"PATH": env.get("PATH", os.defpath)}
    while time.monotonic() < deadline:
        result = bounded.run_bounded(
            ["gdbus", "call", "--address", env["AT_SPI_BUS_ADDRESS"],
             "-d", "org.a11y.atspi.Registry", "-o", "/org/a11y/atspi/registry",
             "-m", "org.a11y.atspi.Registry.GetRegisteredEvents"],
            timeout_seconds=15,
            env=call_env,
        )
        if result.returncode == 0:
            return True
        time.sleep(0.5)
    return False
