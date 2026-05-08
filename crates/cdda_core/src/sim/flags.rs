//! Per-category bitflag system — FixedBitSet backed, one registry per category.

use bevy_ecs::prelude::*;
use fixedbitset::FixedBitSet;

const MAX_CATEGORY_FLAGS: usize = 256;

#[derive(Debug, Clone)]
pub struct FlagMap {
    map: bidimap::BiMap<String, u16>,
    next_idx: u16,
}

impl Default for FlagMap {
    fn default() -> Self {
        Self { map: bidimap::BiMap::new(), next_idx: 0 }
    }
}

impl FlagMap {
    pub fn register(&mut self, flag: &str) -> u16 {
        if let Some(&idx) = self.map.get_by_left(flag) { return idx; }
        let idx = self.next_idx;
        assert!((idx as usize) < MAX_CATEGORY_FLAGS);
        self.map.insert(flag.to_string(), idx);
        self.next_idx += 1;
        idx
    }
    pub fn idx(&self, flag: &str) -> u16 { *self.map.get_by_left(flag).unwrap() }
    pub fn to_bitset(&mut self, flags: &[String]) -> FixedBitSet {
        let mut bs = FixedBitSet::with_capacity(MAX_CATEGORY_FLAGS);
        for flag in flags { bs.put(self.register(flag) as usize); }
        bs
    }
}

// Per-category registries
#[derive(Resource, Debug, Clone, Default)] pub struct ItemFlagRegistry(pub FlagMap);
#[derive(Resource, Debug, Clone, Default)] pub struct MonsterFlagRegistry(pub FlagMap);
#[derive(Resource, Debug, Clone, Default)] pub struct TerrainFlagRegistry(pub FlagMap);
#[derive(Resource, Debug, Clone, Default)] pub struct FurnitureFlagRegistry(pub FlagMap);
#[derive(Resource, Debug, Clone, Default)] pub struct MeleeFlagRegistry(pub FlagMap);
#[derive(Resource, Debug, Clone, Default)] pub struct ArmorFlagRegistry(pub FlagMap);
#[derive(Resource, Debug, Clone, Default)] pub struct GunFlagRegistry(pub FlagMap);

macro_rules! flag_comp {
    ($n:ident, $r:ty) => {
        #[derive(Component, Debug, Clone)]
        pub struct $n(pub FixedBitSet);
        impl $n {
            pub fn new() -> Self { Self(FixedBitSet::with_capacity(MAX_CATEGORY_FLAGS)) }
            pub fn has(&self, reg: &$r, flag: &str) -> bool {
                reg.0.map.get_by_left(flag).map_or(false, |&i| self.0.contains(i as usize))
            }
            pub fn len(&self) -> usize { self.0.count_ones(..) }
            pub fn is_empty(&self) -> bool { self.0.count_ones(..) == 0 }
            pub fn has_idx(&self, idx: u16) -> bool { self.0.contains(idx as usize) }
        }
        impl Default for $n { fn default() -> Self { Self::new() } }
    };
}
flag_comp!(ItemFlags, ItemFlagRegistry);
flag_comp!(MonsterFlags, MonsterFlagRegistry);
flag_comp!(TerrainFlags, TerrainFlagRegistry);
flag_comp!(FurnitureFlags, FurnitureFlagRegistry);
flag_comp!(MeleeFlags, MeleeFlagRegistry);
flag_comp!(ArmorFlags, ArmorFlagRegistry);
flag_comp!(GunFlags, GunFlagRegistry);
pub type ItemFlagList = ItemFlags;
