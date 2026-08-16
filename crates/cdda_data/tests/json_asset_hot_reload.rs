//! End-to-end hot-reload test for the Bevy asset pipeline.
//!
//! Proves the actual reload loop the game relies on:
//!
//! 1. Existing CDDA JSON (a "file") is loaded as a [`CddaJsonFile`] asset.
//! 2. That file's content changes (new definitions added/removed).
//! 3. The new JSON is automatically re-parsed by the asset server and the new
//!    [`CddaJsonFile`] is observable through `Assets<CddaJsonFile>`.
//! 4. Feeding the re-parsed values through [`Loader`] (`ingest_values` +
//!    `resolve`) produces a registry that reflects the new definitions — the
//!    exact seam `cdda_app::data_assets::reload_modified_data` uses to rebuild
//!    the def-world.
//!
//! The test drives reload through a Bevy [`AssetWatcher`] we own, mirroring the
//! exact pattern Bevy uses in its own asset hot-reload tests
//! (`bevy_asset`'s `create_app_with_source_event_sender`): register a custom
//! asset source backed by [`MemoryAssetReader`], capture the watcher's event
//! sender, replace the "file", then emit [`AssetSourceEvent::ModifiedAsset`].
//! This keeps the test deterministic (no filesystem debounce timing) while
//! exercising the genuine asset-server re-read → re-parse → registry path.

use std::path::{Path, PathBuf};

use async_channel::Sender;
use bevy_app::{App, TaskPoolPlugin};
use bevy_asset::io::memory::Dir;
use bevy_asset::io::memory::MemoryAssetReader;
use bevy_asset::io::{AssetSourceBuilder, AssetSourceEvent, AssetSourceId, AssetWatcher};
use bevy_asset::{AssetApp, AssetEvent, AssetPlugin, AssetServer, Assets, Handle};
use bevy_ecs::message::Messages;
use cdda_core_types::core::id::DefId;
use cdda_data::json_asset::{CddaJsonFile, CddaJsonFileLoader};
use cdda_data::Loader;
use serde_json::Value;

// ---------------------------------------------------------------------------
// A watcher that hands its event sender back so the test can signal reloads.
// ---------------------------------------------------------------------------

struct TestWatcher;

impl AssetWatcher for TestWatcher {}

// ---------------------------------------------------------------------------
// Harness: a real App whose `cdda` asset source is an in-memory `Dir`.
// ---------------------------------------------------------------------------

/// The "file" inside the `cdda` asset source that we mutate.
const FILE_PATH: &str = "core/items.json";

/// Builds an `App` that reads the `cdda` source from `dir`, loads
/// `FILE_PATH` as a [`CddaJsonFile`], and returns the event sender so the test
/// can emit `ModifiedAsset` to trigger a reload.
fn build_harness(
    initial_content: &str,
) -> (App, Dir, Sender<AssetSourceEvent>, Handle<CddaJsonFile>) {
    let dir = Dir::new(PathBuf::from("cdda_test"));
    dir.insert_asset_text(Path::new(FILE_PATH), initial_content);

    let (sender_tx, sender_rx) = async_channel::bounded(1);

    let mut app = App::new();
    let dir_for_reader = dir.clone();
    app.register_asset_source(
        AssetSourceId::Name("cdda".into()),
        AssetSourceBuilder::new(move || {
            let dir = dir_for_reader.clone();
            Box::new(MemoryAssetReader { root: dir })
        })
        .with_watcher(move |sender: async_channel::Sender<AssetSourceEvent>| {
            sender_tx
                .send_blocking(sender)
                .expect("harness sender should be sent exactly once");
            Some(Box::new(TestWatcher) as Box<dyn AssetWatcher>)
        }),
    );
    app.add_plugins((
        TaskPoolPlugin::default(),
        AssetPlugin {
            watch_for_changes_override: Some(true),
            ..Default::default()
        },
    ));
    app.init_asset::<CddaJsonFile>();
    app.init_asset_loader::<CddaJsonFileLoader>();

    let asset_path = bevy_asset::AssetPath::from_path(Path::new(FILE_PATH))
        .into_owned()
        .with_source("cdda");
    let handle: Handle<CddaJsonFile> = app.world().resource::<AssetServer>().load(asset_path);

    let sender = sender_rx
        .try_recv()
        .expect("watcher should send its channel");

    (app, dir, sender, handle)
}

/// Pump the app until `predicate` holds, mirroring `bevy_asset`'s
/// `run_app_until` helper (lets async asset I/O finish).
fn run_app_until(app: &mut App, mut predicate: impl FnMut(&App) -> bool) {
    for _ in 0..5000 {
        app.update();
        if predicate(app) {
            return;
        }
    }
    panic!("Timed out waiting for the asset condition");
}

/// The values currently parsed for `handle`, or `None` if not yet loaded.
fn parsed_values(app: &App, handle: &Handle<CddaJsonFile>) -> Option<Vec<Value>> {
    app.world()
        .resource::<Assets<CddaJsonFile>>()
        .get(handle)
        .map(|f| f.values.clone())
}

/// Resolve the asset's *current* values into a [`DefRegistry`] via the same
/// seam `cdda_app::reload_modified_data` uses.
fn resolve_from_asset(app: &App, handle: &Handle<CddaJsonFile>) -> cdda_data::DefRegistry {
    let values = parsed_values(app, handle).expect("asset should be parsed");
    let mut loader = Loader::new(Vec::new());
    loader.ingest_values(vec![(PathBuf::from(FILE_PATH), values)]);
    loader.resolve().expect("resolve registry")
}

/// True when the asset has been parsed and contains an item with `id`.
fn contains_id(app: &App, handle: &Handle<CddaJsonFile>, id: &str) -> bool {
    parsed_values(app, handle)
        .map(|v| {
            v.iter()
                .any(|d| d.get("id").and_then(|i| i.as_str()) == Some(id))
        })
        .unwrap_or(false)
}

#[test]
fn changed_json_file_automatically_reparses_and_updates_registry() {
    let initial = r##"[
      {"type":"ITEM","id":"rock","name":"rock","volume":"250 ml","weight":"100 g"},
      {"type":"ITEM","id":"stick","name":"stick","volume":"250 ml","weight":"50 g"}
    ]"##;

    let (mut app, dir, sender, handle) = build_harness(initial);

    // ---- 1. Initial load: both definitions resolve. ----
    run_app_until(&mut app, |app| contains_id(app, &handle, "rock"));
    let registry_v1 = resolve_from_asset(&app, &handle);
    assert!(registry_v1.items.contains_key(&DefId::new("rock")));
    assert!(registry_v1.items.contains_key(&DefId::new("stick")));

    // ---- 2. The file changes: stick is removed, a new candle is added. ----
    let changed = r##"[
      {"type":"ITEM","id":"rock","name":"stone","volume":"250 ml","weight":"100 g"},
      {"type":"ITEM","id":"candle","name":"candle","volume":"250 ml","weight":"20 g","flags":["FLAMMABLE"]}
    ]"##;
    dir.insert_asset_text(Path::new(FILE_PATH), changed);
    sender
        .send_blocking(AssetSourceEvent::ModifiedAsset(PathBuf::from(FILE_PATH)))
        .unwrap();

    // ---- 3. The asset server automatically re-parses the new content. ----
    run_app_until(&mut app, |app| contains_id(app, &handle, "candle"));
    let parsed = parsed_values(&app, &handle).expect("re-parsed asset");
    let ids: Vec<&str> = parsed
        .iter()
        .filter_map(|d| d.get("id").and_then(|i| i.as_str()))
        .collect();
    assert_eq!(
        ids,
        vec!["rock", "candle"],
        "asset re-parsed to new content"
    );

    // ---- 4. The registry reflects the new definitions. ----
    let registry_v2 = resolve_from_asset(&app, &handle);
    assert!(
        registry_v2.items.contains_key(&DefId::new("candle")),
        "new candle definition should resolve"
    );
    assert!(
        !registry_v2.items.contains_key(&DefId::new("stick")),
        "removed stick definition should no longer resolve"
    );
    assert_eq!(
        registry_v2.items.len(),
        2,
        "registry reflects exactly the reloaded file"
    );

    // An asset event (Added on first load / Modified on reload) must have fired.
    let had_event = app
        .world_mut()
        .resource_mut::<Messages<AssetEvent<CddaJsonFile>>>()
        .drain()
        .any(|ev| {
            matches!(
                ev,
                AssetEvent::Added { .. }
                    | AssetEvent::Modified { .. }
                    | AssetEvent::LoadedWithDependencies { .. }
            )
        });
    assert!(had_event, "the reload path should emit asset events");
}
