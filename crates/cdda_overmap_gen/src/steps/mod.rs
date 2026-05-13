//! Generation step systems.
//!
//! Each module exports one or more systems that participate in the
//! generation pipeline. Systems are registered in `OvermapGenPlugin`.

pub mod init_base;
pub mod forests;
pub mod lakes;
pub mod oceans;
pub mod swamps;
pub mod ravines;
pub mod rivers;
pub mod cities;
pub mod build_cities;
pub mod roads;
pub mod railroads;
pub mod forest_trails;
pub mod city_buildings;
pub mod specials;
pub mod mutable_specials;
pub mod finalize;
pub mod mongroups;
pub mod radios;
pub mod subway;
pub mod elevated;
pub mod highway;
pub mod highway_interchanges;
pub mod forest_trailheads;
pub mod stubs;

pub use init_base::init_base_terrain;
pub use forests::{place_forests, calculate_forestosity};
pub use lakes::place_lakes;
pub use oceans::{place_oceans, calculate_ocean_gradient};
pub use swamps::place_swamps;
pub use ravines::place_ravines;
pub use cities::{place_cities, calculate_urbanity, CityTiles};
pub use build_cities::build_cities;
pub use roads::place_roads;
pub use railroads::place_railroads;
pub use forest_trails::place_forest_trails;
pub use city_buildings::place_city_buildings;
pub use specials::{place_specials, PlacedSpecial};
pub use mutable_specials::{place_mutable_specials, PlacedMutableSpecial};
pub use rivers::{place_rivers, build_river_shores, polish_river};
pub use finalize::finalize_overmap;
pub use mongroups::{place_mongroups, MonsterGroup};
pub use radios::{place_radios, RadioTower};
pub use subway::generate_sub;
pub use elevated::generate_over;
pub use highway::place_highways;
pub use highway_interchanges::{place_highway_interchanges, finalize_highways};
pub use forest_trailheads::place_forest_trailheads;
pub use stubs::*;
