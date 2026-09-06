//! Filesystem adapter for presentation options; UI never performs disk I/O.
use bevy_ecs::prelude::*;
use cdda_components::progress::{OperationReport, ReportEvent, ReportLevel};
use cdda_render::render::settings::{DisplayPreferences, SettingsState};
use std::path::PathBuf;

#[derive(Resource)]
pub struct PreferencesFile {
    pub path: PathBuf,
    saved: DisplayPreferences,
    writable: bool,
}

pub fn load_preferences(world: &mut World) {
    let root = std::env::var_os("CDDA_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config"));
    load_preferences_at(world, root.join("interface.json"));
}

fn load_preferences_at(world: &mut World, path: PathBuf) {
    let mut writable = true;
    match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<DisplayPreferences>(&bytes) {
            Ok(preferences) => preferences.apply(&mut world.resource_mut::<SettingsState>()),
            Err(error) => {
                writable = false; // Preserve malformed user data for repair.
                crate::loading::publish_report(
                    world,
                    ReportEvent::progress(
                        "Settings",
                        format!(
                            "{}: {error}. Using defaults; file preserved.",
                            path.display()
                        ),
                    )
                    .level(ReportLevel::Warning),
                );
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            writable = false;
            crate::loading::publish_report(
                world,
                ReportEvent::progress("Settings", format!("{}: {error}", path.display()))
                    .level(ReportLevel::Warning),
            );
        }
    }
    let saved = DisplayPreferences::from(world.resource::<SettingsState>());
    world.insert_resource(PreferencesFile {
        path,
        saved,
        writable,
    });
}

pub fn save_preferences(
    state: Res<SettingsState>,
    file: Option<ResMut<PreferencesFile>>,
    mut report: ResMut<OperationReport>,
) {
    let Some(mut file) = file else {
        return;
    };
    if !state.is_changed() {
        return;
    }
    let preferences = DisplayPreferences::from(&*state);
    if file.saved == preferences {
        return;
    }
    let result = if file.writable {
        save(&file.path, &preferences)
    } else {
        Err(std::io::Error::other(
            "Settings file could not be read; changes apply only to this session",
        ))
    };
    let event = match result {
        Ok(()) => ReportEvent::progress("Settings", "Display preferences saved")
            .level(ReportLevel::Complete),
        Err(error) => {
            ReportEvent::progress("Settings", error.to_string()).level(ReportLevel::Warning)
        }
    };
    eprintln!("{event}");
    report.record(event);
    // Do not retry filesystem failures on every focus movement.
    file.saved = preferences;
}

fn save(path: &std::path::Path, preferences: &DisplayPreferences) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(preferences)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_roundtrip_and_invalid_files_are_preserved() {
        let directory = std::env::temp_dir().join(format!(
            "cdda-preferences-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("interface.json");
        let preferences = DisplayPreferences {
            theme: 2,
            scale_percent: 120,
            fullscreen: true,
            menu_art: false,
        };
        save(&path, &preferences).unwrap();
        let mut world = World::new();
        world.init_resource::<SettingsState>();
        world.init_resource::<OperationReport>();
        load_preferences_at(&mut world, path.clone());
        assert_eq!(
            DisplayPreferences::from(world.resource::<SettingsState>()),
            preferences
        );
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::write(&path, "broken configuration").unwrap();
        load_preferences_at(&mut world, path.clone());
        assert!(!world.resource::<PreferencesFile>().writable);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "broken configuration"
        );
        assert_eq!(world.resource::<OperationReport>().warnings, 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
