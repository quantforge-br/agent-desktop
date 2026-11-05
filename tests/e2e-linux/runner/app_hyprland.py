from __future__ import annotations

import json
from pathlib import Path

from oracle import hyprland, proc

FIREFOX_PREFERENCES = (
    ("accessibility.force_disabled", "0"),
    ("browser.shell.checkDefaultBrowser", "false"),
    ("browser.startup.homepage", '"about:blank"'),
    ("browser.startup.page", "0"),
    ("browser.aboutwelcome.enabled", "false"),
    ("datareporting.policy.dataSubmissionEnabled", "false"),
    ("datareporting.healthreport.uploadEnabled", "false"),
    ("toolkit.telemetry.enabled", "false"),
    ("app.update.auto", "false"),
    ("app.update.enabled", "false"),
    ("browser.tabs.warnOnClose", "false"),
    ("startup.homepage_welcome_url", '""'),
    ("startup.homepage_welcome_url.additional", '""'),
    ("network.proxy.type", "0"),
    ("extensions.autoDisableScopes", "15"),
    ("browser.newtabpage.enabled", "false"),
    ("browser.contentblocking.introCount", "99"),
    ("trailhead.firstrun.didSeeAboutWelcome", "true"),
)

CODE_FLAGS = (
    "--enable-features=UseOzonePlatform",
    "--ozone-platform=wayland",
    "--force-renderer-accessibility",
    "--disable-gpu",
    "--password-store=basic",
    "--disable-extensions",
    "--disable-workspace-trust",
    "--skip-release-notes",
    "--skip-welcome",
    "--new-window",
)


CODE_SETTINGS = {"editor.accessibilitySupport": "on"}


def _code_data(runtime_dir: Path) -> Path:
    """The private VS Code user-data dir; accessibility support on makes Monaco expose its editor textbox."""
    data = runtime_dir / "code-data"
    settings = data / "User" / "settings.json"
    settings.parent.mkdir(parents=True, exist_ok=True)
    settings.write_text(json.dumps(CODE_SETTINGS) + "\n")
    return data


def _firefox_profile(runtime_dir: Path) -> Path:
    profile = runtime_dir / "firefox-profile"
    profile.mkdir(mode=0o700, exist_ok=True)
    lines = [f'user_pref("{name}", {value});' for name, value in FIREFOX_PREFERENCES]
    (profile / "user.js").write_text("\n".join(lines) + "\n")
    return profile


def adapt(tier, app: str, command: list[str], env: dict[str, str]) -> list[str]:
    """Launch flags and variables that put an app on the nested compositor with accessibility on."""
    name = Path(command[0]).name
    if name == "code":
        data = _code_data(tier.runtime_dir)
        env["ACCESSIBILITY_ENABLED"] = "1"
        return command + list(CODE_FLAGS) + [f"--user-data-dir={data}"]
    if name == "firefox":
        profile = _firefox_profile(tier.runtime_dir)
        env.update(MOZ_ENABLE_WAYLAND="1", GNOME_ACCESSIBILITY="1", MOZ_CRASHREPORTER_DISABLE="1",
                   MOZ_DISABLE_AUTO_SAFE_MODE="1")
        return command + ["--no-remote", "--new-instance", "--profile", str(profile), "about:blank"]
    return command


def compositor_facts(tier, pid: int):
    """Identity of a process whose environment the kernel hides, taken from it being a client of the private compositor."""
    if tier.name != "hyprland" or pid not in {view.pid for view in hyprland.windows(tier)}:
        return None
    try:
        start, _, name = proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())
    except FileNotFoundError:
        return None
    return start, name, {"XDG_RUNTIME_DIR": str(tier.runtime_dir)}
