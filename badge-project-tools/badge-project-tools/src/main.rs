#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use crate::character::{process_character, CharacterCli};
use crate::image::{process_image, ImageCli};
use clap::Parser;
use crate::gui::{start_gui, GuiCli};

pub mod image;
pub mod character;
mod gui;

#[derive(clap::Subcommand, Debug)]
enum CliCommand {
    Image(ImageCli),
    Char(CharacterCli),
    Gui(GuiCli)
}

#[derive(clap::Parser, Debug)]
#[command(
    about="Set of utilities for badge-project firmware",
    long_about=None
)]
struct Cli {
    #[command(subcommand)]
    command: CliCommand
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Image(img) => process_image(img),
        CliCommand::Char(char) => process_character(char),
        CliCommand::Gui(_) => start_gui(),
    }
}
