use serde::{Deserialize, Serialize};

/// Design system alvo; a UI nativa mapeia os tokens para seus componentes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignSystem {
    Material3,
    Hig,
    Fluent,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeTokens {
    pub design_system: DesignSystem,
    pub colors: Colors,
    pub typography: Typography,
    pub shape: Shape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Colors {
    pub background: String,
    pub surface: String,
    pub on_surface: String,
    pub primary: String,
    pub on_primary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    pub font_family: String,
    pub base_size_px: f32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shape {
    pub corner_radius_px: f32,
    pub spacing_unit_px: f32,
}

impl ThemeTokens {
    pub fn from_json(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json).map_err(|e| crate::Error::Invalid(e.to_string()))
    }

    /// CSS injetado no conteúdo HTML (WebView) para seguir o tema.
    pub fn to_content_css(&self, font_scale: f32) -> String {
        format!(
            "html{{background:{bg};color:{fg};font-family:{ff};font-size:{fs}px;line-height:{lh};}}\
             a{{color:{pr};}}",
            bg = self.colors.background,
            fg = self.colors.on_surface,
            ff = self.typography.font_family,
            fs = self.typography.base_size_px * font_scale,
            lh = self.typography.line_height,
            pr = self.colors.primary,
        )
    }
}
