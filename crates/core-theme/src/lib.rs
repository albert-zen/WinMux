use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "core-theme";

#[must_use]
pub fn crate_name() -> &'static str {
    CRATE_NAME
}

/// A set of 16 ANSI colors plus foreground, background, cursor, and selection colors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePalette {
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub selection: String,
    pub ansi: [String; 16],
}

/// A named terminal color theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub palette: ThemePalette,
}

/// Error type for theme registry operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeError {
    ThemeNotFound,
    InvalidJson,
    MissingField(String),
}

/// Registry of available themes with an active selection.
#[derive(Debug, Clone)]
pub struct ThemeRegistry {
    themes: Vec<Theme>,
    active_theme_id: String,
}

impl ThemeRegistry {
    /// Create a registry preloaded with built-in themes.
    /// The first built-in theme ("dark") is active by default.
    #[must_use]
    pub fn with_builtins() -> Self {
        let themes = vec![
            Self::builtin_dark(),
            Self::builtin_light(),
            Self::builtin_solarized_dark(),
        ];
        Self {
            active_theme_id: themes[0].id.clone(),
            themes,
        }
    }

    /// List all registered themes.
    #[must_use]
    pub fn list(&self) -> &[Theme] {
        &self.themes
    }

    /// Get the currently active theme.
    #[must_use]
    pub fn get_active(&self) -> &Theme {
        self.themes
            .iter()
            .find(|t| t.id == self.active_theme_id)
            .expect("active theme must exist in registry")
    }

    /// Get the active theme ID.
    #[must_use]
    pub fn active_theme_id(&self) -> &str {
        &self.active_theme_id
    }

    /// Set the active theme by ID. Returns error if theme not found.
    pub fn set_active(&mut self, id: &str) -> Result<(), ThemeError> {
        if !self.themes.iter().any(|t| t.id == id) {
            return Err(ThemeError::ThemeNotFound);
        }
        self.active_theme_id = id.to_string();
        Ok(())
    }

    /// Get a theme by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == id)
    }

    /// Import a theme from a JSON string. The theme is added to the registry.
    /// If a theme with the same ID already exists, it is replaced.
    pub fn import_from_json(&mut self, json: &str) -> Result<(), ThemeError> {
        let theme: Theme = serde_json::from_str(json).map_err(|_| ThemeError::InvalidJson)?;
        // Validate required fields
        if theme.id.is_empty() {
            return Err(ThemeError::MissingField("id".to_string()));
        }
        if theme.name.is_empty() {
            return Err(ThemeError::MissingField("name".to_string()));
        }
        // Replace existing or add new
        if let Some(existing) = self.themes.iter_mut().find(|t| t.id == theme.id) {
            *existing = theme;
        } else {
            self.themes.push(theme);
        }
        Ok(())
    }

    // -- Built-in themes --------------------------------------------------

    fn builtin_dark() -> Theme {
        Theme {
            id: "dark".to_string(),
            name: "Dark".to_string(),
            palette: ThemePalette {
                foreground: "#d4d4d4".to_string(),
                background: "#1e1e1e".to_string(),
                cursor: "#aeafad".to_string(),
                selection: "#264f78".to_string(),
                ansi: [
                    "#000000".to_string(), "#cd3131".to_string(),
                    "#0dbc79".to_string(), "#e5e510".to_string(),
                    "#2472c8".to_string(), "#bc3fbc".to_string(),
                    "#11a8cd".to_string(), "#e5e5e5".to_string(),
                    "#666666".to_string(), "#f14c4c".to_string(),
                    "#23d18b".to_string(), "#f5f543".to_string(),
                    "#3b8eea".to_string(), "#d670d6".to_string(),
                    "#29b8db".to_string(), "#ffffff".to_string(),
                ],
            },
        }
    }

    fn builtin_light() -> Theme {
        Theme {
            id: "light".to_string(),
            name: "Light".to_string(),
            palette: ThemePalette {
                foreground: "#383a42".to_string(),
                background: "#fafafa".to_string(),
                cursor: "#526eff".to_string(),
                selection: "#e5e5e6".to_string(),
                ansi: [
                    "#000000".to_string(), "#e45649".to_string(),
                    "#50a14f".to_string(), "#c18401".to_string(),
                    "#4078f2".to_string(), "#a626a4".to_string(),
                    "#0184bc".to_string(), "#a0a1a7".to_string(),
                    "#5c6370".to_string(), "#e06c75".to_string(),
                    "#98c379".to_string(), "#d19a66".to_string(),
                    "#61afef".to_string(), "#c678dd".to_string(),
                    "#56b6c2".to_string(), "#ffffff".to_string(),
                ],
            },
        }
    }

    fn builtin_solarized_dark() -> Theme {
        Theme {
            id: "solarized-dark".to_string(),
            name: "Solarized Dark".to_string(),
            palette: ThemePalette {
                foreground: "#839496".to_string(),
                background: "#002b36".to_string(),
                cursor: "#839496".to_string(),
                selection: "#073642".to_string(),
                ansi: [
                    "#073642".to_string(), "#dc322f".to_string(),
                    "#859900".to_string(), "#b58900".to_string(),
                    "#268bd2".to_string(), "#d33682".to_string(),
                    "#2aa198".to_string(), "#eee8d5".to_string(),
                    "#002b36".to_string(), "#cb4b16".to_string(),
                    "#586e75".to_string(), "#657b83".to_string(),
                    "#839496".to_string(), "#6c71c4".to_string(),
                    "#93a1a1".to_string(), "#fdf6e3".to_string(),
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_expected_name() {
        assert_eq!(crate_name(), CRATE_NAME);
    }

    #[test]
    fn builtins_contain_three_themes() {
        let reg = ThemeRegistry::with_builtins();
        assert_eq!(reg.list().len(), 3);
    }

    #[test]
    fn builtin_theme_ids_are_unique() {
        let reg = ThemeRegistry::with_builtins();
        let ids: Vec<&str> = reg.list().iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["dark", "light", "solarized-dark"]);
    }

    #[test]
    fn default_active_theme_is_dark() {
        let reg = ThemeRegistry::with_builtins();
        assert_eq!(reg.active_theme_id(), "dark");
        assert_eq!(reg.get_active().name, "Dark");
    }

    #[test]
    fn set_active_changes_active_theme() {
        let mut reg = ThemeRegistry::with_builtins();
        reg.set_active("light").expect("light theme should exist");
        assert_eq!(reg.active_theme_id(), "light");
        assert_eq!(reg.get_active().name, "Light");
    }

    #[test]
    fn set_active_rejects_unknown_theme_id() {
        let mut reg = ThemeRegistry::with_builtins();
        let err = reg.set_active("nonexistent").unwrap_err();
        assert_eq!(err, ThemeError::ThemeNotFound);
    }

    #[test]
    fn get_returns_theme_by_id() {
        let reg = ThemeRegistry::with_builtins();
        let theme = reg.get("solarized-dark").expect("should find solarized-dark");
        assert_eq!(theme.name, "Solarized Dark");
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let reg = ThemeRegistry::with_builtins();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn builtin_palettes_have_16_ansi_colors() {
        let reg = ThemeRegistry::with_builtins();
        for theme in reg.list() {
            assert_eq!(theme.palette.ansi.len(), 16, "theme {} should have 16 ANSI colors", theme.id);
        }
    }

    #[test]
    fn import_from_json_adds_new_theme() {
        let mut reg = ThemeRegistry::with_builtins();
        let json = serde_json::to_string(&Theme {
            id: "monokai".to_string(),
            name: "Monokai".to_string(),
            palette: ThemePalette {
                foreground: "#f8f8f2".to_string(),
                background: "#272822".to_string(),
                cursor: "#f8f8f0".to_string(),
                selection: "#49483e".to_string(),
                ansi: std::array::from_fn(|_| "#000000".to_string()),
            },
        }).unwrap();

        reg.import_from_json(&json).expect("import should succeed");
        assert_eq!(reg.list().len(), 4);
        assert_eq!(reg.get("monokai").unwrap().name, "Monokai");
    }

    #[test]
    fn import_from_json_replaces_existing_theme() {
        let mut reg = ThemeRegistry::with_builtins();
        let json = serde_json::to_string(&Theme {
            id: "dark".to_string(),
            name: "Dark Modified".to_string(),
            palette: reg.get("dark").unwrap().palette.clone(),
        }).unwrap();

        reg.import_from_json(&json).expect("import should succeed");
        assert_eq!(reg.list().len(), 3); // same count
        assert_eq!(reg.get("dark").unwrap().name, "Dark Modified");
    }

    #[test]
    fn import_from_json_rejects_invalid_json() {
        let mut reg = ThemeRegistry::with_builtins();
        let err = reg.import_from_json("{not valid}").unwrap_err();
        assert_eq!(err, ThemeError::InvalidJson);
    }

    #[test]
    fn import_from_json_rejects_empty_id() {
        let mut reg = ThemeRegistry::with_builtins();
        let json = r##"{"id":"","name":"Empty","palette":{"foreground":"#fff","background":"#000","cursor":"#fff","selection":"#333","ansi":["#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000"]}}"##;
        let err = reg.import_from_json(json).unwrap_err();
        assert_eq!(err, ThemeError::MissingField("id".to_string()));
    }

    #[test]
    fn import_from_json_rejects_empty_name() {
        let mut reg = ThemeRegistry::with_builtins();
        let json = r##"{"id":"test","name":"","palette":{"foreground":"#fff","background":"#000","cursor":"#fff","selection":"#333","ansi":["#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000","#000"]}}"##;
        let err = reg.import_from_json(json).unwrap_err();
        assert_eq!(err, ThemeError::MissingField("name".to_string()));
    }

    #[test]
    fn theme_serde_roundtrip() {
        let reg = ThemeRegistry::with_builtins();
        for theme in reg.list() {
            let json = serde_json::to_string(theme).expect("serialize should succeed");
            let restored: Theme = serde_json::from_str(&json).expect("deserialize should succeed");
            assert_eq!(&restored, theme);
        }
    }
}
