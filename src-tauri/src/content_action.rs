use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

const MEDIA_EXTENSIONS: &[&str] = &[
    "aac", "avi", "bmp", "flac", "flv", "gif", "heic", "jpg", "jpeg", "m4a", "m4v", "mkv", "mov",
    "mp3", "mp4", "mpeg", "mpg", "ogg", "opus", "png", "tif", "tiff", "vob", "wav", "webm", "wma",
    "webp", "wmv",
];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContentAction {
    Open,
    Reveal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentFile {
    pub id: u32,
    pub path: String,
    pub progress: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TorrentContent {
    pub content_path: PathBuf,
    pub root_path: Option<PathBuf>,
    pub files: Vec<ContentFile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetKind {
    File,
    Directory,
}

#[derive(Debug, PartialEq, Eq)]
struct ContentTarget {
    path: PathBuf,
    containment_root: PathBuf,
    kind: TargetKind,
    progress_complete: bool,
}

pub fn perform(
    content: &TorrentContent,
    file_id: Option<u32>,
    action: ContentAction,
) -> Result<(), String> {
    let target = resolve_target(content, file_id)?;
    let path = existing_target_path(&target, action)?;

    if action == ContentAction::Open && target.kind == TargetKind::File {
        if !target.progress_complete {
            return Err(
                "This file is still downloading. Only completed files can be opened.".to_string(),
            );
        }
        if !is_media_path(&path) {
            return Err(
                "Only completed audio, video, and image files can be opened directly.".to_string(),
            );
        }
    }

    let selected_file = file_id.is_some();
    let result = match (action, target.kind) {
        (ContentAction::Reveal, TargetKind::File) if selected_file => {
            match tauri_plugin_opener::reveal_item_in_dir(&path) {
                Ok(()) => Ok(()),
                Err(_) => tauri_plugin_opener::open_path(
                    opener_path(&path, action, TargetKind::File, false)?,
                    None::<&str>,
                ),
            }
        }
        _ => tauri_plugin_opener::open_path(
            opener_path(&path, action, target.kind, selected_file)?,
            None::<&str>,
        ),
    };

    result.map_err(|error| format!("The operating system could not open this content: {error}"))
}

fn opener_path(
    path: &Path,
    action: ContentAction,
    kind: TargetKind,
    selected_file: bool,
) -> Result<&Path, String> {
    if action == ContentAction::Reveal && kind == TargetKind::File && !selected_file {
        return path.parent().ok_or_else(|| {
            "The downloaded file's containing folder could not be found.".to_string()
        });
    }

    Ok(path)
}

fn resolve_target(content: &TorrentContent, file_id: Option<u32>) -> Result<ContentTarget, String> {
    if content.files.is_empty() || content.content_path.as_os_str().is_empty() {
        return Err("The torrent's file information is not available yet.".to_string());
    }
    if !content.content_path.is_absolute()
        || content
            .root_path
            .as_deref()
            .is_some_and(|path| !path.is_absolute())
    {
        return Err("qBittorrent returned an invalid content path.".to_string());
    }

    if let Some(file_id) = file_id {
        let file = content
            .files
            .iter()
            .find(|file| file.id == file_id)
            .ok_or_else(|| "That torrent file no longer exists.".to_string())?;
        let (path, containment_root) = if content.files.len() == 1 {
            let root = content
                .content_path
                .parent()
                .unwrap_or(&content.content_path)
                .to_path_buf();
            (content.content_path.clone(), root)
        } else {
            let base = content
                .root_path
                .as_deref()
                .filter(|path| !path.as_os_str().is_empty())
                .and_then(Path::parent)
                .unwrap_or(&content.content_path)
                .to_path_buf();
            let relative = safe_relative_path(&file.path)?;
            (base.join(relative), base)
        };

        return Ok(ContentTarget {
            path,
            containment_root,
            kind: TargetKind::File,
            progress_complete: file.progress >= 1.0,
        });
    }

    if content.files.len() == 1 {
        let file = &content.files[0];
        return Ok(ContentTarget {
            containment_root: content
                .content_path
                .parent()
                .unwrap_or(&content.content_path)
                .to_path_buf(),
            path: content.content_path.clone(),
            kind: TargetKind::File,
            progress_complete: file.progress >= 1.0,
        });
    }

    Ok(ContentTarget {
        path: content.content_path.clone(),
        containment_root: content.content_path.clone(),
        kind: TargetKind::Directory,
        progress_complete: true,
    })
}

fn safe_relative_path(path: &str) -> Result<PathBuf, String> {
    let mut relative = PathBuf::new();
    for segment in path.split(['/', '\\']) {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err("qBittorrent returned an unsafe file path.".to_string());
        }
        #[cfg(windows)]
        if segment.contains(':') {
            return Err("qBittorrent returned an unsafe file path.".to_string());
        }
        relative.push(segment);
    }

    if relative.as_os_str().is_empty() || relative.is_absolute() {
        return Err("qBittorrent returned an unsafe file path.".to_string());
    }
    Ok(relative)
}

fn existing_target_path(target: &ContentTarget, action: ContentAction) -> Result<PathBuf, String> {
    let candidate = if target.path.exists() {
        target.path.clone()
    } else if action == ContentAction::Reveal && target.kind == TargetKind::File {
        let partial = PathBuf::from(format!("{}.!qB", target.path.to_string_lossy()));
        if partial.exists() {
            partial
        } else {
            return Err(
                "The downloaded file could not be found at its expected location.".to_string(),
            );
        }
    } else {
        let noun = if target.kind == TargetKind::Directory {
            "folder"
        } else {
            "file"
        };
        return Err(format!(
            "The downloaded {noun} could not be found at its expected location."
        ));
    };

    let canonical_root = fs::canonicalize(&target.containment_root).map_err(|_| {
        "The downloaded content's containing folder could not be accessed.".to_string()
    })?;
    let canonical_path = fs::canonicalize(&candidate)
        .map_err(|_| "The downloaded content could not be accessed.".to_string())?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err("The downloaded file resolves outside its torrent folder.".to_string());
    }

    let metadata = fs::metadata(&canonical_path)
        .map_err(|_| "The downloaded content could not be accessed.".to_string())?;
    match target.kind {
        TargetKind::File if !metadata.is_file() => {
            Err("The downloaded file could not be found at its expected location.".to_string())
        }
        TargetKind::Directory if !metadata.is_dir() => {
            Err("The downloaded folder could not be found at its expected location.".to_string())
        }
        _ => Ok(canonical_path),
    }
}

fn is_media_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            MEDIA_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute_path(path: &str) -> PathBuf {
        #[cfg(windows)]
        return PathBuf::from(format!("C:/{path}"));

        #[cfg(not(windows))]
        return PathBuf::from(format!("/{path}"));
    }

    fn content(
        content_path: &str,
        root_path: Option<&str>,
        files: &[(&str, f64)],
    ) -> TorrentContent {
        TorrentContent {
            content_path: absolute_path(content_path),
            root_path: root_path.map(absolute_path),
            files: files
                .iter()
                .enumerate()
                .map(|(id, (path, progress))| ContentFile {
                    id: id as u32,
                    path: (*path).to_string(),
                    progress: *progress,
                })
                .collect(),
        }
    }

    #[test]
    fn resolves_single_file_to_the_authoritative_content_path() {
        let content = content(
            "downloads/renamed/movie.mkv",
            None,
            &[("original-name.mkv", 1.0)],
        );

        let target = resolve_target(&content, Some(0)).unwrap();

        assert_eq!(target.path, absolute_path("downloads/renamed/movie.mkv"));
        assert_eq!(target.kind, TargetKind::File);
        assert!(target.progress_complete);
    }

    #[test]
    fn resolves_rooted_and_rootless_multi_file_layouts() {
        let rooted = content(
            "downloads/Show",
            Some("downloads/Show"),
            &[("Show/episode.mkv", 1.0), ("Show/poster.jpg", 1.0)],
        );
        let rootless = content(
            "downloads",
            None,
            &[("episode.mkv", 1.0), ("poster.jpg", 1.0)],
        );

        assert_eq!(
            resolve_target(&rooted, Some(0)).unwrap().path,
            absolute_path("downloads/Show/episode.mkv")
        );
        assert_eq!(
            resolve_target(&rootless, Some(0)).unwrap().path,
            absolute_path("downloads/episode.mkv")
        );
        assert_eq!(
            resolve_target(&rooted, None).unwrap().kind,
            TargetKind::Directory
        );
    }

    #[test]
    fn rejects_traversal_and_absolute_file_paths() {
        for path in ["../outside.mkv", "folder/../../outside.mkv", "/outside.mkv"] {
            let content = content("downloads/Show", None, &[(path, 1.0), ("safe.mkv", 1.0)]);
            assert_eq!(
                resolve_target(&content, Some(0)).unwrap_err(),
                "qBittorrent returned an unsafe file path."
            );
        }
    }

    #[test]
    fn media_allowlist_excludes_svg_documents_and_executables() {
        for path in ["movie.MKV", "song.flac", "poster.jpeg", "scan.tif"] {
            assert!(
                is_media_path(Path::new(path)),
                "expected {path} to be media"
            );
        }
        for path in ["image.svg", "notes.pdf", "installer.exe", "unknown"] {
            assert!(
                !is_media_path(Path::new(path)),
                "expected {path} to be blocked"
            );
        }
    }

    #[test]
    fn records_incomplete_file_targets() {
        let content = content("downloads/movie.mkv", None, &[("movie.mkv", 0.75)]);
        assert!(!resolve_target(&content, None).unwrap().progress_complete);
    }

    #[test]
    fn rejects_relative_content_paths() {
        let mut content = content("downloads/movie.mkv", None, &[("movie.mkv", 1.0)]);
        content.content_path = PathBuf::from("downloads/movie.mkv");

        assert_eq!(
            resolve_target(&content, None).unwrap_err(),
            "qBittorrent returned an invalid content path."
        );
    }

    #[test]
    fn showing_a_single_file_torrent_uses_its_parent_folder() {
        let file = absolute_path("downloads/movie.mkv");

        assert_eq!(
            opener_path(&file, ContentAction::Reveal, TargetKind::File, false).unwrap(),
            file.parent().unwrap()
        );
    }

    #[test]
    fn showing_an_individual_file_still_targets_the_file_for_selection() {
        let file = absolute_path("downloads/movie.mkv");

        assert_eq!(
            opener_path(&file, ContentAction::Reveal, TargetKind::File, true).unwrap(),
            file
        );
    }
}
