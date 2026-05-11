use crate::core::raw_types::LocalizedString;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A widget definition from JSON type `"widget"`.
///
/// Widgets define UI elements such as text labels, number displays,
/// graphs, layouts, and sidebars.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WidgetDef {
    /// Unique identifier (e.g. "activity_desc", "compass_text_template").
    pub id: String,

    /// Display label for the widget (plain string or localized object).
    #[serde(default)]
    pub label: Option<LocalizedString>,

    /// Widget style: "text", "number", "graph", "layout", "sidebar", etc.
    #[serde(default)]
    pub style: Option<String>,

    /// Variable to display (e.g. "compass_text", "power_text", "bp_hp").
    #[serde(default)]
    pub var: Option<String>,

    /// Width of the widget in characters.
    #[serde(default)]
    pub width: Option<i32>,

    /// Height of the widget in characters.
    #[serde(default)]
    pub height: Option<i32>,

    /// Padding around the widget content.
    #[serde(default)]
    pub padding: Option<i32>,

    /// Text alignment: "left", "center", "right".
    #[serde(default)]
    pub text_align: Option<String>,

    /// Fill style for graph widgets: "bucket" or other.
    #[serde(default)]
    pub fill: Option<String>,

    /// Fill style (alternative field name).
    #[serde(default)]
    pub fill_style: Option<String>,

    /// Symbols used for graph rendering (e.g. ".\\|").
    #[serde(default)]
    pub symbols: Option<String>,

    /// Single symbol character for the widget.
    #[serde(default)]
    pub symbol: Option<String>,

    /// Color values — can be a single string or an array of strings.
    #[serde(default)]
    pub colors: Option<serde_json::Value>,

    /// Breakpoints for color changes in graph widgets.
    #[serde(default)]
    pub breaks: Option<Vec<f64>>,

    /// Whether the widget has a scrollbar.
    #[serde(default)]
    pub scrollbar: Option<bool>,

    /// Direction for compass widgets (e.g. "N", "S", "E", "W").
    #[serde(default)]
    pub direction: Option<String>,

    /// Body part associated with this widget (e.g. "head", "torso").
    #[serde(default)]
    pub bodypart: Option<String>,

    /// Layout direction: "columns", "rows", or other.
    #[serde(default)]
    pub arrange: Option<String>,

    /// Child widget IDs for layout/sidebar widgets.
    #[serde(default)]
    pub widgets: Option<Vec<String>>,

    /// Clauses for conditional text display.
    #[serde(default)]
    pub clauses: Option<Vec<WidgetClause>>,

    /// Separator string for sidebar widgets.
    #[serde(default)]
    pub separator: Option<String>,

    /// Flags (e.g. "W_LABEL_NONE", "W_NO_PADDING").
    #[serde(default)]
    pub flags: Option<Vec<String>>,

    /// Show or hide the label.
    #[serde(default)]
    pub show_label: Option<bool>,

    /// Whether the widget manages its own scrolling.
    #[serde(default)]
    pub manages_scroll: Option<bool>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A conditional clause for widget text display.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WidgetClause {
    /// Identifier for this clause.
    #[serde(default)]
    pub id: Option<String>,

    /// Text to display when the condition is met.
    #[serde(default)]
    pub text: Option<LocalizedString>,

    /// Color to use when this clause is active.
    #[serde(default)]
    pub color: Option<String>,

    /// Condition that must be true for this clause.
    #[serde(default)]
    pub condition: Option<serde_json::Value>,
}
