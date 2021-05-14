use env_logger;
use log::{error, LevelFilter};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;

use vicuno::{Library, LibraryError};

#[derive(Debug, StructOpt)]
#[structopt(name = "vicuno", about = "Edit audio metadata with your editor.")]
struct Options {
    /// Print more information.
    #[structopt(short, long)]
    verbose: bool,

    /// Just print what would happen without doing anything.
    #[structopt(short = "n", long)]
    dry_run: bool,

    /// Editor to use for making changes.
    #[structopt(short, long, env = "EDITOR")]
    editor: String,

    /// Tag to read and write.
    #[structopt(
        short,
        long,
        default_value = "genre",
        possible_values = &[
            "album",
            "album_artist",
            "artist",
            "comment",
            "composer",
            "copyright",
            "date",
            "encoded_by",
            "genre",
            "title",
        ],
    )]
    tag: Tag,

    /// Directory to scan for audio files.
    #[structopt(parse(from_os_str))]
    root: PathBuf,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("root is not a directory: {}", .0)]
    InvalidRoot(String),

    #[error("invalid tag type: {}", .0)]
    InvalidTag(String),

    #[error("cannot read file: {}", .0)]
    LibraryError(LibraryError),
}

impl From<LibraryError> for AppError {
    fn from(err: LibraryError) -> Self {
        Self::LibraryError(err)
    }
}

fn main() -> Result<(), AppError> {
    let opt = Options::from_args();
    if !opt.root.is_dir() {
        return Err(AppError::InvalidRoot(
            opt.root.to_string_lossy().to_string(),
        ));
    }

    env_logger::builder()
        .parse_env("VICUNO_LOG")
        .filter(
            None,
            if opt.verbose {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            },
        )
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    // --------------------------------------------------------------------- //

    let library = Library::from_dir(&opt.root)?;
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

#[derive(Debug)]
pub enum Tag {
    Album,
    AlbumArtist,
    Artist,
    Comment,
    Composer,
    Copyright,
    Date,
    EncodedBy,
    Genre,
    Title,
}

impl FromStr for Tag {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let simple = s.to_lowercase().replace("_", "-");
        match simple.as_str() {
            "album" => Ok(Tag::Album),
            "album_artist" => Ok(Tag::AlbumArtist),
            "artist" => Ok(Tag::Artist),
            "comment" => Ok(Tag::Comment),
            "composer" => Ok(Tag::Composer),
            "copyright" => Ok(Tag::Copyright),
            "date" => Ok(Tag::Date),
            "encoded_by" => Ok(Tag::EncodedBy),
            "genre" => Ok(Tag::Genre),
            "title" => Ok(Tag::Title),
            _ => Err(AppError::InvalidTag(s.to_owned())),
        }
    }
}
