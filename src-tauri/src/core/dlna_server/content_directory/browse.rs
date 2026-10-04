//! ContentDirectory `Browse()` action handler.
//!
//! Receives parsed SOAP request params and returns a DIDL-Lite payload
//! plus the counts the SOAP layer needs (NumberReturned, TotalMatches,
//! UpdateID).
//!
//! Hierarchy is documented in [`super::object_id`].

use std::sync::Arc;

use crate::core::dlna_server::content_directory::didl::DidlBuilder;
use crate::core::dlna_server::content_directory::object_id::{encode_path, parse_object_id, ObjectId};
use crate::core::dlna_server::error::DlnaError;
use crate::core::dlna_server::library::provider::LibraryProvider;

/// Parsed parameters of a Browse SOAP call.
#[derive(Debug, Clone)]
pub struct BrowseRequest {
    pub object_id: String,
    pub browse_flag: BrowseFlag,
    pub starting_index: u32,
    pub requested_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseFlag {
    /// Return the metadata of the object itself.
    BrowseMetadata,
    /// Return the children of the object (the common case).
    BrowseDirectChildren,
}

/// Result returned to the SOAP layer for further encoding.
#[derive(Debug)]
pub struct BrowseResponse {
    pub didl_xml: String,
    pub number_returned: u32,
    pub total_matches: u32,
    pub update_id: u32,
}

/// Dispatch a Browse request to the appropriate hierarchy walker.
pub async fn handle_browse(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    request: &BrowseRequest,
) -> Result<BrowseResponse, DlnaError> {
    if request.browse_flag == BrowseFlag::BrowseMetadata {
        let langue = provider.language().await;
        return Ok(metadata_for(&request.object_id, server_base_url, &langue));
    }

    match parse_object_id(&request.object_id) {
        ObjectId::Root => {
            let langue = provider.language().await;
            browse_root(provider, server_base_url, &langue).await
        }
        ObjectId::Library(lib) => Ok(browse_library_root(lib, server_base_url, &provider.language().await)),
        ObjectId::Artists(lib) => browse_artists(provider, server_base_url, lib, request).await,
        ObjectId::Artist(lib, id) => {
            browse_artist_albums(provider, server_base_url, lib, &id, request).await
        }
        ObjectId::Albums(lib) => browse_albums(provider, server_base_url, lib, request).await,
        ObjectId::Album(lib, id) => {
            browse_album_tracks(provider, server_base_url, lib, &id, request).await
        }
        ObjectId::Folders(lib) => {
            browse_root_folders(provider, server_base_url, lib, request).await
        }
        ObjectId::Folder(lib, path) => {
            browse_folder_entries(provider, server_base_url, lib, &path, request).await
        }
        ObjectId::Unknown => Ok(empty_response(server_base_url)),
    }
}

// ─── Static / fixed-shape responses ───────────────────────────────────

/// Browse the absolute root.
/// - 0 libraries  → empty
/// - 1 library    → auto-collapse : return that library's Artists/Albums/Folders
/// - N libraries  → one container per library
async fn browse_root(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    langue: &str,
) -> Result<BrowseResponse, DlnaError> {
    let libs = provider.list_libraries().await?;

    let mut didl = DidlBuilder::new(server_base_url.to_string());

    if libs.len() == 1 {
        emit_library_children(&mut didl, libs[0].id, "0", langue);
        return Ok(fixed_response(didl.build(), 3));
    }

    for lib in &libs {
        didl.add_library(lib, "0");
    }
    Ok(fixed_response(didl.build(), libs.len() as u32))
}

/// Browse a specific library's root : Artists / Albums / Folders.
fn browse_library_root(library_id: i64, server_base_url: &str, langue: &str) -> BrowseResponse {
    let parent = format!("0/lib/{}", library_id);
    let mut didl = DidlBuilder::new(server_base_url.to_string());
    emit_library_children(&mut didl, library_id, &parent, langue);
    fixed_response(didl.build(), 3)
}

/// Helper : emit Artists / Albums / Folders sub-containers for a given library
/// under `parent_id`. Used for both library roots and the auto-collapsed root.
fn emit_library_children(didl: &mut DidlBuilder, library_id: i64, parent_id: &str, langue: &str) {
    didl.add_container(
        &format!("0/lib/{}/artists", library_id),
        parent_id,
        titre(Menu::Artistes, langue),
        0,
        "object.container",
    );
    didl.add_container(
        &format!("0/lib/{}/albums", library_id),
        parent_id,
        titre(Menu::Albums, langue),
        0,
        "object.container",
    );
    didl.add_container(
        &format!("0/lib/{}/folders", library_id),
        parent_id,
        titre(Menu::Dossiers, langue),
        0,
        "object.container.storageFolder",
    );
}

#[derive(Clone, Copy)]
enum Menu {
    Artistes,
    Albums,
    Dossiers,
}

/// Titre d'un menu dans la langue de l'app ; anglais pour une langue inconnue.
fn titre(menu: Menu, langue: &str) -> &'static str {
    match (menu, langue) {
        (Menu::Artistes, "fr") => "Artistes",
        (Menu::Artistes, "es") => "Artistas",
        (Menu::Artistes, "de") => "Künstler",
        (Menu::Artistes, "it") => "Artisti",
        (Menu::Artistes, _) => "Artists",
        (Menu::Albums, "es") => "Álbumes",
        (Menu::Albums, "de") => "Alben",
        (Menu::Albums, "it") => "Album",
        (Menu::Albums, _) => "Albums",
        (Menu::Dossiers, "fr") => "Dossiers",
        (Menu::Dossiers, "es") => "Carpetas",
        (Menu::Dossiers, "de") => "Ordner",
        (Menu::Dossiers, "it") => "Cartelle",
        (Menu::Dossiers, _) => "Folders",
    }
}

// ─── Paginated browsers ───────────────────────────────────────────────

async fn browse_artists(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    library_id: i64,
    request: &BrowseRequest,
) -> Result<BrowseResponse, DlnaError> {
    let artists = provider.list_artists(library_id).await?;
    let parent_id = format!("0/lib/{}/artists", library_id);
    Ok(paginated(server_base_url, &artists, request, |didl, a| {
        didl.add_artist(library_id, a, &parent_id);
    }))
}

async fn browse_artist_albums(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    library_id: i64,
    artist_id: &str,
    request: &BrowseRequest,
) -> Result<BrowseResponse, DlnaError> {
    let albums = provider
        .list_albums_by_artist(library_id, artist_id)
        .await?;
    let parent_id = format!("0/lib/{}/artists/{}", library_id, artist_id);
    Ok(paginated(server_base_url, &albums, request, |didl, a| {
        didl.add_album(library_id, a, &parent_id);
    }))
}

async fn browse_albums(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    library_id: i64,
    request: &BrowseRequest,
) -> Result<BrowseResponse, DlnaError> {
    let albums = provider.list_albums(library_id).await?;
    let parent_id = format!("0/lib/{}/albums", library_id);
    Ok(paginated(server_base_url, &albums, request, |didl, a| {
        didl.add_album(library_id, a, &parent_id);
    }))
}

async fn browse_album_tracks(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    library_id: i64,
    album_id: &str,
    request: &BrowseRequest,
) -> Result<BrowseResponse, DlnaError> {
    let tracks = provider.list_tracks_by_album(library_id, album_id).await?;
    let parent_id = format!("0/lib/{}/albums/{}", library_id, album_id);
    Ok(paginated(server_base_url, &tracks, request, |didl, t| {
        didl.add_track(t, &parent_id);
    }))
}

async fn browse_root_folders(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    library_id: i64,
    request: &BrowseRequest,
) -> Result<BrowseResponse, DlnaError> {
    let folders = provider.list_root_folders(library_id).await?;
    let parent_id = format!("0/lib/{}/folders", library_id);
    Ok(paginated(server_base_url, &folders, request, |didl, f| {
        didl.add_folder(library_id, &f.name, &f.path, &parent_id);
    }))
}

/// Folder entries : subfolders + tracks treated as one logical stream for
/// pagination purposes (folders first, then tracks). Doesn't fit the simple
/// `paginated()` helper because items are heterogeneous.
async fn browse_folder_entries(
    provider: Arc<dyn LibraryProvider>,
    server_base_url: &str,
    library_id: i64,
    folder_path: &str,
    request: &BrowseRequest,
) -> Result<BrowseResponse, DlnaError> {
    let entries = provider.list_folder_entries(library_id, folder_path).await?;
    let parent_id = format!("0/lib/{}/folders/{}", library_id, encode_path(folder_path));

    let total = (entries.subfolders.len() + entries.tracks.len()) as u32;
    let start = request.starting_index as usize;
    let limit = effective_limit(request.requested_count);

    let mut didl = DidlBuilder::new(server_base_url.to_string());
    let mut emitted = 0u32;
    let mut idx = 0usize;

    for f in &entries.subfolders {
        if idx >= start && (emitted as usize) < limit {
            didl.add_folder(library_id, &f.name, &f.path, &parent_id);
            emitted += 1;
        }
        idx += 1;
    }
    for t in &entries.tracks {
        if idx >= start && (emitted as usize) < limit {
            didl.add_track(t, &parent_id);
            emitted += 1;
        }
        idx += 1;
    }

    Ok(BrowseResponse {
        didl_xml: didl.build(),
        number_returned: emitted,
        total_matches: total,
        update_id: 1,
    })
}

// ─── Pagination helpers ───────────────────────────────────────────────

/// Run the standard `list → paginate → render → BrowseResponse` flow.
/// `render` is called once per item with a mutable reference to the DIDL
/// builder so each browser only has to say *how* to render its kind.
fn paginated<T>(
    server_base_url: &str,
    items: &[T],
    request: &BrowseRequest,
    mut render: impl FnMut(&mut DidlBuilder, &T),
) -> BrowseResponse {
    let total = items.len() as u32;
    let slice = paginate(items, request.starting_index, request.requested_count);

    let mut didl = DidlBuilder::new(server_base_url.to_string());
    for item in slice {
        render(&mut didl, item);
    }
    BrowseResponse {
        didl_xml: didl.build(),
        number_returned: slice.len() as u32,
        total_matches: total,
        update_id: 1,
    }
}

/// Slice `items` according to DLNA pagination params. `count == 0` means "all".
fn paginate<T>(items: &[T], start: u32, count: u32) -> &[T] {
    let start = start as usize;
    if start >= items.len() {
        return &[];
    }
    let end = (start + effective_limit(count)).min(items.len());
    &items[start..end]
}

/// Translate the SOAP `RequestedCount` into a usable usize limit
/// (0 in the SOAP body means "no limit").
fn effective_limit(count: u32) -> usize {
    if count == 0 {
        usize::MAX
    } else {
        count as usize
    }
}

// ─── Misc response builders ───────────────────────────────────────────

fn fixed_response(didl_xml: String, n: u32) -> BrowseResponse {
    BrowseResponse {
        didl_xml,
        number_returned: n,
        total_matches: n,
        update_id: 1,
    }
}

fn empty_response(server_base_url: &str) -> BrowseResponse {
    BrowseResponse {
        didl_xml: DidlBuilder::new(server_base_url.to_string()).build(),
        number_returned: 0,
        total_matches: 0,
        update_id: 1,
    }
}

/// Minimal `BrowseMetadata` response : describe the queried object as a
/// generic container. Sufficient for most amps' probe step.
fn metadata_for(object_id: &str, server_base_url: &str, langue: &str) -> BrowseResponse {
    let title = if object_id == "0" || object_id.is_empty() {
        "RustMusic"
    } else if object_id.ends_with("/artists") {
        titre(Menu::Artistes, langue)
    } else if object_id.ends_with("/albums") {
        titre(Menu::Albums, langue)
    } else if object_id.ends_with("/folders") {
        titre(Menu::Dossiers, langue)
    } else {
        "Container"
    };
    let parent = if object_id == "0" || object_id.is_empty() {
        "-1".to_string()
    } else if let Some((p, _)) = object_id.rsplit_once('/') {
        p.to_string()
    } else {
        "0".to_string()
    };
    let mut didl = DidlBuilder::new(server_base_url.to_string());
    didl.add_container(object_id, &parent, title, 0, "object.container");
    fixed_response(didl.build(), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menus_dans_la_langue_de_l_app() {
        assert_eq!(titre(Menu::Artistes, "fr"), "Artistes");
        assert_eq!(titre(Menu::Dossiers, "en"), "Folders");
        assert_eq!(titre(Menu::Albums, "de"), "Alben");
        // Langue inconnue : anglais.
        assert_eq!(titre(Menu::Dossiers, "pt"), "Folders");
    }
}
