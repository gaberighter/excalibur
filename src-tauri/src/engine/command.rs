//! Finds the executable a manifest's `command` refers to (D38, D47).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Folders GUI apps do not get on their `PATH` but plugins commonly need.
const COMMON_DIRS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];

/// The app's own `PATH`, then the common folders, then the user's extras.
pub fn search_path(extra: &[PathBuf]) -> Vec<PathBuf> {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = std::env::split_paths(&inherited).collect();
    let more = COMMON_DIRS
        .iter()
        .map(PathBuf::from)
        .chain(extra.iter().map(|dir| expand_home(dir, home.as_deref())));
    for dir in more {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

pub fn join_path(dirs: &[PathBuf]) -> OsString {
    std::env::join_paths(dirs).unwrap_or_default()
}

fn expand_home(dir: &Path, home: Option<&Path>) -> PathBuf {
    match (dir.strip_prefix("~"), home) {
        (Ok(rest), Some(home)) => home.join(rest),
        _ => dir.to_path_buf(),
    }
}

/// A bare name is looked up in the plugin's folder, then beside the bundled
/// helper, then on the search path. A name with a `/` is a path, relative to
/// the plugin's folder.
pub fn resolve(
    program: &str,
    plugin_dir: &Path,
    helper_dir: Option<&Path>,
    search_path: &[PathBuf],
) -> Option<PathBuf> {
    if program.contains('/') {
        let path = plugin_dir.join(program);
        return is_executable(&path).then_some(path);
    }
    [plugin_dir]
        .into_iter()
        .chain(helper_dir)
        .chain(search_path.iter().map(PathBuf::as_path))
        .map(|dir| dir.join(program))
        .find(|path| is_executable(path))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn add_executable(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn plugin_folder_wins_over_helper_and_path() {
        let root = tempfile::tempdir().unwrap();
        let (plugin, helper, bin) = (
            root.path().join("plugin"),
            root.path().join("helper"),
            root.path().join("bin"),
        );
        let expected = add_executable(&plugin, "tool");
        add_executable(&helper, "tool");
        add_executable(&bin, "tool");
        assert_eq!(
            resolve("tool", &plugin, Some(&helper), &[bin]),
            Some(expected)
        );
    }

    #[test]
    fn helper_folder_wins_over_path() {
        let root = tempfile::tempdir().unwrap();
        let (plugin, helper, bin) = (
            root.path().join("plugin"),
            root.path().join("helper"),
            root.path().join("bin"),
        );
        std::fs::create_dir_all(&plugin).unwrap();
        let expected = add_executable(&helper, "tool");
        add_executable(&bin, "tool");
        assert_eq!(
            resolve("tool", &plugin, Some(&helper), &[bin]),
            Some(expected)
        );
    }

    #[test]
    fn search_path_is_used_last() {
        let root = tempfile::tempdir().unwrap();
        let (plugin, bin) = (root.path().join("plugin"), root.path().join("bin"));
        std::fs::create_dir_all(&plugin).unwrap();
        let expected = add_executable(&bin, "tool");
        assert_eq!(resolve("tool", &plugin, None, &[bin]), Some(expected));
    }

    #[test]
    fn missing_command_is_none() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(resolve("tool", root.path(), None, &[]), None);
    }

    #[test]
    fn non_executable_file_is_not_a_command() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("tool"), "data").unwrap();
        assert_eq!(resolve("tool", root.path(), None, &[]), None);
    }

    #[test]
    fn relative_path_is_resolved_against_the_plugin_folder() {
        let root = tempfile::tempdir().unwrap();
        let expected = add_executable(&root.path().join("bin"), "tool");
        let resolved = resolve("./bin/tool", root.path(), None, &[]).unwrap();
        assert_eq!(
            resolved.canonicalize().unwrap(),
            expected.canonicalize().unwrap()
        );
    }

    #[test]
    fn search_path_adds_common_and_extra_folders() {
        let dirs = search_path(&[PathBuf::from("/opt/tools/bin")]);
        assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
        assert_eq!(dirs.last(), Some(&PathBuf::from("/opt/tools/bin")));
    }

    #[test]
    fn tilde_expands_to_home() {
        let home = Path::new("/Users/someone");
        assert_eq!(
            expand_home(Path::new("~/bin"), Some(home)),
            PathBuf::from("/Users/someone/bin")
        );
        assert_eq!(
            expand_home(Path::new("/usr/bin"), Some(home)),
            PathBuf::from("/usr/bin")
        );
    }
}
