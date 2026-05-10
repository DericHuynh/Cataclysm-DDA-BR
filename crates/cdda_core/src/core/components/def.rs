//! # Composable ECS components for definition entities
//!
//! Instead of one monolithic struct per definition type (ItemTemplate with
//! 12 `Option<SubBehavior>` fields), each definition is an **entity** that
//! gets exactly the components its subtypes require.
//!
//! A carrot: `DefStrId("carrot")` + `ItemName("carrot")` + `FoodData { ... }`
//! A glock:  `DefStrId("glock_17")` + `ItemName("Glock 17")` + `GunData { ... }` + `WeaponData { ... }`
//!
//! Queries become surgical:
//! ```ignore
//! // Only entities with GunData — carrots are invisible
//! fn gun_recoil_system(query: Query<&GunData, With<GunData>>) { }
//! ```
//!
//! These components live on entities in a **separate `World`** (the
//! `DefinitionWorld`), never in the gameplay world. Runtime entities
//! reference definition entities via a single component.

use bevy_ecs::prelude::*;

// ===========================================================================
// MARKER — on every definition entity
// ===========================================================================

/// Present on every definition entity. Use `Without<IsDef>` on gameplay queries
/// to keep worlds cleanly separated.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct IsDef;

// ===========================================================================
// UNIVERSAL — every item definition gets these
// ===========================================================================

/// The string identifier from JSON (e.g. "t_shirt", "glock_17", "carrot").
#[derive(Component, Debug, Clone)]
pub struct DefStrId(pub String);

/// Display name (localized).
#[derive(Component, Debug, Clone)]
pub struct ItemName(pub String);

/// Flavour text.
#[derive(Component, Debug, Clone)]
pub struct ItemDescription(pub String);

/// Weight in grams.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemWeight(pub u32);

/// Volume in millilitres.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemVolume(pub u32);

/// ASCII symbol for map display.
#[derive(Component, Debug, Clone)]
pub struct ItemSymbol(pub char);

/// Map colour.
#[derive(Component, Debug, Clone)]
pub struct ItemColor(pub String);

/// Materials the item is made from (e.g. ["steel", "plastic"]).
#[derive(Component, Debug, Clone)]
pub struct ItemMaterials(pub Vec<String>);

/// Bitflag component — see `crate::data::flags::ItemFlagList`.
pub type ItemFlagList = crate::data::flags::ItemFlagList;

/// Phase of matter.
#[derive(Component, Debug, Clone, Copy)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    Plasma,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemPhase(pub Phase);

/// Stack size / count mode.
#[derive(Component, Debug, Clone, Copy)]
pub enum CountMode {
    Single,
    ByCount { default: u32, max: u32 },
    Charges { default: u32, max: u32 },
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemCountMode(pub CountMode);

/// Price before and after the cataclysm (in cents).
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemPrice {
    pub price: u64,
    pub price_postapoc: u64,
}

/// Which category this item belongs to (e.g. "weapons", "food", "ammo").
#[derive(Component, Debug, Clone)]
pub struct ItemCategory(pub String);

// ===========================================================================
// SUBTYPE COMPONENTS — only present for items of that subtype
// ===========================================================================

// ── Weapon (melee) ─────────────────────────────────────────────────────────

/// Present on items that can be used as melee weapons.
#[derive(Component, Debug, Clone)]
pub struct WeaponData {
    pub damage_bash: i32,
    pub damage_cut: i32,
    pub damage_stab: i32,
    pub to_hit: i32,
    pub moves_per_attack: i32,
    pub reach: u8,
    /// Combat techniques (e.g. "SWEEP", "RAPID", "BLOCK").
    pub techniques: Vec<String>,
    /// Number of melee damage dice.
    pub dice: u32,
    /// Sides on each melee damage die.
    pub dice_sides: u32,
    /// Weapon skill used (e.g. "bashing", "cutting", "stabbing").
    pub skill: String,
}

// ── Ranged weapon (gun) ────────────────────────────────────────────────────

/// Present on guns and other ranged weapons.
#[derive(Component, Debug, Clone)]
pub struct GunData {
    pub skill: String,
    pub ammo_type: String,
    pub dispersion: i32,
    pub recoil: i32,
    pub reload_time: i32,
    pub clip_size: i32,
    /// Rounds fired per trigger pull.
    pub burst: u32,
    /// Ammo effects (e.g. "INCENDIARY", "EXPLOSIVE", "SHOT").
    pub ammo_effects: Vec<String>,
}

// ── Ammo ───────────────────────────────────────────────────────────────────

/// Present on ammunition items.
#[derive(Component, Debug, Clone)]
pub struct AmmoData {
    pub ammo_type: String,
    pub damage: i32,
    pub pierce: i32,
    pub range: i32,
    pub dispersion: i32,
    pub recoil: i32,
    pub count: i32,
    /// Casing item left after firing.
    pub casing: Option<String>,
    /// Ammo effects (e.g. "INCENDIARY", "FRAG", "TANGLE").
    pub effects: Vec<String>,
    /// How many rounds stack in one tile.
    pub stack_size: u32,
}

// ── Magazine ───────────────────────────────────────────────────────────────

/// Present on magazine items.
#[derive(Component, Debug, Clone)]
pub struct MagazineData {
    pub ammo_type: String,
    pub capacity: i32,
    pub reload_time: i32,
    /// Item ID of the belt linkage (for belt-fed weapons).
    pub linkage: Option<String>,
    /// Default ammo loaded when spawned.
    pub default_ammo: String,
}

// ── Armour ─────────────────────────────────────────────────────────────────

/// One body part covered by an armour item.
#[derive(Debug, Clone)]
pub struct ArmourPart {
    /// Body parts covered (joined from `covers` array, e.g. "head,torso").
    pub body_part: String,
    pub coverage: u8,
    pub encumbrance: i32,
    pub warmth: i32,
    /// Which clothing layers this piece occupies (e.g. "NORMAL", "OUTER").
    pub layers: Vec<String>,
    /// Sub-parts specifically covered (optional detail).
    pub specifically_covers: Vec<String>,
    /// Materials: (material_id, thickness_mm, covered_by_mat_pct).
    pub material: Vec<(String, f64, f64)>,
}

/// Present on armour items.
#[derive(Component, Debug, Clone)]
pub struct ArmourData {
    pub parts: Vec<ArmourPart>,
    pub material_thickness: f32,
    /// Environmental protection: [acid, fire, electrical, radiation, all].
    pub env_protection: [u32; 5],
}

// ── Food / drink (comestible) ──────────────────────────────────────────────

/// Present on comestible items (food, drink, meds).
#[derive(Component, Debug, Clone)]
pub struct FoodData {
    pub calories: i32,
    pub quench: i32,
    pub fun: i32,
    pub healthy: i32,
    pub stim: i32,
    pub spoils_in: u32,          // turns until rotten
    pub comestible_type: String, // "FOOD", "DRINK", "MED"
}

// ── Tool ───────────────────────────────────────────────────────────────────

/// Present on tool items.
#[derive(Component, Debug, Clone)]
pub struct ToolData {
    pub max_charges: i32,
    pub charges_per_use: i32,
    pub turns_per_charge: i32,
    pub ammo_type: Option<String>,
    /// Item ID this tool reverts to when depleted (e.g. lighter → empty lighter).
    pub revert_to: Option<String>,
    /// Power draw description (e.g. "2000 W" for a welding tool).
    pub power_draw: Option<String>,
}

// ── Book ───────────────────────────────────────────────────────────────────

/// Present on books and learnable items.
#[derive(Component, Debug, Clone)]
pub struct BookData {
    pub skill: String,
    pub required_level: u8,
    pub max_level: u8,
    pub fun: i32,
    pub intelligence: u8,
    pub time: u32,
    /// Number of chapters (0 = infinite / no chapter system).
    pub chapters: u32,
    /// Martial art style this book teaches (empty if not a martial art manual).
    pub martial_art: String,
}

// ── Gun mod ────────────────────────────────────────────────────────────────

/// Present on gun modification items.
#[derive(Component, Debug, Clone)]
pub struct GunModData {
    pub install_time: u32,
}

// ── Container / pocket ─────────────────────────────────────────────────────

/// A single pocket inside a container.
#[derive(Debug, Clone)]
pub struct PocketTemplate {
    pub pocket_type: String,
    pub max_volume: u32,
    pub max_weight: u32,
    pub max_item_length: u32,
    pub sealed: bool,
    pub rigid: bool,
    pub holster: bool,
    pub ablative: bool,
    pub description: String,
    pub flag_restriction: Vec<String>,
}

/// Present on container items.
#[derive(Component, Debug, Clone)]
pub struct ContainerData {
    pub pockets: Vec<PocketTemplate>,
    /// Total max volume of the container in ml.
    pub max_volume: u32,
    /// Total max weight the container can hold in grams.
    pub max_weight: u32,
}

// ── Drug / medicine ────────────────────────────────────────────────────────

/// Present on drug/medicine items.
#[derive(Component, Debug, Clone)]
pub struct DrugData {
    pub effects: Vec<String>,
    pub duration: u32,
    pub addiction_potential: i32,
}

// ── Miscellaneous item components ──────────────────────────────────────────

/// Stack size — how many of this item fit in one tile / inventory slot.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemStackSize(pub u32);

/// Longest side of the item in millimetres (for sorting / pocket fitting).
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemLongestSide(pub u32);

/// Insulation / warmth value (positive = keeps warm, negative = cools).
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemInsulation(pub i32);

/// Whether the item covers the head (affects sight-related penalties).
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemCoversHead(pub bool);

// ===========================================================================
// MONSTER DEFINITION COMPONENTS
// ===========================================================================

/// Present on monster definition entities.
#[derive(Component, Debug, Clone)]
pub struct MonsterName(pub String);

#[derive(Component, Debug, Clone)]
pub struct MonsterDescription(pub String);

#[derive(Component, Debug, Clone)]
pub struct MonsterStats {
    pub hp: i32,
    pub speed: i32,
    pub attack_cost: i32,
    pub dodge: i32,
    pub morale: i32,
    pub aggression: i32,
    pub melee_skill: i32,
    pub melee_dice: i32,
    pub melee_dice_sides: i32,
    pub grab_strength: i32,
    pub bleed_rate: i32,
    pub diff: i32,
}

#[derive(Component, Debug, Clone)]
pub struct MonsterMelee {
    pub dice: u32,
    pub dice_sides: u32,
    pub damage_bash: i32,
    pub damage_cut: i32,
    pub damage_stab: i32,
    pub to_hit: i32,
}

#[derive(Component, Debug, Clone)]
pub struct MonsterVision {
    pub day: u32,
    pub night: u32,
}

#[derive(Component, Debug, Clone)]
pub struct MonsterArmour {
    pub bash: i32,
    pub cut: i32,
    pub bullet: i32,
    pub fire: i32,
    pub acid: i32,
    pub electric: i32,
    pub cold: i32,
    pub stab: i32,
}

/// Monster upgrade / evolution path.
#[derive(Component, Debug, Clone)]
pub struct MonsterUpgrade {
    /// Monster ID it upgrades into.
    pub into: Option<String>,
    /// Monster group ID to pull the upgrade from.
    pub into_group: Option<String>,
    /// Time in days before upgrade triggers.
    pub into_time: u32,
    /// Percentage chance of upgrading.
    pub into_pct: f32,
}

/// Harvest list ID for butchery results.
#[derive(Component, Debug, Clone)]
pub struct MonsterHarvest(pub String);

/// Death function name (e.g. "DISAPPEAR", "TRIFFID_HEART", "FUNGUS", "KEEP_MAJOR").
#[derive(Component, Debug, Clone)]
pub struct MonsterDeathFunction(pub String);

/// Item group ID for death drops (loot that spawns on death).
#[derive(Component, Debug, Clone)]
pub struct MonsterDeathDrops(pub String);

/// List of special attack IDs this monster can use.
#[derive(Component, Debug, Clone)]
pub struct MonsterSpecialAttacks(pub Vec<String>);

/// Species this monster belongs to (e.g. "HUMAN", "ZOMBIE", "MAMMAL", "INSECT").
#[derive(Component, Debug, Clone)]
pub struct MonsterSpecies(pub Vec<String>);

/// Default faction the monster belongs to (e.g. "zombie", "ant", "bee").
#[derive(Component, Debug, Clone)]
pub struct MonsterDefaultFaction(pub String);

/// Body type string (e.g. "human", "bird", "snake", "insect").
#[derive(Component, Debug, Clone)]
pub struct MonsterBodyType(pub String);

// ===========================================================================
// TERRAIN DEFINITION COMPONENTS
// ===========================================================================

#[derive(Component, Debug, Clone)]
pub struct TerrainName(pub String);
#[derive(Component, Debug, Clone)]
pub struct TerrainSymbol(pub char);
#[derive(Component, Debug, Clone)]
pub struct TerrainColor(pub String);
#[derive(Component, Debug, Clone)]
pub struct TerrainMoveCost(pub i32); // 100 = normal, 0 = impassable
#[derive(Component, Debug, Clone)]
pub struct TerrainOpacity(pub i32); // 0 = transparent

/// Light emitted by this terrain tile per turn (for glow-in-the-dark tiles).
#[derive(Component, Debug, Clone, Copy)]
pub struct TerrainLightEmitted(pub u32);

/// Roof terrain above this tile (e.g. "t_floor" for a building interior).
#[derive(Component, Debug, Clone)]
pub struct TerrainRoof(pub Option<String>);

/// Whether this tile has a ceiling (affects ranged attacks and weather).
#[derive(Component, Debug, Clone, Copy)]
pub struct TerrainHasCeiling(pub bool);

/// Terrain type IDs this terrain visually connects to (e.g. "WALL", "RAILING").
#[derive(Component, Debug, Clone)]
pub struct TerrainConnectsTo(pub Vec<String>);

/// Examine action override for this terrain (e.g. "pedestal_temperature").
#[derive(Component, Debug, Clone)]
pub struct TerrainExamineAction(pub String);

/// Trap ID placed on this terrain (e.g. "tr_beartrap", "tr_pit").
#[derive(Component, Debug, Clone)]
pub struct TerrainTrap(pub String);

// ===========================================================================
// FURNITURE DEFINITION COMPONENTS
// ===========================================================================

#[derive(Component, Debug, Clone)]
pub struct FurnitureName(pub String);
#[derive(Component, Debug, Clone)]
pub struct FurnitureSymbol(pub char);
#[derive(Component, Debug, Clone)]
pub struct FurnitureColor(pub String);
#[derive(Component, Debug, Clone)]
pub struct FurnitureMoveCostMod(pub i32); // added to terrain move cost (negative = faster)

/// Coverage percentage this furniture provides (e.g. 75 for a counter).
#[derive(Component, Debug, Clone, Copy)]
pub struct FurnitureCoverage(pub u32);

/// Strength required to move / bash this furniture.
#[derive(Component, Debug, Clone, Copy)]
pub struct FurnitureRequiredStr(pub i32);

/// Max volume this furniture can hold (for storage furniture) in ml.
#[derive(Component, Debug, Clone, Copy)]
pub struct FurnitureMaxVolume(pub u32);

/// Comfort rating for sleeping / sitting.
#[derive(Component, Debug, Clone, Copy)]
pub struct FurnitureComfort(pub i32);

/// Light emitted by this furniture (e.g. a lamp or fire).
#[derive(Component, Debug, Clone, Copy)]
pub struct FurnitureLightEmitted(pub u32);

/// Examine action override for this furniture.
#[derive(Component, Debug, Clone)]
pub struct FurnitureExamineAction(pub String);

/// Mass of this furniture in grams.
#[derive(Component, Debug, Clone, Copy)]
pub struct FurnitureMass(pub u32);

// ===========================================================================
// BODY PART DEFINITION COMPONENTS
// ===========================================================================

/// String ID of a body part type (e.g. "head", "arm_l", "torso").
/// Present on body part DEF entities.
#[derive(Component, Debug, Clone)]
pub struct BodyPartDefId(pub String);

/// Display name (e.g. "head", "left arm").
#[derive(Component, Debug, Clone)]
pub struct BodyPartName(pub String);

/// Hit size modifier — larger = easier to hit.
#[derive(Component, Debug, Clone, Copy)]
pub struct BodyPartHitSize(pub f32);

/// Hit difficulty modifier — higher = harder to hit.
#[derive(Component, Debug, Clone, Copy)]
pub struct BodyPartHitDifficulty(pub f32);

/// Base HP for this body part.
#[derive(Component, Debug, Clone, Copy)]
pub struct BodyPartBaseHp(pub f32);

/// Drench capacity in ml.
#[derive(Component, Debug, Clone, Copy)]
pub struct BodyPartDrenchCapacity(pub u32);

/// Side: "left", "right", or "both".
#[derive(Component, Debug, Clone)]
pub struct BodyPartSide(pub String);

/// Legacy ID for save compatibility (e.g. "HEAD", "TORSO").
#[derive(Component, Debug, Clone)]
pub struct BodyPartLegacyId(pub String);

/// Capability markers — zero-sized, composable, Clone.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct IsVital;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CanGrasp;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CanWalk;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CanSee;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CanBite;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CanFly;

/// Sub-parts of this body part def.
/// Relationship: body part def -> child body part defs.
#[derive(Component, Debug, Clone)]
#[relationship_target(relationship = ParentPart)]
pub struct SubParts(Vec<Entity>);

impl SubParts {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// Points to the parent body part def.
#[derive(Component, Debug, Clone)]
#[relationship(relationship_target = SubParts)]
pub struct ParentPart(pub Entity);

/// Marker present on every recipe definition entity.
/// Used to distinguish recipe defs from item/monster defs in queries.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct IsRecipeDef;

// ===========================================================================
// RECIPE DEFINITION COMPONENTS
// ===========================================================================

/// Primary skill used for this recipe (e.g. "fabrication", "cooking").
#[derive(Component, Debug, Clone)]
pub struct RecipeSkillUsed(pub String);

/// Skill difficulty rating (1-10).
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeDifficulty(pub u32);

/// Required skill level (possibly optional, for recipes with requirements).
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeRequiredLevel(pub u32);

/// Base crafting time in turns.
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeTime(pub u32);

/// Whether this recipe is automatically learned when skill is high enough.
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeAutolearn(pub bool);

/// Whether this recipe can be reversed (uncrafted).
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeReversible(pub bool);

/// Result item ID.
#[derive(Component, Debug, Clone)]
pub struct RecipeResult(pub String);

/// Number of result items produced per craft.
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeResultCount(pub u32);

/// Charges on the result (for items that use charges).
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeResultCharges(pub u32);

/// One component requirement — a specific item and quantity.
#[derive(Debug, Clone)]
pub struct RecipeComponentEntry {
    pub item_id: String,
    pub count: u32,
    pub recovered: bool,
}

/// Component requirements: outer Vec is alternatives, inner Vec is all required.
#[derive(Component, Debug, Clone)]
pub struct RecipeComponents(pub Vec<Vec<RecipeComponentEntry>>);

/// A tool requirement.
#[derive(Debug, Clone)]
pub struct RecipeToolEntry {
    pub item_id: String,
    pub amount: u32,
}

/// Tool requirements: outer Vec is alternatives, inner Vec is required tools.
#[derive(Component, Debug, Clone)]
pub struct RecipeTools(pub Vec<Vec<RecipeToolEntry>>);

/// A required tool quality.
#[derive(Component, Debug, Clone)]
pub struct RecipeQuality(pub String, pub u32);

/// Required qualities for the recipe.
#[derive(Component, Debug, Clone)]
pub struct RecipeQualities(pub Vec<(String, u32)>);

/// Recipe category (e.g. "CC_WEAPON", "CC_FOOD").
#[derive(Component, Debug, Clone)]
pub struct RecipeCategory(pub String);

/// Recipe subcategory (e.g. "CSC_FOOD_BREAD", parsed from the raw `subcategory` field).
#[derive(Component, Debug, Clone)]
pub struct RecipeSubcategory(pub String);

/// Recipe flags (e.g. "BLIND_EASY", "SECRET").
#[derive(Component, Debug, Clone)]
pub struct RecipeFlags(pub Vec<String>);

/// Byproducts produced alongside the result.
#[derive(Debug, Clone)]
pub struct RecipeByproduct {
    pub item_id: String,
    pub count: u32,
}

/// Byproducts produced by this recipe.
#[derive(Component, Debug, Clone)]
pub struct RecipeByproducts(pub Vec<RecipeByproduct>);

/// Container item for the result (e.g. a jar for jam).
#[derive(Component, Debug, Clone)]
pub struct RecipeContainer(pub String);

/// Batch time factors: time reduction per additional unit.
#[derive(Component, Debug, Clone, Copy)]
pub struct RecipeBatchTime(pub f32);
