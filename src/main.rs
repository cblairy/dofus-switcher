    mod autoswitch;
    mod capture;
    mod gui;
    mod runtime;
    mod window;
    mod name_registry;
    mod string_utils;

    fn main() -> eframe::Result<()> {
        gui::run()
    }
