pub mod utility;

use itertools::Itertools;
use log::{debug, error};
use metaflac::Tag;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("codec not supported: {:?}", codec)]
    UnsupportedCodec { codec: Codec },

    #[error("file format invalid: {:?}", path)]
    InvalidFormat { path: PathBuf },

    #[error("error reading FLAC: {:?}", .0)]
    InvalidFlac(metaflac::Error),
}

impl From<metaflac::Error> for LibraryError {
    fn from(err: metaflac::Error) -> Self {
        Self::InvalidFlac(err)
    }
}

#[derive(Debug)]
pub struct Library {
    root: PathBuf,

    albums: HashMap<String, Album>,
}

impl Library {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            root: path.into(),
            albums: HashMap::new(),
        }
    }

    pub fn from_dir(path: impl Into<PathBuf>) -> Result<Self, LibraryError> {
        let root = path.into();
        let mut this = Self::new(&root);

        for entry in WalkDir::new(&root)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
            .map(|e| e.unwrap())
        {
            let key = this.key_relative(entry.path());
            debug!("read {}", &key);
            match Album::from_path(entry.path()) {
                Ok(album) => {
                    if album.len() > 0 {
                        this.albums.insert(key, album);
                    }
                }
                Err(err) => error!("{}: {}", &key, err),
            }
        }
        Ok(this)
    }

    pub fn collection(&self) -> &HashMap<String, Album> {
        &self.albums
    }

    pub fn keys(&self) -> Vec<&str> {
        let mut keys = self
            .albums
            .keys()
            .map(|x| x.as_ref())
            .collect::<Vec<&str>>();
        keys.sort();
        keys
    }

    pub fn albums(&self) -> Vec<&Album> {
        self.keys()
            .into_iter()
            .map(|key| &self.albums[key])
            .collect()
    }

    pub fn key_relative(&self, key: &Path) -> String {
        if key == self.root {
            return ".".into();
        }

        let root = self.root.to_string_lossy() + "/";
        key.to_string_lossy()
            .as_ref()
            .strip_prefix(root.as_ref())
            .expect("unexpected key outside library root")
            .to_owned()
    }
}

#[derive(Debug)]
pub struct Album {
    dir: PathBuf,

    tracks: Vec<Track>,
}

impl Album {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            dir: path.into(),
            tracks: Vec::new(),
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, LibraryError> {
        let dir = path.into();
        let mut this = Self::new(&dir);

        for entry in WalkDir::new(&dir)
            .sort_by_file_name()
            .max_depth(1)
            .into_iter()
            .map(|e| e.unwrap())
        {
            if entry.file_type().is_dir() {
                continue;
            }

            if Codec::from_path(entry.path()).is_some() {
                this.add_track(entry.path())?;
            }
        }
        Ok(this)
    }

    pub fn add_track(&mut self, path: impl Into<PathBuf>) -> Result<(), LibraryError> {
        self.tracks.push(Track::from_file(path)?);
        Ok(())
    }

    pub fn path(&self) -> &Path {
        self.dir.as_path()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn genres(&self) -> Vec<String> {
        self.tracks
            .iter()
            .map(|t| t.genres())
            .flatten()
            .fold(HashSet::new(), |mut hs, g| {
                hs.insert(g.to_owned());
                hs
            })
            .drain()
            .sorted()
            .collect()
    }
}

#[derive(Debug)]
pub enum Codec {
    FLAC,
    OPUS,
    M4A,
    MP3,
}

impl Codec {
    pub fn from_path(path: impl AsRef<Path>) -> Option<Codec> {
        path.as_ref()
            .extension()
            .map(|s| s.to_str().unwrap().to_lowercase())
            .and_then(|s| match s.as_str() {
                "flac" => Some(Codec::FLAC),
                "opus" => Some(Codec::OPUS),
                "m4a" => Some(Codec::M4A),
                "aac" => Some(Codec::M4A),
                "mp3" => Some(Codec::MP3),
                _ => None,
            })
    }
}

#[derive(Debug)]
pub struct Track {
    file: Option<PathBuf>,
    file_codec: Option<Codec>,

    title: Option<String>,        // flac:TITLE
    album: Option<String>,        // flac:ALBUM
    artist: Vec<String>,          // flac:ARTIST
    album_artist: Option<String>, // flac:ALBUMARTIST
    composer: Vec<String>,        // flac:COMPOSER
    track_number: Option<usize>,  // flac:TRACKNUMBER
    track_total: Option<usize>,   // flac:TRACKTOTAL
    disc_number: Option<usize>,   // flac:DISCNUMBER
    disc_total: Option<usize>,    // flac:DISCTOTAL
    date: Option<usize>,          // flac:DATE
    www: Option<String>,          // flac:CONTACT
    genre: Vec<String>,           // flac:GENRE
    copyright: Option<String>,    // flac:COPYRIGHT
    encoded_by: Option<String>,   // flac:ENCODED-BY
    comment: Option<String>,      // flac:DESCRIPTION

    /// Set to true when any field is modified, thereby signalling that
    /// the metadata should be written back to file.
    modified: bool,

    /// Contains fields that contain data that will not be written back.
    discarded: Vec<String>,
}

impl Track {
    /// Create a new empty Track.
    pub fn new() -> Self {
        Self {
            file: None,
            file_codec: None,

            title: None,
            album: None,
            artist: Vec::new(),
            album_artist: None,
            composer: Vec::new(),
            track_number: None,
            track_total: None,
            disc_number: None,
            disc_total: None,
            date: None,
            www: None,
            genre: Vec::new(),
            copyright: None,
            encoded_by: None,
            comment: None,

            modified: false,
            discarded: Vec::new(),
        }
    }

    /// Create a new Track from a file with a supported audio format.
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, LibraryError> {
        let path = path.into();
        if let Some(codec) = Codec::from_path(&path) {
            match codec {
                Codec::FLAC => Track::from_flac(path),
                _ => Err(LibraryError::UnsupportedCodec { codec: codec }),
            }
        } else {
            Err(LibraryError::InvalidFormat { path: path })
        }
    }

    /// Create a new Track from a FLAC file.
    pub fn from_flac(path: impl Into<PathBuf>) -> Result<Self, LibraryError> {
        let path = path.into();
        let tag = Tag::read_from_path(&path)?;
        if let Some(comments) = tag.vorbis_comments() {
            let discarded = RefCell::new(Vec::new());

            let get_str = |key: &str| -> Option<String> {
                comments.get(key).map(|xs| {
                    if xs.len() > 1 {
                        discarded.borrow_mut().push(key.into());
                    }
                    xs[0].clone()
                })
            };
            let get_strs = |key: &str| -> Vec<String> {
                comments
                    .get(key)
                    .map(|xs| xs.iter().map(|v| v.into()).collect())
                    .unwrap_or(Vec::new())
            };
            let get_num =
                |key: &str| -> Option<usize> { get_str(key).and_then(|x| x.parse().ok()) };

            Ok(Self {
                file: Some(path),
                file_codec: Some(Codec::FLAC),

                title: get_str("TITLE"),
                album: get_str("ALBUM"),
                artist: get_strs("ARTIST"),
                album_artist: get_str("ALBUMARTIST"),
                composer: get_strs("COMPOSER"),
                track_number: get_num("TRACKNUMBER"),
                track_total: get_num("TRACKTOTAL"),
                disc_number: get_num("DISCNUMBER"),
                disc_total: get_num("DISCTOTAL"),
                date: get_num("DATE"),
                www: get_str("CONTACT"),
                genre: get_strs("GENRE"),
                copyright: get_str("COPYRIGHT"),
                encoded_by: get_str("ENCODED-BY"),
                comment: get_str("DESCRIPTION"),

                modified: false,
                discarded: discarded.into_inner(),
            })
        } else {
            Ok(Self::new())
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.file.as_ref().map(|x| x.as_path())
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|x| x.as_str())
    }

    pub fn album(&self) -> Option<&str> {
        self.album.as_ref().map(|x| x.as_str())
    }

    pub fn artists(&self) -> &Vec<String> {
        &self.artist
    }

    pub fn album_artist(&self) -> Option<&str> {
        self.album_artist.as_ref().map(|x| x.as_str())
    }

    pub fn composers(&self) -> &Vec<String> {
        &self.composer
    }

    pub fn genres(&self) -> &Vec<String> {
        &self.genre
    }

    pub fn track_number(&self) -> Option<usize> {
        self.track_number
    }

    pub fn track_total(&self) -> Option<usize> {
        self.track_total
    }

    pub fn disc_number(&self) -> Option<usize> {
        self.disc_number
    }

    pub fn disc_total(&self) -> Option<usize> {
        self.disc_total
    }

    pub fn date(&self) -> Option<usize> {
        self.date
    }

    pub fn www(&self) -> Option<&str> {
        self.www.as_ref().map(|x| x.as_str())
    }

    pub fn copyright(&self) -> Option<&str> {
        self.copyright.as_ref().map(|x| x.as_str())
    }

    pub fn encoded_by(&self) -> Option<&str> {
        self.encoded_by.as_ref().map(|x| x.as_str())
    }

    pub fn comment(&self) -> Option<&str> {
        self.comment.as_ref().map(|x| x.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.album.is_none()
            && self.artist.is_empty()
            && self.album_artist.is_none()
            && self.composer.is_empty()
            && self.track_number.is_none()
            && self.track_total.is_none()
            && self.disc_number.is_none()
            && self.disc_total.is_none()
            && self.date.is_none()
            && self.www.is_none()
            && self.genre.is_empty()
            && self.copyright.is_none()
            && self.encoded_by.is_none()
            && self.comment.is_none()
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }
}
