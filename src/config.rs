use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Markup used for a coloured span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColourStyle {
    /// Obsidian 1.14+ native colour highlight: `==🟣text==`.
    #[default]
    Obsidian,
    /// Obsidian Painter class: `<mark class="hltr-p">text</mark>`.
    Painter,
}

/// How each highlight is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HighlightLayout {
    /// Blockquote, then a `**Highlight** (colour) - Page - Location` line.
    #[default]
    Quote,
    /// One line, Flint-style: `text — colour | Page - Location`.
    Line,
}

/// Where highlight colours are shown, and in which markup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HighlightColours {
    pub style: ColourStyle,
    /// Colour the highlighted passage.
    pub text: bool,
    /// Colour the colour-name label on the metadata line.
    pub label: bool,
}

impl Default for HighlightColours {
    fn default() -> Self {
        Self {
            style: ColourStyle::Obsidian,
            text: false,
            label: true,
        }
    }
}

impl HighlightColours {
    /// What `enable_painter_highlights = true` always produced.
    const LEGACY_PAINTER: Self = Self {
        style: ColourStyle::Painter,
        text: false,
        label: true,
    };
}

/// Everything that shapes a rendered highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HighlightFormat {
    pub layout: HighlightLayout,
    pub colours: HighlightColours,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_export_folder: PathBuf,
    // Plain values before tables, or the TOML serialiser refuses to write it.
    #[serde(default)]
    highlight_layout: HighlightLayout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    highlight_colours: Option<HighlightColours>,
    /// Legacy switch from before `[highlight_colours]`: read so old configs
    /// keep working, never written back.
    #[serde(default, skip_serializing)]
    enable_painter_highlights: bool,
}

impl Default for Config {
    fn default() -> Self {
        // Default to user's home directory under Documents/Inkwell
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            default_export_folder: PathBuf::from(home).join("Documents/Inkwell"),
            highlight_layout: HighlightLayout::default(),
            highlight_colours: Some(HighlightColours::default()),
            enable_painter_highlights: false,
        }
    }
}

impl Config {
    /// The configured colours; `[highlight_colours]` wins over the legacy
    /// `enable_painter_highlights` flag.
    pub fn highlight_colours(&self) -> HighlightColours {
        match (self.highlight_colours, self.enable_painter_highlights) {
            (Some(colours), _) => colours,
            (None, true) => HighlightColours::LEGACY_PAINTER,
            (None, false) => HighlightColours::default(),
        }
    }

    pub fn highlight_format(&self) -> HighlightFormat {
        HighlightFormat {
            layout: self.highlight_layout,
            colours: self.highlight_colours(),
        }
    }

    pub fn load() -> Result<Self> {
        // Try local config.toml first (in same directory as binary or current directory)
        let local_config = PathBuf::from("./config.toml");

        // Try binary directory config
        let binary_dir_config = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("config.toml")));

        // Try user config directory
        let user_config = Self::user_config_path().ok();

        // Check in order: local, binary dir, user config
        let config_path = if local_config.exists() {
            local_config
        } else if let Some(ref path) = binary_dir_config {
            if path.exists() {
                path.clone()
            } else if let Some(ref user_path) = user_config {
                user_path.clone()
            } else {
                return Ok(Config::default());
            }
        } else if let Some(ref user_path) = user_config {
            user_path.clone()
        } else {
            return Ok(Config::default());
        };

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .context(format!("Failed to read config file: {:?}", config_path))?;
            let config: Config =
                toml::from_str(&contents).context("Failed to parse config file")?;
            Ok(config)
        } else {
            // Only auto-create in user config directory
            if let Some(user_path) = user_config {
                let config = Config::default();
                if let Some(parent) = user_path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                if let Ok(contents) = toml::to_string_pretty(&config) {
                    fs::write(&user_path, contents).ok();
                }
                Ok(config)
            } else {
                Ok(Config::default())
            }
        }
    }

    fn user_config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config/inkwell/config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(extra: &str) -> Result<Config> {
        let contents = format!("default_export_folder = \"/tmp/notes\"\n{}", extra);
        Ok(toml::from_str(&contents)?)
    }

    #[test]
    fn colours_default_to_obsidian_label_only() -> Result<()> {
        let colours = parse("")?.highlight_colours();
        assert_eq!(colours.style, ColourStyle::Obsidian);
        assert!(!colours.text);
        assert!(colours.label);
        Ok(())
    }

    #[test]
    fn reads_highlight_colours_table() -> Result<()> {
        let colours =
            parse("[highlight_colours]\nstyle = \"painter\"\ntext = false\nlabel = true")?
                .highlight_colours();
        assert_eq!(
            colours,
            HighlightColours {
                style: ColourStyle::Painter,
                text: false,
                label: true,
            }
        );
        Ok(())
    }

    #[test]
    fn missing_keys_in_table_take_defaults() -> Result<()> {
        let colours = parse("[highlight_colours]\nlabel = false")?.highlight_colours();
        assert_eq!(colours.style, ColourStyle::Obsidian);
        assert!(!colours.text);
        assert!(!colours.label);
        Ok(())
    }

    #[test]
    fn legacy_painter_flag_keeps_its_old_output() -> Result<()> {
        assert_eq!(
            parse("enable_painter_highlights = true")?.highlight_colours(),
            HighlightColours::LEGACY_PAINTER
        );
        Ok(())
    }

    #[test]
    fn legacy_painter_flag_off_falls_back_to_default() -> Result<()> {
        assert_eq!(
            parse("enable_painter_highlights = false")?.highlight_colours(),
            HighlightColours::default()
        );
        Ok(())
    }

    #[test]
    fn colours_table_overrides_legacy_painter_flag() -> Result<()> {
        let config =
            parse("enable_painter_highlights = true\n[highlight_colours]\nstyle = \"obsidian\"")?;
        assert_eq!(config.highlight_colours(), HighlightColours::default());
        Ok(())
    }

    #[test]
    fn rejects_unknown_colour_style() {
        assert!(parse("[highlight_colours]\nstyle = \"neon\"").is_err());
    }

    #[test]
    fn default_config_writes_colours_but_not_legacy_flag() -> Result<()> {
        let written = toml::to_string_pretty(&Config::default())?;
        assert!(written.contains("[highlight_colours]"));
        assert!(written.contains("style = \"obsidian\""));
        assert!(!written.contains("enable_painter_highlights"));
        Ok(())
    }

    #[test]
    fn layout_defaults_to_quote() -> Result<()> {
        assert_eq!(parse("")?.highlight_format().layout, HighlightLayout::Quote);
        Ok(())
    }

    #[test]
    fn reads_line_layout() -> Result<()> {
        let format = parse("highlight_layout = \"line\"")?.highlight_format();
        assert_eq!(format.layout, HighlightLayout::Line);
        assert_eq!(format.colours, HighlightColours::default());
        Ok(())
    }

    #[test]
    fn default_config_writes_layout() -> Result<()> {
        let written = toml::to_string_pretty(&Config::default())?;
        assert!(
            written.contains("highlight_layout = \"quote\""),
            "{written}"
        );
        Ok(())
    }
}
