_PAYLOAD = r"""
rt=$AGENT_DESKTOP_PRIVATE_SESSION
mkdir -p "$rt/ad-home"
env | grep -E '^(XDG_RUNTIME_DIR|DISPLAY|WAYLAND_DISPLAY|DBUS_SESSION_BUS_ADDRESS|XDG_CONFIG_HOME|XDG_DATA_HOME|XDG_CACHE_HOME|TMPDIR|LD_LIBRARY_PATH|GSETTINGS_BACKEND|XDG_CURRENT_DESKTOP|AGENT_DESKTOP_PRIVATE_SESSION|AGENT_DESKTOP_HOME|HYPRLAND_INSTANCE_SIGNATURE)=' > "$AD_TIER_ENV_FILE.tmp"
mv "$AD_TIER_ENV_FILE.tmp" "$AD_TIER_ENV_FILE"
printf '%s %s\n' "$AD_RUNNER_PID" "$AD_RUNNER_START" > "$rt/harness-owner"
trap 'exit 0' TERM INT
mkfifo "$rt/release-pipe"
exec 9<>"$rt/release-pipe"
while [ ! -f "$AD_TIER_RELEASE_FILE" ]; do read -t 0.2 -u 9 || :; done
"""

_ENV_KEYS = (
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "TMPDIR",
    "LD_LIBRARY_PATH",
    "GSETTINGS_BACKEND",
    "XDG_CURRENT_DESKTOP",
    "AGENT_DESKTOP_PRIVATE_SESSION",
    "AGENT_DESKTOP_HOME",
    "DISPLAY",
    "WAYLAND_DISPLAY",
)

HYPRLAND_ENV_KEYS = ("HYPRLAND_INSTANCE_SIGNATURE",)


def hyprland_env(env_values: dict[str, str]) -> dict[str, str]:
    """Variables only the nested Hyprland tier gives its apps and products.

    The signature comes from the private session's own env file, never from the harness
    process, so a signature inherited from the owner's real session cannot reach a command.
    Labels are read in one fixed language so name matches do not depend on the owner's locale.
    """
    return {
        "HYPRLAND_INSTANCE_SIGNATURE": env_values.get("HYPRLAND_INSTANCE_SIGNATURE", ""),
        "XDG_SESSION_DESKTOP": "Hyprland",
        "LANG": "C.UTF-8",
    }
