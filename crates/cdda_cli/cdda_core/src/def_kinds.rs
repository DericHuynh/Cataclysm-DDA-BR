/// Central registry of all CDDA definition kinds.
///
/// This is the SINGLE canonical list. When adding a new definition type,
/// add ONE line here. Everything else (DefRegistry fields, schema
/// generation, loader resolution, integration tests) auto-updates.
///
/// Each entry: (ShortName, "json_type_string")
/// where ShortName is the PascalCase Rust type prefix.
///
/// Three calling conventions:
/// - `(call $t:ident)` — item context: expands to `$t!(Item, "ITEM"); $t!(Monster, "MONSTER"); ...`
/// - `(list $mapper:ident)` — expression context: expands to `vec![$mapper!(Item, "ITEM"), $mapper!(Monster, "MONSTER"), ...]`
/// - `($mapper:ident)` — bulk context: expands to `$mapper!(Item, "ITEM", Monster, "MONSTER", ...)`
///
/// Usage examples:
/// ```ignore
/// // Item context (e.g. inside another macro)
/// macro_rules! my_helper { ($name:ident, $json:expr) => { ... }; }
/// for_each_def_kind!(call my_helper);
///
/// // Expression context (e.g. building a Vec)
/// macro_rules! entry { ($name:ident, $json:expr) => { ... }; }
/// let kinds: Vec<_> = for_each_def_kind!(list entry);
///
/// // Bulk context (e.g. building a list from a macro that takes all pairs at once)
/// macro_rules! bulk { ($($name:ident, $json:expr),*) => { ... }; }
/// for_each_def_kind!(bulk);
/// ```
#[macro_export]
macro_rules! for_each_def_kind {
    (call $t:ident) => {
        $t!(Item, "ITEM");
        $t!(Monster, "MONSTER");
        $t!(Terrain, "terrain");
        $t!(Furniture, "furniture");
        $t!(Recipe, "recipe");
        $t!(ItemGroup, "item_group");
        $t!(MapgenPalette, "palette");
        $t!(OvermapTerrain, "overmap_terrain");
        $t!(OvermapSpecial, "overmap_special");
        $t!(OvermapConnection, "overmap_connection");
        $t!(OvermapLocation, "overmap_location");
        $t!(OvermapLandUseCode, "overmap_land_use_code");
        $t!(Field, "field_type");
        $t!(VehiclePart, "vehicle_part");
        $t!(VehiclePartLocation, "vehicle_part_location");
        $t!(VehiclePartCategory, "vehicle_part_category");
        $t!(Mutation, "mutation");
        $t!(MutationCategory, "mutation_category");
        $t!(TraitGroup, "trait_group");
        $t!(Bionic, "bionic");
        $t!(Effect, "effect_type");
        $t!(Faction, "faction");
        $t!(Scenario, "scenario");
        $t!(Material, "material");
        $t!(Skill, "skill");
        $t!(Trap, "trap");
        $t!(StartLocation, "start_location");
    };
    (list $mapper:ident) => {
        vec![
            $mapper!(Item, "ITEM"),
            $mapper!(Monster, "MONSTER"),
            $mapper!(Terrain, "terrain"),
            $mapper!(Furniture, "furniture"),
            $mapper!(Recipe, "recipe"),
            $mapper!(ItemGroup, "item_group"),
            $mapper!(MapgenPalette, "palette"),
            $mapper!(OvermapTerrain, "overmap_terrain"),
            $mapper!(OvermapSpecial, "overmap_special"),
            $mapper!(OvermapConnection, "overmap_connection"),
            $mapper!(OvermapLocation, "overmap_location"),
            $mapper!(OvermapLandUseCode, "overmap_land_use_code"),
            $mapper!(Field, "field_type"),
            $mapper!(VehiclePart, "vehicle_part"),
            $mapper!(VehiclePartLocation, "vehicle_part_location"),
            $mapper!(VehiclePartCategory, "vehicle_part_category"),
            $mapper!(Mutation, "mutation"),
            $mapper!(MutationCategory, "mutation_category"),
            $mapper!(TraitGroup, "trait_group"),
            $mapper!(Bionic, "bionic"),
            $mapper!(Effect, "effect_type"),
            $mapper!(Faction, "faction"),
            $mapper!(Scenario, "scenario"),
            $mapper!(Material, "material"),
            $mapper!(Skill, "skill"),
            $mapper!(Trap, "trap"),
            $mapper!(StartLocation, "start_location"),
        ]
    };
    ($mapper:ident) => {
        $mapper!(
            Item,
            "ITEM",
            Monster,
            "MONSTER",
            Terrain,
            "terrain",
            Furniture,
            "furniture",
            Recipe,
            "recipe",
            ItemGroup,
            "item_group",
            MapgenPalette,
            "palette",
            OvermapTerrain,
            "overmap_terrain",
            OvermapSpecial,
            "overmap_special",
            OvermapConnection,
            "overmap_connection",
            OvermapLocation,
            "overmap_location",
            OvermapLandUseCode,
            "overmap_land_use_code",
            Field,
            "field_type",
            VehiclePart,
            "vehicle_part",
            VehiclePartLocation,
            "vehicle_part_location",
            VehiclePartCategory,
            "vehicle_part_category",
            Mutation,
            "mutation",
            MutationCategory,
            "mutation_category",
            TraitGroup,
            "trait_group",
            Bionic,
            "bionic",
            Effect,
            "effect_type",
            Faction,
            "faction",
            Scenario,
            "scenario",
            Material,
            "material",
            Skill,
            "skill",
            Trap,
            "trap",
            StartLocation,
            "start_location",
        )
    };
}
