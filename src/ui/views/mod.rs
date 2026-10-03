pub mod format;
pub mod processing;
pub mod result;
pub mod scan;
pub mod settings;
pub mod start;

pub use format::render_format_view;
pub use processing::render_processing_view;
pub use result::render_result_view;
pub use scan::render_scan_view;
pub use settings::render_settings_view;
pub use start::render_start_view;
