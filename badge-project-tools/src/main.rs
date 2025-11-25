#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use crate::character::{process_character, CharacterCli};
use crate::image::{process_image, ImageCli};
use clap::Parser;

pub mod image;
pub mod character;

#[derive(clap::Subcommand, Debug)]
enum CliCommand {
    Image(ImageCli),
    Char(CharacterCli)
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
    }
}
