//! Native text measurement, layout and glyph rasterization without a renderer/window.
use bevy::{
    app::{HierarchyPropagatePlugin, PropagateSet},
    camera::{ComputedCameraValues, RenderTargetInfo},
    prelude::*,
    text::{detect_text_needs_rerender, TextLayoutInfo, TextPlugin},
    ui::{
        ui_layout_system,
        ui_surface::UiSurface,
        update::propagate_ui_target_cameras,
        widget::{measure_text_system, text_system},
        ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiSystems,
    },
};
use cdda_render::render::{UiFontHandle, UiPresentationPlugin};

pub fn add_text_pipeline(app: &mut App) {
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TextPlugin,
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
        UiPresentationPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<TextureAtlasLayout>()
    .init_resource::<UiSurface>()
    .init_resource::<UiScale>()
    .configure_sets(
        PostUpdate,
        (
            UiSystems::Prepare,
            UiSystems::Propagate,
            UiSystems::Content,
            UiSystems::Layout,
            UiSystems::PostLayout,
        )
            .chain(),
    )
    .configure_sets(
        PostUpdate,
        (
            PropagateSet::<ComputedUiTargetCamera>::default().in_set(UiSystems::Propagate),
            PropagateSet::<ComputedUiRenderTargetInfo>::default().in_set(UiSystems::Propagate),
        ),
    )
    .add_systems(
        PostUpdate,
        (
            propagate_ui_target_cameras.in_set(UiSystems::Prepare),
            (detect_text_needs_rerender::<Text>, measure_text_system)
                .chain()
                .in_set(UiSystems::Content),
            ui_layout_system.in_set(UiSystems::Layout),
            text_system
                .in_set(UiSystems::PostLayout)
                .before(bevy::asset::AssetEventSystems),
        ),
    );
    let font = Font::try_from_bytes(
        include_bytes!("../../../cdda_app/assets/fonts/ShareTechMono-Regular.ttf").to_vec(),
    )
    .unwrap();
    let handle = app.world_mut().resource_mut::<Assets<Font>>().add(font);
    app.insert_resource(UiFontHandle(Some(handle)));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1280, 720),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
    ));
}

/// Check the very first presented frame, not just an eventual settled screenshot.
pub fn assert_shaped(world: &mut World) {
    let font = world.resource::<UiFontHandle>().0.clone().unwrap();
    let mut count = 0;
    for (text, actual_font, layout, node) in world
        .query::<(&Text, &TextFont, &TextLayoutInfo, &ComputedNode)>()
        .iter(world)
    {
        assert_eq!(actual_font.font, font, "fallback font on {}", text.0);
        if !text.trim().is_empty() && node.size().min_element() > 0.0 {
            assert!(!layout.glyphs.is_empty(), "blank text on {}", text.0);
            for glyph in &layout.glyphs {
                assert!(world
                    .resource::<Assets<Image>>()
                    .contains(glyph.atlas_info.texture));
            }
            count += 1;
        }
    }
    assert!(count > 0, "fixture must lay out visible text");
}
