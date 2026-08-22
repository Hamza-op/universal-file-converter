pub mod context_menu;
pub mod notification;
pub mod single_instance;

/// Trait to add creation_flags on Windows Command
pub trait CommandExt {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

impl CommandExt for std::process::Command {
    #[cfg(target_os = "windows")]
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt as WinCmdExt;
        WinCmdExt::creation_flags(self, flags);
        self
    }

    #[cfg(not(target_os = "windows"))]
    fn creation_flags(&mut self, _flags: u32) -> &mut Self {
        self
    }
}
