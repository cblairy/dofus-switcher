mod autoswitch;
mod config;
mod gui;
mod name_registry;
mod runtime;
mod string_utils;
mod window;

fn main() -> eframe::Result<()> {
    gui::run()
}
