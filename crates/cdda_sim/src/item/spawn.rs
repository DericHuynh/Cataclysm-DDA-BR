//! Native item creation. Prepare immutable data before committing a transaction.
use bevy_ecs::prelude::*;
use cdda_catalog::{interner::ItemTypeRegistry, inventory::ItemDefinitionRef};
use cdda_components::{def::*, item::*};
use cdda_core_types::core::units::{Length, Volume, Weight};

/// Snapshot a craft output before consuming inputs. No live definition Entity is
/// needed to finish it after a reload. Only explicit item capabilities are copied.
#[derive(Component, Clone)]
pub struct PreparedItem {
    pub key: String,
    item_name: Option<ItemName>,
    item_description: Option<ItemDescription>,
    item_weight: Option<ItemWeight>,
    item_volume: Option<ItemVolume>,
    item_symbol: Option<ItemSymbol>,
    item_color: Option<ItemColor>,
    item_materials: Option<ItemMaterials>,
    item_phase: Option<ItemPhase>,
    item_count_mode: Option<ItemCountMode>,
    item_price: Option<ItemPrice>,
    item_category: Option<ItemCategory>,
    weapon_data: Option<WeaponData>,
    gun_data: Option<GunData>,
    ammo_data: Option<AmmoData>,
    magazine_data: Option<MagazineData>,
    armour_data: Option<ArmourData>,
    food_data: Option<FoodData>,
    tool_data: Option<ToolData>,
    book_data: Option<BookData>,
    gun_mod_data: Option<GunModData>,
    container_data: Option<ContainerData>,
    drug_data: Option<DrugData>,
    item_stack_size: Option<ItemStackSize>,
    item_longest_side: Option<ItemLongestSide>,
    item_insulation: Option<ItemInsulation>,
    item_covers_head: Option<ItemCoversHead>,
    item_qualities: Option<ItemQualities>,
    item_definition_ref: Option<ItemDefinitionRef>,
}
impl PreparedItem {
    pub fn from_definition(world: &World, entity: Entity) -> Result<Self, String> {
        if world.get::<IsDef>(entity).is_none() {
            return Err("Expected an item definition entity".into());
        }
        let key = world
            .get::<DefStrId>(entity)
            .ok_or("Item definition has no stable key")?
            .0
            .clone();
        if world.get::<ItemName>(entity).is_none() {
            return Err("Item definition has no name".into());
        }
        Ok(Self {
            key,
            item_name: world.get::<ItemName>(entity).cloned(),
            item_description: world.get::<ItemDescription>(entity).cloned(),
            item_weight: world.get::<ItemWeight>(entity).cloned(),
            item_volume: world.get::<ItemVolume>(entity).cloned(),
            item_symbol: world.get::<ItemSymbol>(entity).cloned(),
            item_color: world.get::<ItemColor>(entity).cloned(),
            item_materials: world.get::<ItemMaterials>(entity).cloned(),
            item_phase: world.get::<ItemPhase>(entity).cloned(),
            item_count_mode: world.get::<ItemCountMode>(entity).cloned(),
            item_price: world.get::<ItemPrice>(entity).cloned(),
            item_category: world.get::<ItemCategory>(entity).cloned(),
            weapon_data: world.get::<WeaponData>(entity).cloned(),
            gun_data: world.get::<GunData>(entity).cloned(),
            ammo_data: world.get::<AmmoData>(entity).cloned(),
            magazine_data: world.get::<MagazineData>(entity).cloned(),
            armour_data: world.get::<ArmourData>(entity).cloned(),
            food_data: world.get::<FoodData>(entity).cloned(),
            tool_data: world.get::<ToolData>(entity).cloned(),
            book_data: world.get::<BookData>(entity).cloned(),
            gun_mod_data: world.get::<GunModData>(entity).cloned(),
            container_data: world.get::<ContainerData>(entity).cloned(),
            drug_data: world.get::<DrugData>(entity).cloned(),
            item_stack_size: world.get::<ItemStackSize>(entity).cloned(),
            item_longest_side: world.get::<ItemLongestSide>(entity).cloned(),
            item_insulation: world.get::<ItemInsulation>(entity).cloned(),
            item_covers_head: world.get::<ItemCoversHead>(entity).cloned(),
            item_qualities: world.get::<ItemQualities>(entity).cloned(),
            item_definition_ref: world.get::<ItemDefinitionRef>(entity).cloned(),
        })
    }
    pub fn validate_spawn(&self, world: &World, owner: Entity, count: u32) -> Result<(), String> {
        if count > 1
            && self
                .item_definition_ref
                .as_ref()
                .is_some_and(|d| !d.0.pockets.is_empty())
        {
            return Err("Container instances cannot share a stack of pockets".into());
        }
        StackCount::new(count).map_err(str::to_string)?;
        if world.get_entity(owner).is_err() {
            return Err("Item owner no longer exists".into());
        }
        Ok(())
    }
    /// Spawn into an existing owner after all fallible checks have succeeded.
    pub fn spawn(&self, world: &mut World, owner: Entity, count: u32) -> Result<Entity, String> {
        self.validate_spawn(world, owner, count)?;
        let count = StackCount::new(count).expect("validated positive count");
        world.init_resource::<ItemTypeRegistry>();
        let token = world.resource_mut::<ItemTypeRegistry>().intern(&self.key);
        let entity = world
            .spawn((
                DefStrId(self.key.clone()),
                ItemType(token),
                DefOrigin(token.0),
                count,
                InsideContainer(owner),
            ))
            .id();
        if let Some(value) = &self.item_name {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_description {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_weight {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_volume {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_symbol {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_color {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_materials {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_phase {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_count_mode {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_price {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_category {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.weapon_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.gun_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.ammo_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.magazine_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.armour_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.food_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.tool_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.book_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.gun_mod_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.container_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.drug_data {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_stack_size {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_longest_side {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_insulation {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_covers_head {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_qualities {
            world.entity_mut(entity).insert(value.clone());
        }
        if let Some(value) = &self.item_definition_ref {
            world.entity_mut(entity).insert(value.clone());
        }
        // Native pocket capabilities have independent ownership and lifetime.
        if let Some(definition) = &self.item_definition_ref {
            for pocket in &definition.0.pockets {
                world.spawn((
                    IsPocket,
                    MountedOn(entity),
                    Pocket {
                        max_volume: Volume(pocket.volume_ml as u64),
                        max_weight: Weight(pocket.weight_g as u64),
                        max_item_length: Length(u32::MAX),
                        min_item_volume: Volume(0),
                        pocket_type: PocketType::Container,
                    },
                ));
            }
        }
        Ok(entity)
    }
}
