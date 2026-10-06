// 창과 트레이 — 링 창(panel), 설정 창, 트레이 메뉴, Rust 가 보이는 글
#[cfg(target_os = "macos")]
pub mod app_menu;
pub mod panel;
pub mod path_panel;
pub mod ring_window;
pub mod settings_window;
pub mod texts;
pub mod tray;
