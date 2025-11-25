use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Header;
use crate::character::repr::{BinaryRepr, Character, StateImage};
use crate::image::encode_image_data;

mod repr;
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

pub fn process_character(cli: CharacterCli) -> anyhow::Result<()> {
    let char: Character = serde_json::from_str(&fs::read_to_string(cli.input_file)?)?;

    let file = File::create(cli.output_file)?;

    let char_path = format!("characters/{}", char.id);

    let mut archive = tar::Builder::new(file);
    append_vec(&mut archive, format!("{char_path}/character.bin"), &char.to_bin()?)?;

    for (state_name, state) in &char.states {
        let state_path = format!("{char_path}/states/{state_name}");
        append_vec(&mut archive, format!("{state_path}/state.bin"), &state.to_bin()?)?;

        if let StateImage::Single {
            name,
            path,
            width,
            height,
            alpha,
            ..
        } = &state.image {
            let file = fs::read(path)?;
            append_vec(
                &mut archive,
                format!("{char_path}/images/{name}.bin"),
                &encode_image_data(&file, *width, *height, *alpha, true)?
            )?;
        }

        let transitions_path = format!("{state_path}/transitions");
        for transition in &state.transitions {
            append_vec(
                &mut archive,
                format!("{transitions_path}/{}/transition.bin", transition.to_state),
                &transition.to_bin()?
            )?;
        }
    }

    for (anim_name, anim) in &char.animations {
        let anim_path = format!("{char_path}/animations/{anim_name}");
        append_vec(&mut archive, format!("{anim_path}/animation.bin"), &anim.to_bin()?)?;

        for index in 1..=anim.frame_count {
            let image = fs::read(
                anim.frame_folder
                    .join(
                        format!("{index}.{}", anim.frame_extension)
                    )
            )?;

            append_vec(
                &mut archive,
                format!("{anim_path}/frames/{index}.bin"),
                &encode_image_data(&image, anim.real_width(), anim.real_height(), false, false)?
            )?;
        }
    }

    for (action_name, action) in &char.actions {
        let action_path = format!("{char_path}/actions/{action_name}");
        append_vec(&mut archive, format!("{action_path}/action.bin"), &action.to_bin()?)?;
    }

    archive.finish()?;
    Ok(())
}