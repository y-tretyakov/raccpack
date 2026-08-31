use std::path::PathBuf;

use crossterm::event::KeyCode;
use raccpack_core::app::PackResult;

/// Options editable from the flow's Preview; mirrored to the worker on run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFlowOptions {
    pub deny_content_secrets: bool,
    pub zstd_level: u32,
    /// Optional custom archive name (without `.tar.zst`); `None` → auto
    /// `{slug}__{ts}.tar.zst`. Mirrors core [`PackOptions::output_name`].
    pub output_name: Option<String>,
}

impl Default for PackFlowOptions {
    fn default() -> Self {
        Self {
            deny_content_secrets: true,
            zstd_level: 3,
            output_name: None,
        }
    }
}

impl PackFlowOptions {
    pub fn toggle_deny_content(&mut self) {
        self.deny_content_secrets = !self.deny_content_secrets;
    }

    pub fn cycle_zstd_level(&mut self) {
        self.zstd_level = match self.zstd_level {
            3 => 5,
            5 => 10,
            10 => 19,
            _ => 3,
        };
    }

    /// Set the custom archive name (`None` restores auto naming). Inline
    /// validation is left to the core; the TUI only stores the value.
    pub fn set_output_name(&mut self, name: Option<String>) {
        self.output_name = name;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum PackFlowPhase {
    Preparing,
    Preview(PackResult),
    Running,
    Done(PackResult),
    Failed(String),
}

impl std::fmt::Debug for PackFlowPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preparing => f.write_str("Preparing"),
            Self::Preview(_) => f.write_str("Preview(..)"),
            Self::Running => f.write_str("Running"),
            Self::Done(_) => f.write_str("Done(..)"),
            Self::Failed(message) => f.debug_tuple("Failed").field(message).finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum PackCommand {
    PreviewConfirm,
    PreviewCancel,
    Run,
    Close,
}

impl std::fmt::Debug for PackCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreviewConfirm => f.write_str("PreviewConfirm"),
            Self::PreviewCancel => f.write_str("PreviewCancel"),
            Self::Run => f.write_str("Run"),
            Self::Close => f.write_str("Close"),
        }
    }
}

pub struct PackFlow {
    pub project: PathBuf,
    pub den_dir: PathBuf,
    pub options: PackFlowOptions,
    pub phase: PackFlowPhase,
    pub percent: u8,
    pub message: String,
    /// Whether the preview is collecting an `output_name` inline; when true
    /// printable keys go to `output_name_buffer` instead of the option toggles.
    pub editing_output_name: bool,
    /// Text edited while `editing_output_name` is active.
    pub output_name_buffer: String,
}

impl std::fmt::Debug for PackFlow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackFlow")
            .field("project", &self.project)
            .field("den_dir", &self.den_dir)
            .field("options", &self.options)
            .field("phase", &self.phase)
            .field("percent", &self.percent)
            .field("message", &self.message)
            .field("editing_output_name", &self.editing_output_name)
            .field("output_name_buffer", &self.output_name_buffer)
            .finish()
    }
}

impl PackFlow {
    pub fn new(project: PathBuf, den_dir: PathBuf, options: PackFlowOptions) -> Self {
        Self {
            project,
            den_dir,
            options,
            phase: PackFlowPhase::Preparing,
            percent: 0,
            message: String::new(),
            editing_output_name: false,
            output_name_buffer: String::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<PackCommand> {
        match &mut self.phase {
            PackFlowPhase::Preparing | PackFlowPhase::Preview(_) => {
                if self.editing_output_name {
                    self.handle_output_name_key(key)
                } else {
                    self.handle_preview_key(key)
                }
            }
            PackFlowPhase::Running => None,
            PackFlowPhase::Done(_) | PackFlowPhase::Failed(_) => match key {
                KeyCode::Enter | KeyCode::Esc => Some(PackCommand::Close),
                _ => None,
            },
        }
    }

    /// Preview keys when not editing the output name: confirm/cancel plus the
    /// option toggles (`c` content-deny, `z` zstd-level, `o` output-name).
    fn handle_preview_key(&mut self, key: KeyCode) -> Option<PackCommand> {
        match key {
            KeyCode::Char('y') | KeyCode::Enter => Some(PackCommand::PreviewConfirm),
            KeyCode::Char('n') | KeyCode::Esc => Some(PackCommand::PreviewCancel),
            KeyCode::Char('c') => {
                self.options.toggle_deny_content();
                None
            }
            KeyCode::Char('z') => {
                self.options.cycle_zstd_level();
                None
            }
            KeyCode::Char('o') => {
                self.start_output_name_input();
                None
            }
            _ => None,
        }
    }

    /// Keys while collecting an inline `output_name`. Enter commits (empty
    /// clears to `None`), Esc cancels the edit and restores the old value.
    fn handle_output_name_key(&mut self, key: KeyCode) -> Option<PackCommand> {
        match key {
            KeyCode::Enter => {
                self.commit_output_name();
                None
            }
            KeyCode::Esc => {
                self.cancel_output_name();
                None
            }
            KeyCode::Backspace => {
                self.output_name_buffer.pop();
                None
            }
            KeyCode::Char(c) if !c.is_control() => {
                self.output_name_buffer.push(c);
                None
            }
            _ => None,
        }
    }

    /// Enter the output-name editor, seeding the buffer with the current value
    /// so Backspace can edit an already-set name.
    fn start_output_name_input(&mut self) {
        self.output_name_buffer = self.options.output_name.clone().unwrap_or_default();
        self.editing_output_name = true;
    }

    /// Commit the edited buffer into [`PackFlowOptions::output_name`]; an
    /// empty buffer restores auto naming. Inline validation happens in core.
    fn commit_output_name(&mut self) {
        let name = self.output_name_buffer.trim();
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        self.options.set_output_name(name);
        self.editing_output_name = false;
        self.output_name_buffer.clear();
    }

    /// Abandon the edit, restoring the previously committed name.
    fn cancel_output_name(&mut self) {
        self.editing_output_name = false;
        self.output_name_buffer.clear();
    }

    pub fn on_progress(&mut self, percent: u8, message: &str) {
        self.percent = percent;
        self.message = message.to_string();
    }

    pub fn start_running(&mut self) {
        self.phase = PackFlowPhase::Running;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options() {
        let opts = PackFlowOptions::default();
        assert!(opts.deny_content_secrets);
        assert_eq!(opts.zstd_level, 3);
        assert_eq!(opts.output_name, None);
    }

    #[test]
    fn set_output_name_round_trip() {
        let mut opts = PackFlowOptions::default();
        assert_eq!(opts.output_name, None);
        opts.set_output_name(Some("my-artifact".to_string()));
        assert_eq!(opts.output_name.as_deref(), Some("my-artifact"));
        opts.set_output_name(None);
        assert_eq!(opts.output_name, None);
    }

    #[test]
    fn cycle_zstd_level_round_trip() {
        let mut opts = PackFlowOptions::default();
        assert_eq!(opts.zstd_level, 3);
        opts.cycle_zstd_level();
        assert_eq!(opts.zstd_level, 5);
        opts.cycle_zstd_level();
        assert_eq!(opts.zstd_level, 10);
        opts.cycle_zstd_level();
        assert_eq!(opts.zstd_level, 19);
        opts.cycle_zstd_level();
        assert_eq!(opts.zstd_level, 3);
    }

    #[test]
    fn toggle_deny_content() {
        let mut opts = PackFlowOptions::default();
        assert!(opts.deny_content_secrets);
        opts.toggle_deny_content();
        assert!(!opts.deny_content_secrets);
        opts.toggle_deny_content();
        assert!(opts.deny_content_secrets);
    }

    #[test]
    fn preview_keys() {
        let mut flow = PackFlow::new(
            PathBuf::from("/proj"),
            PathBuf::from("/den"),
            PackFlowOptions::default(),
        );
        flow.phase = PackFlowPhase::Preview(PackResult {
            source: PathBuf::from("/proj"),
            output: PathBuf::from("/den/packs/x.tar.zst"),
            size_bytes: 0,
            file_count: 0,
            skipped_secret_files: 0,
            dry_run: true,
        });

        assert_eq!(
            flow.handle_key(KeyCode::Char('y')),
            Some(PackCommand::PreviewConfirm)
        );
        assert_eq!(
            flow.handle_key(KeyCode::Enter),
            Some(PackCommand::PreviewConfirm)
        );
        assert_eq!(
            flow.handle_key(KeyCode::Char('n')),
            Some(PackCommand::PreviewCancel)
        );
        assert_eq!(
            flow.handle_key(KeyCode::Esc),
            Some(PackCommand::PreviewCancel)
        );
        // toggle keys
        assert_eq!(flow.handle_key(KeyCode::Char('c')), None);
        assert!(!flow.options.deny_content_secrets);
        assert_eq!(flow.handle_key(KeyCode::Char('z')), None);
        assert_eq!(flow.options.zstd_level, 5);
        // 'o' enters the output-name editor (no command dispatched)
        flow.phase = PackFlowPhase::Preview(PackResult {
            source: PathBuf::from("/proj"),
            output: PathBuf::from("/den/packs/x.tar.zst"),
            size_bytes: 0,
            file_count: 0,
            skipped_secret_files: 0,
            dry_run: true,
        });
        assert_eq!(flow.handle_key(KeyCode::Char('o')), None);
        assert!(flow.editing_output_name);
    }

    #[test]
    fn output_name_editor_round_trip() {
        let mut flow = PackFlow::new(
            PathBuf::from("/proj"),
            PathBuf::from("/den"),
            PackFlowOptions::default(),
        );
        flow.phase = PackFlowPhase::Preview(PackResult {
            source: PathBuf::from("/proj"),
            output: PathBuf::from("/den/packs/x.tar.zst"),
            size_bytes: 0,
            file_count: 0,
            skipped_secret_files: 0,
            dry_run: true,
        });

        // Open the editor and type a name.
        flow.handle_key(KeyCode::Char('o'));
        assert!(flow.editing_output_name);
        for c in "my-artifact".chars() {
            flow.handle_key(KeyCode::Char(c));
        }
        assert_eq!(flow.output_name_buffer, "my-artifact");

        // Commit with Enter propagates to options.
        assert_eq!(flow.handle_key(KeyCode::Enter), None);
        assert!(!flow.editing_output_name);
        assert_eq!(flow.options.output_name.as_deref(), Some("my-artifact"));

        // Backspace inside the editor removes a char.
        flow.handle_key(KeyCode::Char('o'));
        flow.handle_key(KeyCode::Backspace);
        assert_eq!(flow.output_name_buffer, "my-artifac");
        flow.handle_key(KeyCode::Enter);
        assert_eq!(flow.options.output_name.as_deref(), Some("my-artifac"));

        // Empty commit restores auto naming.
        flow.handle_key(KeyCode::Char('o'));
        for _ in 0..flow.output_name_buffer.len() {
            flow.handle_key(KeyCode::Backspace);
        }
        assert!(flow.output_name_buffer.is_empty());
        flow.handle_key(KeyCode::Enter);
        assert_eq!(flow.options.output_name, None);

        // Esc discards an in-progress edit without touching options.
        flow.handle_key(KeyCode::Char('o'));
        flow.handle_key(KeyCode::Char('x'));
        flow.handle_key(KeyCode::Char('y'));
        flow.handle_key(KeyCode::Esc);
        assert!(!flow.editing_output_name);
        assert_eq!(flow.options.output_name, None);
        assert!(flow.output_name_buffer.is_empty());

        // Editor ignores the option-toggle keys (they don't leak through).
        flow.handle_key(KeyCode::Char('o'));
        assert_eq!(flow.handle_key(KeyCode::Char('c')), None);
        assert!(flow.options.deny_content_secrets);
        flow.handle_key(KeyCode::Esc);
    }

    #[test]
    fn running_esc_is_blocked() {
        let mut flow = PackFlow::new(
            PathBuf::from("/proj"),
            PathBuf::from("/den"),
            PackFlowOptions::default(),
        );
        flow.phase = PackFlowPhase::Running;
        assert_eq!(flow.handle_key(KeyCode::Esc), None);
    }

    #[test]
    fn done_close() {
        let mut flow = PackFlow::new(
            PathBuf::from("/proj"),
            PathBuf::from("/den"),
            PackFlowOptions::default(),
        );
        flow.phase = PackFlowPhase::Done(PackResult {
            source: PathBuf::from("/proj"),
            output: PathBuf::from("/den/packs/x.tar.zst"),
            size_bytes: 100,
            file_count: 5,
            skipped_secret_files: 0,
            dry_run: false,
        });
        assert_eq!(flow.handle_key(KeyCode::Enter), Some(PackCommand::Close));
        assert_eq!(flow.handle_key(KeyCode::Esc), Some(PackCommand::Close));
    }
}
