//! UI widgets for displaying monitoring data.

pub mod details;
pub mod host_panel;
pub mod summary;
pub mod table;

#[allow(unused_imports)]
pub use details::render_detail_panel;
pub use host_panel::render_host_panel;
pub use summary::render_summary_bar;
#[allow(unused_imports)]
pub use table::render_host_table;
