use bevy_app::{App, Plugin};

use crate::core::components::item::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        // Item state
        app.register_type::<DefOrigin>();
        app.register_type::<StackCount>();
        app.register_type::<CurrentCharges>();
        app.register_type::<LoadedAmmo>();
        app.register_type::<Spoilable>();
        app.register_type::<ItemDamage>();

        // Container tags
        app.register_type::<Sealed>();
        app.register_type::<Rigid>();
        app.register_type::<Watertight>();
        app.register_type::<PreservesTemp>();
        app.register_type::<Fireproof>();
        app.register_type::<GasTight>();

        // Relationships
        app.register_type::<InsideContainer>();
        app.register_type::<ContainerContents>();
        app.register_type::<WieldedBy>();
        app.register_type::<WieldedItems>();
        app.register_type::<WornOn>();
        app.register_type::<WornBy>();
        app.register_type::<MountedOn>();
        app.register_type::<MountedPockets>();

        // Pocket system
        app.register_type::<Pocket>();
        app.register_type::<PocketType>();
        app.register_type::<PocketRestriction>();
        app.register_type::<AttachmentSlot>();
        app.register_type::<AttachmentType>();
        app.register_type::<Container>();
        app.register_type::<IsPocket>();
        app.register_type::<PocketOf>();

        // Inventory
        app.register_type::<Invlet>();
        app.register_type::<InvletFavorites>();
        app.register_type::<Inventory>();
        app.register_type::<ItemQualities>();

        // In-progress crafting
        app.register_type::<InProgressCraft>();
    }
}
