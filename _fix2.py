#!/usr/bin/env python3
p = "crates/cdda_render/src/dev_worldgen.rs"
with open(p) as f:
    c = f.read()

# Fix imports
c = c.replace(
    "use bevy::prelude::*;\nuse bevy::sprite::Anchor;", "use bevy::prelude::*;"
)

# Fix tracing::info -> info! (bevy re-exports it)
c = c.replace("tracing::info!", "info!")

# Remove Anchor::TopLeft
c = c.replace(
    "        TextLayout::new_with_justify(Justify::Left),\n        Anchor::TopLeft,\n        Transform::from_xyz(-620.0, 350.0, 0.0),",
    "        TextLayout::new_with_justify(Justify::Left),\n        Transform::from_xyz(-620.0, 350.0, 0.0),",
)

# Remove Anchor::BottomLeft
c = c.replace(
    "        TextColor(Color::srgb(0.4, 0.8, 0.4)),\n        Anchor::BottomLeft,\n        Transform::from_xyz(-620.0, -350.0, 0.0),",
    "        TextColor(Color::srgb(0.4, 0.8, 0.4)),\n        Transform::from_xyz(-620.0, -350.0, 0.0),",
)

# Remove Anchor::Center
c = c.replace(
    "        TextColor(Color::srgb(1.0, 0.2, 0.2)),\n        Anchor::Center,\n        Transform::from_xyz(0.0, -300.0, 0.0),",
    "        TextColor(Color::srgb(1.0, 0.2, 0.2)),\n        Transform::from_xyz(0.0, -300.0, 0.0),",
)

with open(p, "w") as f:
    f.write(c)
print("Fixed anchor and tracing issues")
