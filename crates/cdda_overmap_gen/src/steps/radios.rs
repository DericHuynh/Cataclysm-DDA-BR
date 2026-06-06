//! Radio tower and radio message placement.
//!
//! Port of C++ `overmap::place_radios()` (overmap.cpp L3666-3697).
//!
//! Algorithm:
//! 1. Scan every OMT tile at z=0.
//! 2. For radio_tower tiles: 1-in-3 weather radio, 2-in-3 archive message.
//! 3. For lmoe tiles: emergency shelter beacon.
//! 4. For fema_entrance tiles: FEMA camp message.
//! 5. Each radio has a random signal strength.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::direction::Rng;
use cdda_overmap::query::{is_ot_match, OtMatchType};
use cdda_overmap::registry::TerrainRegistry;
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;

// ---------------------------------------------------------------------------
// RadioTower marker component
// ---------------------------------------------------------------------------

/// A radio tower or beacon placed on the overmap.
#[derive(Component)]
pub struct RadioTower {
    /// Signal strength (typical range: 40-120).
    pub strength: i32,
    /// The broadcast message content.
    pub message: String,
    /// OMT x-coordinate.
    pub omt_x: i32,
    /// OMT y-coordinate.
    pub omt_y: i32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum radio signal strength (for towers).
const RADIO_MIN_STRENGTH: i32 = 80;
/// Maximum radio signal strength (for towers).
const RADIO_MAX_STRENGTH: i32 = 120;
/// Minimum signal strength for emergency beacons.
const BEACON_MIN_STRENGTH: i32 = 40;
/// Maximum signal strength for emergency beacons.
const BEACON_MAX_STRENGTH: i32 = 60;

// ---------------------------------------------------------------------------
// Radio messages
// ---------------------------------------------------------------------------

/// Generate a weather radio message.
fn weather_radio_message(rng: &mut XorShiftRng) -> String {
    let messages = [
        "This is the National Weather Service. Partly cloudy with a chance of scattered showers.",
        "Weather alert: severe thunderstorms expected in the area. Seek shelter immediately.",
        "National Weather Service broadcast. Clear skies expected for the next 48 hours.",
        "Emergency weather broadcast: acid rain detected in surrounding areas. Avoid exposure.",
        "Weather service automated message. Temperature: unpredictable. Radiation: variable.",
    ];
    messages[rng.random_usize(messages.len())].into()
}

/// Generate a radio archive message.
fn radio_archive_message(rng: &mut XorShiftRng) -> String {
    let messages = [
        "This is an automated broadcast. The following is a pre-recorded message from the emergency broadcast system.",
        "Repeating: all citizens are advised to remain indoors. Do not attempt to travel.",
        "Archive recording: government officials report the situation is under control. Repeat: under control.",
        "Looping message: this is not a test. The emergency broadcast system has been activated.",
        "Recorded message: military checkpoints have been established. Cooperate with all personnel.",
        "Archive broadcast: report any unusual activity to your local authorities immediately.",
    ];
    messages[rng.random_usize(messages.len())].into()
}

/// Generate an automated emergency shelter beacon message.
fn shelter_beacon_message(_rng: &mut XorShiftRng) -> String {
    "This is an automated emergency shelter beacon. Assistance is available at this location. \
     Repeat: assistance is available. Food, water, and medical supplies are stored on-site."
        .into()
}

/// Generate a FEMA camp message.
fn fema_message(_rng: &mut XorShiftRng) -> String {
    "This is a FEMA emergency camp broadcast. Refugees are directed to proceed to the nearest \
     FEMA camp for processing and relocation. This message will repeat."
        .into()
}

// ---------------------------------------------------------------------------
// place_radios — system entry point
// ---------------------------------------------------------------------------

/// Place radio towers and message beacons on the overmap.
///
/// Port of C++ `overmap::place_radios()` (overmap.cpp L3666-3697).
pub fn place_radios(
    mut commands: Commands,
    chunks: Query<(&ChunkPosition, &OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 37);

    let mut radio_towers = 0usize;
    let mut lmoe_beacons = 0usize;
    let mut fema_beacons = 0usize;

    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as i32 {
            for lx in 0..CHUNK_DIM as i32 {
                let gx = ox + lx;
                let gy = oy + ly;
                if gx < 0 || gx >= OMAP_DIM || gy < 0 || gy >= OMAP_DIM {
                    continue;
                }

                let handle = chunk.get(lx as u8, ly as u8);

                // --- Radio tower check ------------------------------------
                if is_ot_match("radio_tower", handle, &registry, OtMatchType::Prefix) {
                    let (strength, message) = if rng.one_in(3) {
                        // Weather radio
                        (
                            rng.range_i32(RADIO_MIN_STRENGTH, RADIO_MAX_STRENGTH),
                            weather_radio_message(&mut rng),
                        )
                    } else {
                        // Radio archive message
                        (
                            rng.range_i32(RADIO_MIN_STRENGTH, RADIO_MAX_STRENGTH),
                            radio_archive_message(&mut rng),
                        )
                    };

                    commands.spawn(RadioTower {
                        strength,
                        message,
                        omt_x: gx,
                        omt_y: gy,
                    });
                    radio_towers += 1;
                }

                // --- LMOE shelter check -----------------------------------
                if is_ot_match("lmoe", handle, &registry, OtMatchType::Prefix) {
                    let strength = rng.range_i32(BEACON_MIN_STRENGTH, BEACON_MAX_STRENGTH);
                    let message = shelter_beacon_message(&mut rng);

                    commands.spawn(RadioTower {
                        strength,
                        message,
                        omt_x: gx,
                        omt_y: gy,
                    });
                    lmoe_beacons += 1;
                }

                // --- FEMA entrance check ----------------------------------
                if is_ot_match("fema_entrance", handle, &registry, OtMatchType::Prefix) {
                    let strength = rng.range_i32(RADIO_MIN_STRENGTH, RADIO_MAX_STRENGTH);
                    let message = fema_message(&mut rng);

                    commands.spawn(RadioTower {
                        strength,
                        message,
                        omt_x: gx,
                        omt_y: gy,
                    });
                    fema_beacons += 1;
                }
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        radio_towers,
        lmoe_beacons,
        fema_beacons,
        "place_radios: complete"
    );
}
