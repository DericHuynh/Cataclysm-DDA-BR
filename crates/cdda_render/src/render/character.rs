//! Character-sheet screen.
//!
//! Two-column layout:
//!   LEFT  — static overview: name/bio, attributes, vitals, combat, status
//!   RIGHT — tabbed content: Skills | Traits | Effects | Bionics | Proficiencies
//!
//! Tab is switched with Tab / Shift-Tab; j/k scroll within a tab.

use bevy::prelude::*;
use bevy_ecs::message::MessageReader;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_ui::{RetainedRows, RowCell, TextRow};

use super::scroll::{sync_virtual_pane, FocusedRow, VirtualList};
use super::FooterHint;
use crate::render::theme::{self, UiTheme};
use bevy::ecs::system::SystemParam;
use cdda_components::actor::Stats;
use cdda_components::actor::{
    ActionPoints, Active, ActiveEffects, Bionic, Bleeding, BodyTemperature, CreatureMutations,
    CreatureProficiencies, CreatureSkills, DodgeDefense, Health, InstalledBionics, IntrinsicArmor,
    MeleeCapability, Morale, MutationEntry, OnFire, PlayerData, ProficiencyEntry, SkillEntry,
    StatusEffect, Stunned, Visible, Vision, Wetness,
};
use cdda_components::dev::DevPlayer;
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
use cdda_context::state::ContextActions;
use cdda_input::ActiveKeybindings;
use cdda_input::{BindableAction, GameAction, InputAction};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Resource, Default, Clone)]
pub struct CharacterSheetState {
    pub tab: CharacterTab,
    pub scroll: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterTab {
    #[default]
    Skills,
    Traits,
    Effects,
    Bionics,
    Proficiencies,
}

impl CharacterTab {
    fn label(self) -> &'static str {
        match self {
            Self::Skills => "SKILLS",
            Self::Traits => "TRAITS",
            Self::Effects => "EFFECTS",
            Self::Bionics => "BIONICS",
            Self::Proficiencies => "PROFS",
        }
    }

    const ALL: [CharacterTab; 5] = [
        CharacterTab::Skills,
        CharacterTab::Traits,
        CharacterTab::Effects,
        CharacterTab::Bionics,
        CharacterTab::Proficiencies,
    ];

    fn next(self) -> Self {
        let i = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        let i = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct CharSheetLeftContainer;

#[derive(Component)]
pub struct CharSheetContentContainer;

#[derive(Component)]
pub struct CharSheetTabsContainer;

// ---------------------------------------------------------------------------
// CddaScreen trait impl
// ---------------------------------------------------------------------------

pub struct CharacterScreen;

impl CddaScreen for CharacterScreen {
    const CTX: Ctx = Ctx::CharacterSheet;
    const ACTIONS: &'static [(&'static str, BindableAction)] = &[
        ("switch tab", BindableAction::NavigateNextTab),
        ("scroll", BindableAction::NavigateUp),
    ];

    fn spawn(world: &mut World) {
        spawn_character_sheet_screen(world);
    }

    fn update(_world: &mut World) {
        // Handled by update_character_sheet_screen + character_sheet_input in mod.rs.
        // TODO: migrate those systems into this method.
    }
}

pub fn spawn_character_sheet_screen(world: &mut World) {
    // Reset state on every open
    *world.resource_mut::<CharacterSheetState>() = CharacterSheetState::default();

    let ctx_actions = world.resource::<ContextActions>().clone();
    let active_keys = world.resource::<ActiveKeybindings>().clone();
    let font_handle = world.resource::<super::UiFontHandle>().0.clone();

    let cancel_key = active_keys.key_for(BindableAction::Cancel);
    let mut hints = format!("[{}] close", cancel_key);
    for entry in &ctx_actions.actions {
        let key = active_keys.key_for(entry.action);
        hints.push_str(&format!("  [{}] {}", key, entry.label));
    }

    let mut cmds = world.commands();
    cmds.spawn((
        DespawnOnExit(Ctx::CharacterSheet),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        theme::SurfacePaint(theme::Role::Canvas),
    ))
    .with_children(|root| {
        // ── Title bar ─────────────────────────────────────────────────
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                ..default()
            },
            theme::SurfacePaint(theme::Role::Raised),
        ))
        .with_child((
            Text::new("CHARACTER SHEET"),
            TextFont {
                font_size: 26.0,
                ..default()
            },
            theme::TextPaint(theme::Role::Accent),
        ));

        // ── Main body ─────────────────────────────────────────────────
        root.spawn((Node {
            flex_direction: FlexDirection::Row,
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        },))
            .with_children(|main| {
                // ── LEFT PANEL (overview) ─────────────────────────────────
                main.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(360.0),
                        flex_shrink: 0.0,
                        border: UiRect::right(Val::Px(1.0)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    theme::SurfacePaint(theme::Role::Surface),
                    theme::BorderPaint(theme::Role::Border),
                ))
                .with_children(|left| {
                    left.spawn((
                        CharSheetLeftContainer,
                        Node {
                            flex_direction: FlexDirection::Column,
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            ..default()
                        },
                    ));
                });

                // ── RIGHT PANEL (tabs) ────────────────────────────────────
                main.spawn((Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    ..default()
                },))
                    .with_children(|right| {
                        right.spawn((
                            CharSheetTabsContainer,
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_shrink: 0.0,
                                ..default()
                            },
                        ));
                        right.spawn((
                            CharSheetContentContainer,
                            RetainedRows::<usize>::default(),
                            crate::render::scroll::KeyboardScroll,
                            crate::render::scroll::FocusedRow::default(),
                            VirtualList {
                                row_height: 36.0,
                                ..default()
                            },
                            bevy::ui::ScrollPosition::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ));
                    });
            });

        // ── Footer ────────────────────────────────────────────────────
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(8.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            theme::SurfacePaint(theme::Role::Raised),
            theme::BorderPaint(theme::Role::Border),
        ))
        .with_child((
            Text::new(hints),
            super::ui_font(&font_handle, 13.0),
            theme::TextPaint(theme::Role::Muted),
            FooterHint,
        ));
    });
}

// ---------------------------------------------------------------------------
// Update — retain panels and virtualize tab rows
// ---------------------------------------------------------------------------

pub fn update_character_sheet_screen(
    mut commands: Commands,
    state: Res<CharacterSheetState>,
    theme: Res<UiTheme>,
    player_vitals: Query<
        (
            Option<&PlayerData>,
            Option<&Stats>,
            Option<&Health>,
            Option<&ActionPoints>,
            Option<&MeleeCapability>,
            Option<&DodgeDefense>,
            Option<&IntrinsicArmor>,
            Option<&Vision>,
            Option<&BodyTemperature>,
            Option<&Wetness>,
            Option<&Morale>,
        ),
        With<DevPlayer>,
    >,
    player_rels: Query<
        (
            Option<&CreatureSkills>,
            Option<&CreatureMutations>,
            Option<&ActiveEffects>,
            Option<&InstalledBionics>,
            Option<&CreatureProficiencies>,
            Option<&Stunned>,
            Option<&Bleeding>,
            Option<&OnFire>,
        ),
        With<DevPlayer>,
    >,
    skill_entries: Query<&SkillEntry>,
    mutation_entries: Query<&MutationEntry>,
    visible_tags: Query<(), With<Visible>>,
    effect_entries: Query<&StatusEffect>,
    bionic_entries: Query<&Bionic>,
    active_tags: Query<(), With<Active>>,
    proficiency_entries: Query<&ProficiencyEntry>,
    containers: CharacterContainers,
    mut changes: CharacterChanges,
    mut cache: Local<CharacterPresentation>,
) {
    let CharacterContainers {
        left_container,
        mut content_container,
        tabs_container,
    } = containers;
    let Ok(left_entity) = left_container.single() else {
        return;
    };
    let Ok((content_entity, mut list, mut focus, mut position, computed, mut retained)) =
        content_container.single_mut()
    else {
        return;
    };

    let Ok(tabs_entity) = tabs_container.single() else {
        return;
    };
    let data_changed = changes.changed() || cache.root != Some(content_entity);
    let tab_changed = cache.root != Some(content_entity) || cache.tab != state.tab;
    let chrome_changed = data_changed || tab_changed || theme.is_changed();
    if !chrome_changed && !state.is_changed() && !list.is_changed() {
        return;
    }
    cache.root = Some(content_entity);
    cache.tab = state.tab;

    // Extract player data (fallback to defaults when component absent).
    let (pdata, stats, health, ap, combat, dodge, armor, vision, temp, wet, morale) =
        player_vitals.single().unwrap_or((
            None, None, None, None, None, None, None, None, None, None, None,
        ));

    let (
        creature_skills,
        creature_mutations,
        active_effects,
        installed_bionics,
        creature_profs,
        stunned,
        bleeding,
        on_fire,
    ) = player_rels
        .single()
        .unwrap_or((None, None, None, None, None, None, None, None));

    let stats = stats.copied().unwrap_or_default();
    let hp_cur = health.map(|h| h.current).unwrap_or(100);
    let hp_max = health.map(|h| h.max).unwrap_or(100);
    let speed = ap.map(|a| a.speed).unwrap_or(100);

    if data_changed || theme.is_changed() {
        commands.entity(left_entity).despawn_children();
        // ── LEFT PANEL ─────────────────────────────────────────────────────────
        commands.entity(left_entity).with_children(|left| {
            // ── Identity section ───────────────────────────────────────────────
            spawn_section_header(left, "IDENTITY");
            if let Some(pd) = pdata {
                spawn_info_row(left, "Name", &pd.name, theme.color(theme::Role::Text), 0);
                let gender_str = match &pd.gender {
                    cdda_components::actor::Gender::Male => "male",
                    cdda_components::actor::Gender::Female => "female",
                    cdda_components::actor::Gender::NonBinary => "non-binary",
                    cdda_components::actor::Gender::Custom(s) => s.as_str(),
                };
                spawn_info_row(
                    left,
                    "Gender",
                    gender_str,
                    theme.color(theme::Role::Text),
                    1,
                );
                spawn_info_row(
                    left,
                    "Age",
                    &format!("{}", pd.age),
                    theme.color(theme::Role::Text),
                    0,
                );
                spawn_info_row(
                    left,
                    "Height",
                    &format!("{} cm", pd.height),
                    theme.color(theme::Role::Text),
                    1,
                );
                spawn_info_row(
                    left,
                    "Blood",
                    &pd.blood_type,
                    theme.color(theme::Role::Text),
                    0,
                );
            } else {
                spawn_info_row(
                    left,
                    "Name",
                    "Dev Player",
                    theme.color(theme::Role::Text),
                    0,
                );
            }

            // ── Attributes section ─────────────────────────────────────────────
            spawn_section_header(left, "ATTRIBUTES");
            spawn_stat_row(&theme, left, "STR", stats.strength, 0);
            spawn_stat_row(&theme, left, "DEX", stats.dexterity, 1);
            spawn_stat_row(&theme, left, "INT", stats.intelligence, 0);
            spawn_stat_row(&theme, left, "PER", stats.perception, 1);

            // ── Vitals section ─────────────────────────────────────────────────
            spawn_section_header(left, "VITALS");
            let hp_color = if hp_cur <= hp_max / 4 {
                theme::TEXT_RED
            } else if hp_cur <= hp_max / 2 {
                theme::TEXT_ORANGE
            } else if hp_cur < hp_max {
                theme::TEXT_YELLOW
            } else {
                theme::TEXT_GREEN
            };
            spawn_info_row(left, "HP", &format!("{} / {}", hp_cur, hp_max), hp_color, 0);
            let speed_color = if speed < 80 {
                theme::TEXT_RED
            } else if speed < 100 {
                theme::TEXT_YELLOW
            } else if speed > 100 {
                theme::TEXT_GREEN
            } else {
                theme.color(theme::Role::Text)
            };
            spawn_info_row(left, "Speed", &format!("{}", speed), speed_color, 1);

            // ── Combat section ─────────────────────────────────────────────────
            spawn_section_header(left, "COMBAT");
            if let Some(cs) = combat {
                let melee_str = if cs.melee_dice > 0 {
                    format!(
                        "{}d{} (skill {})",
                        cs.melee_dice, cs.melee_dice_sides, cs.melee_skill
                    )
                } else {
                    format!("skill {}", cs.melee_skill)
                };
                spawn_info_row(left, "Melee", &melee_str, theme.color(theme::Role::Text), 0);
            } else {
                spawn_info_row(left, "Melee", "—", theme.color(theme::Role::Muted), 0);
            }
            let dodge_str = dodge.map(|d| d.0.to_string()).unwrap_or_else(|| "—".into());
            spawn_info_row(left, "Dodge", &dodge_str, theme.color(theme::Role::Text), 1);
            if let Some(armor) = armor {
                let armor = &armor.0;
                let armor_str = format!(
                    "bash {} / cut {} / pierce {}",
                    armor.bash, armor.cut, armor.pierce
                );
                spawn_info_row(
                    left,
                    "Natural armor",
                    &armor_str,
                    theme.color(theme::Role::Muted),
                    0,
                );
            }
            if let Some(vis) = vision {
                spawn_info_row(
                    left,
                    "Vision",
                    &format!("{} / {} tiles", vis.day_range, vis.night_range),
                    theme.color(theme::Role::Text),
                    if combat.is_some() { 1 } else { 0 },
                );
            }

            // ── Status section ─────────────────────────────────────────────────
            spawn_section_header(left, "STATUS");
            if let Some(t) = temp {
                let (temp_str, temp_color) = temp_display(&theme, t.0);
                spawn_info_row(left, "Temp", &temp_str, temp_color, 0);
            }
            if let Some(w) = wet {
                let wet_str = match w.0 {
                    0 => "dry",
                    1..=3 => "damp",
                    4..=7 => "wet",
                    _ => "soaked",
                };
                spawn_info_row(left, "Wetness", wet_str, theme.color(theme::Role::Text), 1);
            }
            if let Some(m) = morale {
                let (morale_str, morale_color) = morale_display(&theme, m.0);
                spawn_info_row(left, "Morale", &morale_str, morale_color, 0);
            }

            // Status markers
            let mut status_parts: Vec<(&str, Color)> = Vec::new();
            if stunned.is_some() {
                status_parts.push(("STUNNED", theme::TEXT_YELLOW));
            }
            if bleeding.is_some() {
                status_parts.push(("BLEEDING", theme::TEXT_RED));
            }
            if on_fire.is_some() {
                status_parts.push(("ON FIRE", theme::TEXT_ORANGE));
            }

            if !status_parts.is_empty() {
                spawn_section_header(left, "CONDITIONS");
                for (i, (label, color)) in status_parts.iter().enumerate() {
                    left.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(5.0)),
                            ..default()
                        },
                        BackgroundColor(if i % 2 == 0 {
                            theme.color(theme::Role::Surface)
                        } else {
                            theme.color(theme::Role::Alternate)
                        }),
                    ))
                    .with_child((
                        Text::new(format!("  ● {}", label)),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(*color),
                    ));
                }
            }
        });
    }

    if data_changed || tab_changed {
        cache.rows.clear();
        cache.header.clear();
        cache.empty.clear();
        // Materialize display data only when the underlying ECS data or tab changes.
        match state.tab {
            CharacterTab::Skills => {
                let skills: Vec<SkillEntry> = creature_skills
                    .map(|cs| {
                        cs.iter()
                            .filter_map(|e| skill_entries.get(e).ok())
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                if skills.is_empty() {
                    cache.empty = "No skills learned yet.".to_string();
                } else {
                    cache.header = format!("{:<24}  {:>5}  {:>8}", "Skill", "Level", "XP");
                    for entry in &skills {
                        let row_str = format!(
                            "{:<24}  {:>5}  {:>8}",
                            format!("skill #{}", entry.skill_id.0),
                            entry.level,
                            entry.exercise,
                        );
                        cache.rows.push((row_str, theme::Role::Text));
                    }
                }
            }

            CharacterTab::Traits => {
                let trait_entries: Vec<(Entity, &MutationEntry)> = creature_mutations
                    .map(|cm| {
                        cm.iter()
                            .filter_map(|e| mutation_entries.get(e).ok().map(|m| (e, m)))
                            .collect()
                    })
                    .unwrap_or_default();

                if trait_entries.is_empty() {
                    cache.empty = "No traits or mutations.".to_string();
                } else {
                    cache.header = format!("{:<30}  {}", "Trait / Mutation", "Visible");
                    for (entity, entry) in &trait_entries {
                        let is_visible = visible_tags.get(*entity).is_ok();
                        let row_str = format!(
                            "{:<30}  {}",
                            format!("mutation #{}", entry.id.as_str()),
                            if is_visible { "yes" } else { "no" },
                        );
                        cache.rows.push((row_str, theme::Role::Text));
                    }
                }
            }

            CharacterTab::Effects => {
                let effects: Vec<StatusEffect> = active_effects
                    .map(|ae| {
                        ae.iter()
                            .filter_map(|e| effect_entries.get(e).ok())
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                if effects.is_empty() {
                    cache.empty = "No active effects.".to_string();
                } else {
                    cache.header = format!("{:<28}  {:>6}  {}", "Effect", "Intens", "Duration");
                    for entry in &effects {
                        let duration_str = format!("{}t", entry.remaining.0);
                        let row_str = format!(
                            "{:<28}  {:>6}  {}",
                            format!("effect #{}", entry.effect_id.as_str()),
                            entry.intensity,
                            duration_str,
                        );
                        let color = if entry.intensity > 3 {
                            theme::Role::Danger
                        } else if entry.intensity > 1 {
                            theme::Role::Warning
                        } else {
                            theme::Role::Text
                        };
                        cache.rows.push((row_str, color));
                    }
                }
            }

            CharacterTab::Bionics => {
                let bionic_entries: Vec<(Entity, &Bionic)> = installed_bionics
                    .map(|ib| {
                        ib.iter()
                            .filter_map(|e| bionic_entries.get(e).ok().map(|b| (e, b)))
                            .collect()
                    })
                    .unwrap_or_default();

                if bionic_entries.is_empty() {
                    cache.empty = "No bionics installed.".to_string();
                } else {
                    cache.header = format!("{:<30}  {:>8}  {}", "Bionic", "Power", "Active");
                    for (entity, entry) in &bionic_entries {
                        let is_active = active_tags.get(*entity).is_ok();
                        let row_str = format!(
                            "{:<30}  {:>8}  {}",
                            format!("bionic #{}", entry.bionic_id.as_str()),
                            entry.power_used.0,
                            if is_active { "yes" } else { "no" },
                        );
                        let color = if is_active {
                            theme::Role::Positive
                        } else {
                            theme::Role::Text
                        };
                        cache.rows.push((row_str, color));
                    }
                }
            }

            CharacterTab::Proficiencies => {
                let profs: Vec<ProficiencyEntry> = creature_profs
                    .map(|cp| {
                        cp.iter()
                            .filter_map(|e| proficiency_entries.get(e).ok())
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                if profs.is_empty() {
                    cache.empty = "No proficiencies known.".to_string();
                } else {
                    cache.header = "Proficiency".to_string();
                    for entry in &profs {
                        let row_str = format!("proficiency #{}", entry.id.as_str());
                        cache.rows.push((row_str, theme::Role::Text));
                    }
                }
            }
        }
    }
    if chrome_changed {
        // ── RIGHT PANEL ────────────────────────────────────────────────────────
        commands
            .entity(tabs_entity)
            .despawn_children()
            .with_children(|right| {
                // Tab bar
                right
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            width: Val::Percent(100.0),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        theme::SurfacePaint(theme::Role::Raised),
                        theme::BorderPaint(theme::Role::Border),
                    ))
                    .with_children(|tabs| {
                        for tab in CharacterTab::ALL {
                            let active = tab == state.tab;
                            tabs.spawn((
                                Node {
                                    padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                                    border: UiRect::right(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(if active {
                                    theme.tab_active_bg()
                                } else {
                                    theme.color(theme::Role::Surface)
                                }),
                                theme::BorderPaint(theme::Role::Border),
                            ))
                            .with_child((
                                Text::new(tab.label()),
                                TextFont {
                                    font_size: 15.0,
                                    ..default()
                                },
                                TextColor(if active {
                                    theme.color(theme::Role::Accent)
                                } else {
                                    theme.color(theme::Role::Muted)
                                }),
                            ));
                        }
                    });

                if !cache.header.is_empty() {
                    spawn_list_header(right, &cache.header);
                }
            });
    }
    sync_virtual_pane(
        &mut list,
        &mut focus,
        &mut position,
        computed,
        cache.rows.len(),
        state.scroll,
        tab_changed,
    );
    let mut rows = Vec::new();
    if cache.rows.is_empty() {
        rows.push((
            usize::MAX,
            TextRow {
                node: list.row_node(),
                background: theme.color(theme::Role::Surface),
                border: Color::NONE,
                cells: vec![RowCell::new(
                    cache.empty.clone(),
                    15.0,
                    theme.color(theme::Role::Muted),
                )],
            },
        ));
    }
    for index in list.window.0..list.window.1 {
        let (text, color) = &cache.rows[index];
        rows.push((
            index,
            TextRow {
                node: Node {
                    padding: UiRect::horizontal(Val::Px(18.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..list.row_node()
                },
                background: if index % 2 == 0 {
                    theme.color(theme::Role::Surface)
                } else {
                    theme.color(theme::Role::Alternate)
                },
                border: theme.color(theme::Role::Border),
                cells: vec![RowCell::new(text.clone(), 15.0, theme.color(*color))],
            },
        ));
    }
    retained.sync(&mut commands, content_entity, &list, rows);
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

pub fn character_sheet_input(
    mut reader: MessageReader<InputAction>,
    mut state: ResMut<CharacterSheetState>,
    pane: Query<&VirtualList, With<CharSheetContentContainer>>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    for action in actions {
        let last = pane
            .single()
            .map_or(0, |list| list.total_rows.saturating_sub(1));
        let selected = state.scroll.min(last);
        match action {
            GameAction::NavigateNextTab => {
                state.tab = state.tab.next();
                state.scroll = 0;
            }
            GameAction::NavigatePrevTab => {
                state.tab = state.tab.prev();
                state.scroll = 0;
            }
            GameAction::NavigateUp => {
                state.scroll = selected.saturating_sub(1);
            }
            GameAction::NavigateDown => {
                state.scroll = selected.saturating_add(1).min(last);
            }
            GameAction::NavigatePageUp => {
                state.scroll = selected.saturating_sub(10);
            }
            GameAction::NavigatePageDown => {
                state.scroll = selected.saturating_add(10).min(last);
            }
            GameAction::NavigateHome => {
                state.scroll = 0;
            }
            GameAction::NavigateEnd => {
                state.scroll = last;
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

fn spawn_section_header(parent: &mut ChildSpawnerCommands, title: &str) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(6.0), Val::Px(4.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            theme::SurfacePaint(theme::Role::Raised),
            theme::BorderPaint(theme::Role::Border),
        ))
        .with_child((
            Text::new(title),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            theme::TextPaint(theme::Role::Muted),
        ));
}

fn spawn_info_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    value: &str,
    color: Color,
    alt: usize,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(5.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            theme::SurfacePaint(if alt == 0 {
                theme::Role::Surface
            } else {
                theme::Role::Alternate
            }),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                theme::TextPaint(theme::Role::Muted),
            ));
            row.spawn((
                Text::new(value.to_string()),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(color),
            ));
        });
}

fn spawn_stat_row(
    theme: &UiTheme,
    parent: &mut ChildSpawnerCommands,
    name: &str,
    value: u32,
    alt: usize,
) {
    let bar = stat_bar(value);
    let color = stat_color(theme, value);
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            },
            theme::SurfacePaint(if alt == 0 {
                theme::Role::Surface
            } else {
                theme::Role::Alternate
            }),
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(38.0),
                    ..default()
                },
                // empty
            ))
            .with_child((
                Text::new(name),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                theme::TextPaint(theme::Role::Muted),
            ));
            row.spawn((Node {
                width: Val::Px(30.0),
                ..default()
            },))
                .with_child((
                    Text::new(format!("{:>2}", value)),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(color),
                ));
            row.spawn((
                Text::new(bar),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(color),
            ));
        });
}

fn spawn_list_header(parent: &mut ChildSpawnerCommands, label: &str) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(18.0), Val::Px(6.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            theme::SurfacePaint(theme::Role::Raised),
            theme::BorderPaint(theme::Role::Border),
        ))
        .with_child((
            Text::new(label.to_string()),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            theme::TextPaint(theme::Role::Muted),
        ));
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

fn stat_bar(value: u32) -> String {
    let filled = value.min(20) as usize;
    let empty = 20usize.saturating_sub(filled);
    "█".repeat(filled) + &"░".repeat(empty)
}

fn stat_color(theme: &UiTheme, value: u32) -> Color {
    if value >= 14 {
        theme::TEXT_GREEN
    } else if value >= 10 {
        theme.color(theme::Role::Accent)
    } else if value >= 8 {
        theme.color(theme::Role::Text)
    } else if value >= 6 {
        theme::TEXT_YELLOW
    } else {
        theme::TEXT_RED
    }
}

fn temp_display(theme: &UiTheme, celsius: f64) -> (String, Color) {
    let s = format!("{:.1}°C", celsius);
    let color = if celsius >= 40.0 {
        theme::TEXT_RED
    } else if celsius >= 38.5 {
        theme::TEXT_ORANGE
    } else if celsius < 35.0 {
        theme.color(theme::Role::Accent)
    } else {
        theme::TEXT_GREEN
    };
    (s, color)
}

fn morale_display(theme: &UiTheme, m: i32) -> (String, Color) {
    let label = if m >= 10 {
        "happy"
    } else if m >= 1 {
        "fine"
    } else if m == 0 {
        "neutral"
    } else if m >= -10 {
        "down"
    } else {
        "depressed"
    };
    let color = if m >= 10 {
        theme::TEXT_GREEN
    } else if m > 0 {
        theme.color(theme::Role::Accent)
    } else if m == 0 {
        theme.color(theme::Role::Text)
    } else if m >= -10 {
        theme::TEXT_YELLOW
    } else {
        theme::TEXT_RED
    };
    (format!("{} ({})", m, label), color)
}

#[derive(Default)]
pub struct CharacterPresentation {
    root: Option<Entity>,
    tab: CharacterTab,
    rows: Vec<(String, theme::Role)>,
    header: String,
    empty: String,
}

/// All sources displayed by this screen, including removals of optional data.
/// Changes invalidate the cached read model; scrolling never traverses the lists.
#[derive(SystemParam)]
pub struct CharacterChanges<'w, 's> {
    changed_overview: Query<
        'w,
        's,
        Entity,
        Or<(
            Changed<PlayerData>,
            Changed<Stats>,
            Changed<Health>,
            Changed<ActionPoints>,
            Changed<MeleeCapability>,
            Changed<DodgeDefense>,
            Changed<IntrinsicArmor>,
            Changed<Vision>,
            Changed<BodyTemperature>,
            Changed<Wetness>,
            Changed<Morale>,
            Changed<CreatureSkills>,
            Changed<CreatureMutations>,
            Changed<ActiveEffects>,
        )>,
    >,
    changed_details: Query<
        'w,
        's,
        Entity,
        Or<(
            Changed<InstalledBionics>,
            Changed<CreatureProficiencies>,
            Changed<Stunned>,
            Changed<Bleeding>,
            Changed<OnFire>,
            Changed<SkillEntry>,
            Changed<MutationEntry>,
            Changed<StatusEffect>,
            Changed<Bionic>,
            Changed<ProficiencyEntry>,
            Changed<Visible>,
            Changed<Active>,
        )>,
    >,
    removed_player_data: RemovedComponents<'w, 's, PlayerData>,
    removed_stats: RemovedComponents<'w, 's, Stats>,
    removed_health: RemovedComponents<'w, 's, Health>,
    removed_action_points: RemovedComponents<'w, 's, ActionPoints>,
    removed_melee: RemovedComponents<'w, 's, MeleeCapability>,
    removed_dodge: RemovedComponents<'w, 's, DodgeDefense>,
    removed_armor: RemovedComponents<'w, 's, IntrinsicArmor>,
    removed_vision: RemovedComponents<'w, 's, Vision>,
    removed_temperature: RemovedComponents<'w, 's, BodyTemperature>,
    removed_wetness: RemovedComponents<'w, 's, Wetness>,
    removed_morale: RemovedComponents<'w, 's, Morale>,
    removed_skills: RemovedComponents<'w, 's, CreatureSkills>,
    removed_mutations: RemovedComponents<'w, 's, CreatureMutations>,
    removed_effects: RemovedComponents<'w, 's, ActiveEffects>,
    removed_bionics: RemovedComponents<'w, 's, InstalledBionics>,
    removed_proficiencies: RemovedComponents<'w, 's, CreatureProficiencies>,
    removed_stunned: RemovedComponents<'w, 's, Stunned>,
    removed_bleeding: RemovedComponents<'w, 's, Bleeding>,
    removed_on_fire: RemovedComponents<'w, 's, OnFire>,
    removed_skill_entry: RemovedComponents<'w, 's, SkillEntry>,
    removed_mutation_entry: RemovedComponents<'w, 's, MutationEntry>,
    removed_status_effect: RemovedComponents<'w, 's, StatusEffect>,
    removed_bionic: RemovedComponents<'w, 's, Bionic>,
    removed_proficiency_entry: RemovedComponents<'w, 's, ProficiencyEntry>,
    removed_visible: RemovedComponents<'w, 's, Visible>,
    removed_active: RemovedComponents<'w, 's, Active>,
}
impl CharacterChanges<'_, '_> {
    fn changed(&mut self) -> bool {
        let mut removed = 0;
        removed += self.removed_player_data.read().count();
        removed += self.removed_stats.read().count();
        removed += self.removed_health.read().count();
        removed += self.removed_action_points.read().count();
        removed += self.removed_melee.read().count();
        removed += self.removed_dodge.read().count();
        removed += self.removed_armor.read().count();
        removed += self.removed_vision.read().count();
        removed += self.removed_temperature.read().count();
        removed += self.removed_wetness.read().count();
        removed += self.removed_morale.read().count();
        removed += self.removed_skills.read().count();
        removed += self.removed_mutations.read().count();
        removed += self.removed_effects.read().count();
        removed += self.removed_bionics.read().count();
        removed += self.removed_proficiencies.read().count();
        removed += self.removed_stunned.read().count();
        removed += self.removed_bleeding.read().count();
        removed += self.removed_on_fire.read().count();
        removed += self.removed_skill_entry.read().count();
        removed += self.removed_mutation_entry.read().count();
        removed += self.removed_status_effect.read().count();
        removed += self.removed_bionic.read().count();
        removed += self.removed_proficiency_entry.read().count();
        removed += self.removed_visible.read().count();
        removed += self.removed_active.read().count();
        removed > 0 || !self.changed_overview.is_empty() || !self.changed_details.is_empty()
    }
}

#[derive(SystemParam)]
pub struct CharacterContainers<'w, 's> {
    left_container: Query<'w, 's, Entity, With<CharSheetLeftContainer>>,
    content_container: Query<
        'w,
        's,
        (
            Entity,
            &'static mut VirtualList,
            &'static mut FocusedRow,
            &'static mut ScrollPosition,
            &'static ComputedNode,
            &'static mut RetainedRows<usize>,
        ),
        With<CharSheetContentContainer>,
    >,
    tabs_container: Query<'w, 's, Entity, With<CharSheetTabsContainer>>,
}
