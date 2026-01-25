mod app;

use eframe::{run_native, NativeOptions};
use crate::gui::app::GuiApp;

#[derive(clap::Parser, Debug)]
#[command(
    about="Enters graphical interface mode for the program",
    long_about=None
)]
pub struct GuiCli;

//noinspection RsUnwrap
pub fn start_gui() -> anyhow::Result<()> {
    let native_options = NativeOptions {
        ..Default::default()
    };

    match run_native(
        "BP Tools",
        native_options,
        Box::new(
            |cc| {
                Ok(Box::new(
                    GuiApp::new(cc)
                ))
            }
        )
    ) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{e}")
        }
    };

    Ok(())
}
