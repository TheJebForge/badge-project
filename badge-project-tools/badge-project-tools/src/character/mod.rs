use crate::character::repr::{AnimationFrameSource, BinaryRepr, Character, StateImage};
use crate::image::encode_image_data;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, fs};
use tar::{Builder, Header};

pub mod repr;
pub mod util;

#[derive(clap::Parser, Debug)]
#[command(
    about="Converts json file defining the character into a tar file that can be loaded into the microcontroller",
    long_about=None
)]
pub struct CharacterCli {
    #[arg(help = "Character JSON file")]
    input_file: PathBuf,
    #[arg(help = "Output file")]
    output_file: PathBuf,
}

fn append_vec<P: AsRef<Path>, T: std::io::Write>(builder: &mut tar::Builder<T>, path: P, data: &[u8]) -> std::io::Result<()> {
    let mut header = Header::new_gnu();
    header.set_mode(0o664);
    header.set_size(data.len() as u64);

    builder.append_data(&mut header, path, data)
}

fn save_image(char_path: impl AsRef<Path>, mut archive: &mut Builder<File>, name: &String, location: &Path, path: &Path, width: u32, height: u32, upscale: bool) -> anyhow::Result<()> {
    let real_width = if upscale {
        width / 2
    } else {
        width
    };

    let real_height = if upscale {
        height / 2
    } else {
        height
    };

    let file = fs::read(location.join(path))?;
    append_vec(
        &mut archive,
        PathBuf::from(char_path.as_ref()).join("images").join(format!("{name}.bin")),
        &encode_image_data(&file, real_width, real_height, true)?
    )?;
    Ok(())
}

pub fn process_character_cli(cli: CharacterCli) -> anyhow::Result<()> {
    let char: Character = serde_json::from_str(&fs::read_to_string(cli.input_file)?)?;
    process_character_archive(char, cli.output_file, env::current_dir()?)
}

pub fn process_character_archive(char: Character, path: impl AsRef<Path>, location: impl AsRef<Path>) -> anyhow::Result<()> {
    let location = location.as_ref();

    let file = File::create(path)?;

    let char_path = Path::new("characters").join(&char.id);

    let mut archive = Builder::new(file);
    append_vec(&mut archive, char_path.join("character.bin"), &char.to_bin()?)?;

    for (state_name, state) in &char.states {
        let state_path = char_path.join("states").join(state_name);
        append_vec(&mut archive, state_path.join("state.bin"), &state.to_bin()?)?;

        if let StateImage::Single {
            name,
            path,
            width,
            height,
            upscale,
            ..
        } = &state.image {
            save_image(&char_path, &mut archive, name, location, path, *width, *height, *upscale)?;
        }

        if let StateImage::Sequence {
            frames,
            ..
        } = &state.image {
            let frames_path = state_path.join("frames");
            for (index, frame) in frames.iter().enumerate() {
                // Save image file
                save_image(&char_path, &mut archive, &frame.name, location, &frame.path, frame.width, frame.height, frame.upscale)?;

                // Save frame
                let frame_path = frames_path.join(format!("{index}.bin"));
                append_vec(&mut archive, frame_path, &frame.to_bin()?)?;
            }
        }

        let transitions_path = state_path.join("transitions");
        for transition in &state.transitions {
            append_vec(
                &mut archive,
                transitions_path.join(&transition.to_state).join("transition.bin"),
                &transition.to_bin()?
            )?;
        }
    }

    for (anim_name, anim) in &char.animations {
        let anim_path = char_path.join("animations").join(anim_name);
        append_vec(&mut archive, anim_path.join("animation.bin"), &anim.to_bin()?)?;

        match &anim.frames {
            AnimationFrameSource::Indexed {
                count, folder, extension
            } => {
                let folder = location.join(folder);

                for index in 1..=*count {
                    let image = fs::read(
                        folder.join(format!("{index}.{}", extension))
                    )?;

                    append_vec(
                        &mut archive,
                        anim_path.join("frames").join(format!("{index}.bin")),
                        &encode_image_data(&image, anim.real_width(), anim.real_height(), false)?
                    )?;
                }
            }

            AnimationFrameSource::List(list) => {
                for (index, path) in list.iter().enumerate() {
                    let image = fs::read(location.join(path))?;

                    append_vec(
                        &mut archive,
                        anim_path.join("frames").join(format!("{index}.bin")),
                        &encode_image_data(&image, anim.real_width(), anim.real_height(), false)?
                    )?;
                }
            }
        }

    }

    for (action_name, action) in &char.actions {
        let action_path = char_path.join("actions").join(action_name);
        append_vec(&mut archive, action_path.join("action.bin"), &action.to_bin()?)?;
    }

    archive.finish()?;
    Ok(())
}