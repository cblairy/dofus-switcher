// main.rs
mod autoswitch;
mod capture;
mod gui;
mod runtime;
mod shortcut_biding;
mod window;

fn main() -> eframe::Result<()> {
    gui::run()
}
