use clap::{App, Arg};
use env_logger;
use log::{error, LevelFilter};
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;

use vicuno::{Library, LibraryError};

fn main() -> Result<(), AppError> {
    let matches = App::new("vicuno")
        .version("0.1")
        .arg(
            Arg::with_name("dry-run")
                .short("n")
                .long("dry-run")
                .help("Just print what would happen without doing anything."),
        )
        .arg(
            Arg::with_name("tag")
                .short("t")
                .long("tag")
                .value_name("TAG")
                .default_value("genres")
                .possible_values(&["genres"])
                .help("Tag to read and write."),
        )
        .arg(
            Arg::with_name("editor")
                .short("e")
                .long("editor")
                .value_name("EDITOR")
                .env("EDITOR")
                .help("Use the following editor for making changes."),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Print more information"),
        )
        .arg(
            Arg::with_name("DIRECTORY")
                .help("Directory to read metadata from.")
                .required(true)
                .index(1),
        )
        .get_matches();

    env_logger::builder()
        .parse_env("VICUNO_LOG")
        .filter(
            None,
            if matches.is_present("verbose") {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            },
        )
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    // --------------------------------------------------------------------- //

    let path: PathBuf = matches.value_of("DIRECTORY").unwrap().into();
    let library = Library::from_dir(path)?;
    if library.collection().is_empty() {
        error!("No albums found.");
        return Ok(());
    }

    let keys = library.keys();
    let mut fmt = vicuno::utility::KeyValueFormatter::new();
    fmt.key_padding = keys.iter().map(|x| x.len()).max().unwrap();
    for key in keys {
        println!(
            "{}",
            fmt.format_multi(key, library.collection().get(key).unwrap().genres())
        );
    }
    Ok(())
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("directory cannot be opened: {path}")]
    CannotOpenDir { path: String },

    #[error("cannot read file: {}", .0)]
    LibraryError(LibraryError),
}

impl From<LibraryError> for AppError {
    fn from(err: LibraryError) -> Self {
        Self::LibraryError(err)
    }
}

pub enum Tag {
    Title,
    Artists,
    Album,
    AlbumArtist,
    Composers,
    Genres,
    Copyright,
    Date,
    EncodedBy,
    Comment,
}
