    mod autoswitch;
    mod capture;
    mod gui;
    mod runtime;
    mod window;

    fn main() -> eframe::Result<()> {
        gui::run()
    }
