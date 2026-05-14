//! Interned string tokens — typed integer wrappers for string IDs.

use bevy_reflect::Reflect;

macro_rules! intern_token {
    ($name:ident, $inner:ty) => {
        #[doc = concat!("Interned identifier for a ", stringify!($name), ".")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
        pub struct $name(pub $inner);

        impl $name {
            pub fn new(id: $inner) -> Self {
                Self(id)
            }
        }
    };
}

intern_token!(ItemTypeToken, u32);
intern_token!(SkillToken, u16);
intern_token!(AmmoTypeToken, u16);
intern_token!(BodyPartToken, u16);
intern_token!(ComestibleToken, u16);
