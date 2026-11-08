// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-i3bar — SigmaOS i3bar component inspired by Linux Mint / Omarchy / Arch Linux.
//!
//! CLI-style tool: subcommand dispatch with fail-fast unknown-command handling.
//! Principles: single responsibility, fail-fast, defensive programming,
//! component isolation, stable API, deterministic tests.

use std::fmt;
/// Errors for `ToolError` operations. Fail-fast: invalid input is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Tool name must not be empty
    EmptyName,
    /// Unrecognized subcommand
    UnknownCommand(String),
    /// Subcommand requires arguments
    MissingArgs(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "tool name must not be empty"),
            Self::UnknownCommand(x) => write!(f, "unrecognized subcommand: {x}"),
            Self::MissingArgs(x) => write!(f, "subcommand requires arguments: {x}"),
        }
    }
}

impl std::error::Error for ToolError {}

/// A command-line tool with a small fixed subcommand set.
#[derive(Debug)]
pub struct Tool {
    name: String,
}

impl Tool {
    /// Create the tool; fails fast on empty name.
    pub fn new(name: impl Into<String>) -> Result<Self, ToolError> {
        let name = name.into();
        if name.trim().is_empty() { return Err(ToolError::EmptyName); }
        Ok(Self { name })
    }

    /// Tool name.
    pub fn name(&self) -> &str { &self.name }

    /// Dispatch a subcommand. Supported: `status`, `version`, `help`.
    /// Commands needing args that get none fail fast.
    pub fn run(&self, args: &[&str]) -> Result<String, ToolError> {
        match args.first().copied().unwrap_or("help") {
            "status" => Ok(format!("{}: ready", self.name)),
            "version" => Ok(format!("{}: 0.1.0", self.name)),
            "help" => Ok(format!("{}: commands: status, version, help", self.name)),
            other => Err(ToolError::UnknownCommand(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_version() {
        let t = Tool::new("demo").unwrap();
        assert_eq!(t.run(&["status"]).unwrap(), "demo: ready");
        assert!(t.run(&["version"]).unwrap().contains("0.1.0"));
    }

    #[test]
    fn no_args_shows_help() {
        let t = Tool::new("demo").unwrap();
        assert!(t.run(&[]).unwrap().contains("commands"));
    }

    #[test]
    fn unknown_command_fails() {
        let t = Tool::new("demo").unwrap();
        assert!(matches!(t.run(&["bogus"]), Err(ToolError::UnknownCommand(_))));
    }

    #[test]
    fn empty_name_fails() {
        assert!(Tool::new(" ").is_err());
    }

}

/// Backwards-compatible component handle (kept for existing callers/tests).
#[derive(Debug, Default)]
pub struct Component {
    name: &'static str,
}

impl Component {
    /// Create the component handle.
    pub fn new() -> Self {
        Self { name: "SigmaOS Component" }
    }
}

// ─── i3bar domain model ───────────────────────────────────────────────

/// i3bar position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I3BarPosition { Top, Bottom }

/// i3bar block kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    WorkspaceButtons, Tray, Clock, Battery, Network,
    Custom(String),
}

/// i3bar block
#[derive(Debug, Clone)]
pub struct I3Block {
    pub name: String,
    pub kind: BlockKind,
    pub section: String, // left/center/right
    pub command: Option<String>,
    pub interval: u32,
}

impl I3Block {
    pub fn new(name: &str, kind: BlockKind, section: &str) -> Self {
        Self { name: name.into(), kind, section: section.into(), command: None, interval: 0 }
    }

    pub fn with_command(mut self, cmd: &str) -> Self {
        self.command = Some(cmd.into());
        self
    }
}

/// i3bar
#[derive(Debug, Clone)]
pub struct I3Bar {
    pub position: I3BarPosition,
    pub height: u32,
    pub blocks: Vec<I3Block>,
    pub visible: bool,
}

impl I3Bar {
    pub fn new(position: I3BarPosition) -> Self {
        Self { position, height: 25, blocks: vec![], visible: true }
    }

    pub fn add_block(&mut self, block: I3Block) -> bool {
        if self.blocks.iter().any(|b| b.name == block.name) { return false; }
        self.blocks.push(block);
        true
    }

    pub fn remove_block(&mut self, name: &str) -> bool {
        let before = self.blocks.len();
        self.blocks.retain(|b| b.name != name);
        self.blocks.len() < before
    }

    pub fn blocks_in(&self, section: &str) -> Vec<&I3Block> {
        self.blocks.iter().filter(|b| b.section == section).collect()
    }

    pub fn toggle(&mut self) { self.visible = !self.visible; }
}

#[cfg(test)]
mod i3bar_tests {
    use super::*;

    #[test]
    fn test_position() {
        assert_eq!(I3BarPosition::Top as u8, I3BarPosition::Top as u8);
        assert_ne!(I3BarPosition::Top, I3BarPosition::Bottom);
    }

    #[test]
    fn test_block_creation() {
        let block = I3Block::new("clock", BlockKind::Clock, "right")
            .with_command("date +%H:%M");
        assert_eq!(block.name, "clock");
        assert!(block.command.is_some());
        assert_eq!(block.section, "right");
    }

    #[test]
    fn test_bar_blocks() {
        let mut bar = I3Bar::new(I3BarPosition::Top);
        bar.add_block(I3Block::new("workspaces", BlockKind::WorkspaceButtons, "left"));
        bar.add_block(I3Block::new("tray", BlockKind::Tray, "right"));
        bar.add_block(I3Block::new("clock", BlockKind::Clock, "right"));
        
        assert_eq!(bar.blocks.len(), 3);
        assert_eq!(bar.blocks_in("left").len(), 1);
        assert_eq!(bar.blocks_in("right").len(), 2);
    }

    #[test]
    fn test_duplicate_block_rejected() {
        let mut bar = I3Bar::new(I3BarPosition::Bottom);
        assert!(bar.add_block(I3Block::new("clock", BlockKind::Clock, "right")));
        assert!(!bar.add_block(I3Block::new("clock", BlockKind::Clock, "left")));
    }

    #[test]
    fn test_remove_block() {
        let mut bar = I3Bar::new(I3BarPosition::Top);
        bar.add_block(I3Block::new("tray", BlockKind::Tray, "right"));
        assert!(bar.remove_block("tray"));
        assert!(bar.blocks.is_empty());
    }

    #[test]
    fn test_toggle_visibility() {
        let mut bar = I3Bar::new(I3BarPosition::Bottom);
        assert!(bar.visible);
        bar.toggle();
        assert!(!bar.visible);
    }
}
