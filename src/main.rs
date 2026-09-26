// main.rs
mod autoswitch;
mod capture;
mod gui;
mod shortcut_biding;
pub mod window;

fn main() -> eframe::Result<()> {
    gui::run()
}
