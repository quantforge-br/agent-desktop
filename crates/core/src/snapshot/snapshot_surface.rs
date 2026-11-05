/// The error-detail `kind` an adapter gives `ELEMENT_NOT_FOUND` when the
/// requested surface, such as a menu, is not open: not yet for a selector
/// wait that it appear, and over for one that it go.
pub const SURFACE_ABSENT: &str = "surface_absent";

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotSurface {
    #[default]
    Window,
    Focused,
    Menu,
    Menubar,
    Sheet,
    Popover,
    Alert,
    Desktop,
    Taskbar,
    #[serde(alias = "system-tray")]
    SystemTray,
    #[serde(alias = "quick-settings")]
    QuickSettings,
    #[serde(alias = "notification-center")]
    NotificationCenter,
    Toolbar,
    Dock,
    Spotlight,
    #[serde(alias = "menu-bar-extras")]
    MenuBarExtras,
    #[serde(alias = "system-tray-overflow")]
    SystemTrayOverflow,
    #[serde(alias = "start-menu")]
    StartMenu,
    #[serde(alias = "action-center")]
    ActionCenter,
}

impl SnapshotSurface {
    /// The refusal for a surface an adapter does not root: a wrong argument
    /// when the platform roots other kinds, a platform limit when it roots none.
    pub fn unavailable_error(self, supported: &[Self]) -> crate::AdapterError {
        crate::AdapterError::new(
            if supported.is_empty() {
                crate::ErrorCode::PlatformNotSupported
            } else {
                crate::ErrorCode::InvalidArgs
            },
            format!("Snapshot surface '{}' is not available on this platform", self.as_str()),
        )
        .with_details(serde_json::json!({
            "requested_surface": self.as_str(),
            "supported_surfaces": supported.iter().map(|surface| surface.as_str()).collect::<Vec<_>>()
        }))
        .with_suggestion("Choose one of the supported snapshot surfaces")
        .with_disposition(crate::DeliverySemantics::not_delivered())
    }

    pub fn is_window(surface: &Self) -> bool {
        matches!(surface, Self::Window)
    }

    /// Whether the kind names OS chrome rather than a window's own sub-surface:
    /// the kinds an adapter may resolve directly, with no owning application and
    /// no window in the inventory. Derived from the variant alone - never from a
    /// platform check - so the seam stays platform-neutral.
    pub(crate) fn is_shell_chrome(self) -> bool {
        matches!(
            self,
            Self::StartMenu
                | Self::Taskbar
                | Self::SystemTray
                | Self::SystemTrayOverflow
                | Self::ActionCenter
                | Self::QuickSettings
                | Self::Dock
                | Self::MenuBarExtras
        )
    }

    /// Whether a shell may present the kind as its own surface where other
    /// platforms read it from an application, as dux-shell does with its
    /// notification center and launcher.
    pub(crate) fn may_be_shell_presented(self) -> bool {
        matches!(self, Self::NotificationCenter | Self::Spotlight)
    }

    /// Surfaces a background app with no window still presents, read from its
    /// process; the owner then carries an empty window id.
    pub(crate) fn is_process_surface(self) -> bool {
        matches!(self, Self::Window | Self::Menubar | Self::MenuBarExtras)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::Focused => "focused",
            Self::Menu => "menu",
            Self::Menubar => "menubar",
            Self::Sheet => "sheet",
            Self::Popover => "popover",
            Self::Alert => "alert",
            Self::Desktop => "desktop",
            Self::Taskbar => "taskbar",
            Self::SystemTray => "system-tray",
            Self::QuickSettings => "quick-settings",
            Self::NotificationCenter => "notification-center",
            Self::Toolbar => "toolbar",
            Self::Dock => "dock",
            Self::Spotlight => "spotlight",
            Self::MenuBarExtras => "menu-bar-extras",
            Self::SystemTrayOverflow => "system-tray-overflow",
            Self::StartMenu => "start-menu",
            Self::ActionCenter => "action-center",
        }
    }
}
