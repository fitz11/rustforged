//! The `lib://` asset source.
//!
//! Placed items refer to library images by library-relative path ("tokens/orc.png"), both at
//! runtime and in map files. Bevy loads them through this source, which resolves each path
//! against whichever library is currently open. Switching libraries only swaps the root.

use bevy::asset::AssetPath;
use bevy::asset::io::VecReader;
use bevy::asset::io::{AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, Reader};
use bevy::prelude::*;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Name of the asset source that serves library images.
pub const LIBRARY_SOURCE: &str = "lib";

/// Shared handle to the open library's root directory.
///
/// Cloned into the asset reader at startup; readers resolve paths against it at read time.
#[derive(Resource, Clone, Default)]
pub struct LibraryRoot(Arc<RwLock<PathBuf>>);

impl LibraryRoot {
    pub fn get(&self) -> PathBuf {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set(&self, path: PathBuf) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = path;
    }

    /// Full filesystem path for a library-relative path, or `None` if the path would
    /// escape the library (absolute, `..`, drive prefixes). Map files arrive from other
    /// machines, so their references are not trusted to stay inside the library.
    pub fn resolve(&self, path: &Path) -> Option<PathBuf> {
        let contained = path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
        contained.then(|| self.get().join(path))
    }
}

/// Asset path for a library-relative path, e.g. "tokens/orc.png" -> `lib://tokens/orc.png`.
///
/// Built from a `PathBuf` rather than parsed, so filenames containing `#` aren't mistaken
/// for asset labels.
pub fn library_asset_path(relative: &str) -> AssetPath<'static> {
    AssetPath::from_path_buf(PathBuf::from(relative)).with_source(LIBRARY_SOURCE)
}

struct LibraryAssetReader {
    root: LibraryRoot,
}

impl AssetReader for LibraryAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let full_path = self
            .root
            .resolve(path)
            .ok_or_else(|| AssetReaderError::NotFound(path.to_path_buf()))?;
        match std::fs::read(&full_path) {
            Ok(bytes) => Ok(VecReader::new(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(AssetReaderError::NotFound(full_path))
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        // Library images never carry .meta files; Bevy falls back to default loader settings.
        Err::<VecReader, _>(AssetReaderError::NotFound(path.to_path_buf()))
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        // The library is scanned by AssetLibrary, never through the asset server.
        Err(AssetReaderError::NotFound(path.to_path_buf()))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(self.root.resolve(path).is_some_and(|p| p.is_dir()))
    }
}

/// Registers the `lib://` source and the [`LibraryRoot`] resource it reads from.
///
/// Must run before `DefaultPlugins`: Bevy builds asset sources when `AssetPlugin` is added.
pub fn register_library_source(app: &mut App) {
    let root = LibraryRoot::default();
    let reader_root = root.clone();
    app.register_asset_source(
        LIBRARY_SOURCE,
        AssetSourceBuilder::new(move || {
            Box::new(LibraryAssetReader {
                root: reader_root.clone(),
            })
        }),
    );
    app.insert_resource(root);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_joins_relative_paths() {
        let root = LibraryRoot::default();
        root.set(PathBuf::from("/libs/main"));
        assert_eq!(
            root.resolve(Path::new("tokens/orc.png")),
            Some(PathBuf::from("/libs/main/tokens/orc.png"))
        );
    }

    #[test]
    fn test_resolve_rejects_escaping_paths() {
        let root = LibraryRoot::default();
        root.set(PathBuf::from("/libs/main"));
        assert_eq!(root.resolve(Path::new("../secret.png")), None);
        assert_eq!(root.resolve(Path::new("tokens/../../secret.png")), None);
        assert_eq!(root.resolve(Path::new("/etc/passwd")), None);
    }

    #[test]
    fn test_library_asset_path_keeps_hash_in_filename() {
        let path = library_asset_path("tokens/goblin #2.png");
        assert_eq!(path.path(), Path::new("tokens/goblin #2.png"));
        assert!(path.label().is_none());
        assert_eq!(path.source().as_str(), Some(LIBRARY_SOURCE));
    }

    /// Plain-text asset so the source can be exercised without the render stack.
    #[derive(Asset, TypePath)]
    struct TextFile(String);

    #[derive(TypePath, Default)]
    struct TextLoader;

    impl bevy::asset::AssetLoader for TextLoader {
        type Asset = TextFile;
        type Settings = ();
        type Error = std::io::Error;

        async fn load(
            &self,
            reader: &mut dyn Reader,
            _settings: &(),
            _load_context: &mut bevy::asset::LoadContext<'_>,
        ) -> Result<TextFile, std::io::Error> {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            Ok(TextFile(String::from_utf8_lossy(&bytes).into_owned()))
        }

        fn extensions(&self) -> &[&str] {
            &["txt"]
        }
    }

    fn text_app() -> App {
        let mut app = App::new();
        register_library_source(&mut app);
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<TextFile>()
            .register_asset_loader(TextLoader);
        app
    }

    /// Runs the app until `done` holds, failing the test after a few seconds.
    fn update_until(app: &mut App, mut done: impl FnMut(&mut App) -> bool) {
        for _ in 0..500 {
            app.update();
            if done(app) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("condition not reached");
    }

    fn text(app: &App, handle: &Handle<TextFile>) -> Option<String> {
        app.world()
            .resource::<Assets<TextFile>>()
            .get(handle)
            .map(|t| t.0.clone())
    }

    #[test]
    fn test_lib_source_follows_library_root() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for (dir, content) in [(&first, "first"), (&second, "second")] {
            std::fs::create_dir_all(dir.path().join("notes")).unwrap();
            std::fs::write(dir.path().join("notes/a #1.txt"), content).unwrap();
        }

        let mut app = text_app();
        let root = app.world().resource::<LibraryRoot>().clone();
        root.set(first.path().to_path_buf());

        let handle: Handle<TextFile> = app
            .world()
            .resource::<AssetServer>()
            .load(library_asset_path("notes/a #1.txt"));
        update_until(&mut app, |app| {
            text(app, &handle).as_deref() == Some("first")
        });

        root.set(second.path().to_path_buf());
        app.world()
            .resource::<AssetServer>()
            .reload(library_asset_path("notes/a #1.txt"));
        update_until(&mut app, |app| {
            text(app, &handle).as_deref() == Some("second")
        });
    }

    #[test]
    fn test_lib_source_missing_file_fails_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = text_app();
        app.world()
            .resource::<LibraryRoot>()
            .set(dir.path().to_path_buf());

        let handle: Handle<TextFile> = app
            .world()
            .resource::<AssetServer>()
            .load(library_asset_path("notes/missing.txt"));
        update_until(&mut app, |app| {
            matches!(
                app.world().resource::<AssetServer>().load_state(&handle),
                bevy::asset::LoadState::Failed(_)
            )
        });
    }
}
