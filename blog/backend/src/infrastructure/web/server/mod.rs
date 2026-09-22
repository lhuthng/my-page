// Composition root: builds the service registry and the HTTP server. The
// per-concern configuration lives in `config/`, the runtime builder in
// `lifecycle.rs`, and boot/periodic jobs in `maintenance/`.
pub mod config;
pub mod lifecycle;
pub mod maintenance;
pub mod state;

pub use lifecycle::HTTPServer;
pub use state::{
    AppConfig, AppState, DatabaseSource, MailConfig, MailTransportConfig, MediaConfig,
    ProjectDemoConfig,
};
