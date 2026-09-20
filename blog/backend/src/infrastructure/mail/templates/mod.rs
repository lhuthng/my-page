/// Branded HTML email templates.
///
/// Email clients strip `<style>` blocks and don't support flexbox/grid, so
/// every email is rendered as a table-based layout with inline styles only.
/// The design is a light theme (white card on light gray) with the site's
/// navy/lavender accents, matching the huuthangle.site brand.
mod auth;
mod layout;
mod newsletter;

use layout::{render_shell, site_footer};

pub use auth::{password_reset, verification};
pub use layout::render_shell;
pub use newsletter::{
    campaign, campaign_post_body, campaign_post_text, subscription_confirm, CampaignPostData,
};
#[cfg(debug_assertions)]
pub use layout::{logo_data_uri, render_button};
#[cfg(debug_assertions)]
pub use newsletter::preview_page;

pub const FONT_STACK: &str = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif";

const BACKGROUND: &str = "#eef0f6";
const CARD: &str = "#ffffff";
const NAVY: &str = "#495c83";
const PRIMARY: &str = "#7a86b6";
const TEXT: &str = "#333a4d";
const MUTED: &str = "#6b7280";
const DIVIDER: &str = "#e5e7eb";
const WHITE: &str = "#ffffff";

/// Escapes a plain-text value for safe inclusion in an HTML attribute or body.
pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Dev-only: embeds `logo.png` (a copy of `frontend/static/logo.png`) as a
/// base64 data URI so the preview page shows the logo even when the frontend
/// origin isn't reachable. Real emails keep the hosted `{app_base_url}/logo.png`.
