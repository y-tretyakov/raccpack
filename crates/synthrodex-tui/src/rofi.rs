#![allow(dead_code)]

pub struct RofiLauncher {
    // stub
}

impl Default for RofiLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl RofiLauncher {
    pub fn new() -> Self {
        Self {}
    }

    /// Show rofi window with menu items. Returns selected index or None.
    pub fn show(&self, items: &[String]) -> Result<Option<usize>, String> {
        // Stub: in real impl, pipe items to rofi -dmenu
        let _ = items;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_returns_none_when_empty() {
        let launcher = RofiLauncher::new();
        let result = launcher.show(&[]).unwrap();
        assert!(result.is_none());
    }
}
