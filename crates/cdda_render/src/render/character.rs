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

use super::FooterHint;
use crate::render::theme::{self, UiTheme};
use cdda_components::actor::Stats;
use cdda_components::actor::{
    ActionPoints, Active, ActiveEffects, Bionic, Bleeding, BodyTemperature, CombatStats,
    CreatureMutations, CreatureProficiencies, CreatureSkills, Health, InstalledBionics, Morale,
    MutationEntry, OnFire, PlayerData, ProficiencyEntry, SkillEntry, StatusEffect, Stunned,
    Visible, Vision, Wetness,
};
use cdda_components::context::ContextActions;
use cdda_components::dev::DevPlayer;
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
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
    let theme = world.resource::<UiTheme>().clone();
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
        BackgroundColor(theme::BG),
    ))
    .with_children(|root| {
        // ── Title bar ─────────────────────────────────────────────────
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(theme::HEADER_BG),
        ))
        .with_child((
            Text::new("CHARACTER SHEET"),
            TextFont {
                font_size: 26.0,
                ..default()
            },
            TextColor(theme.accent2()),
        ));

        // ── Main body ─────────────────────────────────────────────────
        root.spawn((Node {
            flex_direction: FlexDirection::Row,
            width: Val::Percent(100.0),
            flex_grow: 1.0,
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
                    BackgroundColor(theme::SIDE_PANEL_BG),
                    BorderColor::all(theme::DIVIDER),
                ))
                .with_children(|left| {
                    left.spawn((
                        CharSheetLeftContainer,
                        Node {
                            flex_direction: FlexDirection::Column,
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                });

                // ── RIGHT PANEL (tabs) ────────────────────────────────────
                main.spawn((Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                },))
                    .with_children(|right| {
                        right.spawn((
                            CharSheetContentContainer,
                            crate::render::scroll::KeyboardScroll,
                            crate::render::scroll::FocusedRow::default(),
                            bevy::ui::ScrollPosition::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
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
            BackgroundColor(theme::HEADER_BG),
            BorderColor::all(theme::DIVIDER),
        ))
        .with_child((
            Text::new(hints),
            super::ui_font(&font_handle, 13.0),
            TextColor(theme::TEXT_DIM),
            FooterHint,
        ));
    });
}

// ---------------------------------------------------------------------------
// Update — rebuild both panels every frame
// ---------------------------------------------------------------------------

pub fn update_character_sheet_screen(
    mut commands: Commands,
    state: Res<CharacterSheetState>,
    _ui_font_handle: Res<super::UiFontHandle>,
    theme: Res<UiTheme>,
    player_vitals: Query<
        (
            Option<&PlayerData>,
            Option<&Stats>,
            Option<&Health>,
            Option<&ActionPoints>,
            Option<&CombatStats>,
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
    left_container: Query<Entity, With<CharSheetLeftContainer>>,
    content_container: Query<Entity, With<CharSheetContentContainer>>,
) {
    let Ok(left_entity) = left_container.single() else {
        return;
    };
    let Ok(content_entity) = content_container.single() else {
        return;
    };

    commands.entity(left_entity).despawn_children();
    commands.entity(content_entity).despawn_children();

    // Extract player data (fallback to defaults when component absent).
    let (pdata, stats, health, ap, combat, vision, temp, wet, morale) = player_vitals
        .single()
        .unwrap_or((None, None, None, None, None, None, None, None, None));

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

    // ── LEFT PANEL ─────────────────────────────────────────────────────────
    commands.entity(left_entity).with_children(|left| {
        // ── Identity section ───────────────────────────────────────────────
        spawn_section_header(left, "IDENTITY");
        if let Some(pd) = pdata {
            spawn_info_row(left, "Name", &pd.name, theme::TEXT_BRIGHT, 0);
            let gender_str = match &pd.gender {
                cdda_components::actor::Gender::Male => "male",
                cdda_components::actor::Gender::Female => "female",
                cdda_components::actor::Gender::NonBinary => "non-binary",
                cdda_components::actor::Gender::Custom(s) => s.as_str(),
            };
            spawn_info_row(left, "Gender", gender_str, theme::TEXT_BRIGHT, 1);
            spawn_info_row(left, "Age", &format!("{}", pd.age), theme::TEXT_BRIGHT, 0);
            spawn_info_row(
                left,
                "Height",
                &format!("{} cm", pd.height),
                theme::TEXT_BRIGHT,
                1,
            );
            spawn_info_row(left, "Blood", &pd.blood_type, theme::TEXT_BRIGHT, 0);
        } else {
            spawn_info_row(left, "Name", "Dev Player", theme::TEXT_BRIGHT, 0);
        }

        // ── Attributes section ─────────────────────────────────────────────
        spawn_section_header(left, "ATTRIBUTES");
        spawn_stat_row(left, "STR", stats.strength, 0);
        spawn_stat_row(left, "DEX", stats.dexterity, 1);
        spawn_stat_row(left, "INT", stats.intelligence, 0);
        spawn_stat_row(left, "PER", stats.perception, 1);

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
            theme::TEXT_BRIGHT
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
            spawn_info_row(left, "Melee", &melee_str, theme::TEXT_BRIGHT, 0);
            spawn_info_row(
                left,
                "Dodge",
                &format!("{}", cs.dodge),
                theme::TEXT_BRIGHT,
                1,
            );
            let armor = &cs.armor;
            let armor_str = format!(
                "bash {} / cut {} / pierce {}",
                armor.bash, armor.cut, armor.pierce
            );
            spawn_info_row(left, "Armor", &armor_str, theme::TEXT_DIM, 0);
        } else {
            spawn_info_row(left, "Melee", "—", theme::TEXT_DIM, 0);
            spawn_info_row(left, "Dodge", "—", theme::TEXT_DIM, 1);
        }
        if let Some(vis) = vision {
            spawn_info_row(
                left,
                "Vision",
                &format!("{} / {} tiles", vis.day_range, vis.night_range),
                theme::TEXT_BRIGHT,
                if combat.is_some() { 1 } else { 0 },
            );
        }

        // ── Status section ─────────────────────────────────────────────────
        spawn_section_header(left, "STATUS");
        if let Some(t) = temp {
            let (temp_str, temp_color) = temp_display(t.0);
            spawn_info_row(left, "Temp", &temp_str, temp_color, 0);
        }
        if let Some(w) = wet {
            let wet_str = match w.0 {
                0 => "dry",
                1..=3 => "damp",
                4..=7 => "wet",
                _ => "soaked",
            };
            spawn_info_row(left, "Wetness", wet_str, theme::TEXT_BRIGHT, 1);
        }
        if let Some(m) = morale {
            let (morale_str, morale_color) = morale_display(m.0);
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
                        theme::PANEL_BG
                    } else {
                        theme::ROW_ALT_BG
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

    // ── RIGHT PANEL ────────────────────────────────────────────────────────
    commands.entity(content_entity).with_children(|right| {
        // Tab bar
        right
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    width: Val::Percent(100.0),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::HEADER_BG),
                BorderColor::all(theme::DIVIDER),
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
                            theme::TAB_BG
                        }),
                        BorderColor::all(theme::DIVIDER),
                    ))
                    .with_child((
                        Text::new(tab.label()),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(if active {
                            theme::TAB_TEXT_ACTIVE
                        } else {
                            theme::TEXT_DIM
                        }),
                    ));
                }
            });

        // Tab content
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
                    spawn_empty_message(right, "No skills learned yet.");
                } else {
                    spawn_list_header(
                        right,
                        &format!("{:<24}  {:>5}  {:>8}", "Skill", "Level", "XP"),
                    );
                    for (i, entry) in skills.iter().enumerate() {
                        let row_str = format!(
                            "{:<24}  {:>5}  {:>8}",
                            format!("skill #{}", entry.skill_id.0),
                            entry.level,
                            entry.exercise,
                        );
                        spawn_content_row(right, &row_str, i % 2 == 0, theme::TEXT_BRIGHT);
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
                    spawn_empty_message(right, "No traits or mutations.");
                } else {
                    spawn_list_header(right, &format!("{:<30}  {}", "Trait / Mutation", "Visible"));
                    for (i, (entity, entry)) in trait_entries.iter().enumerate() {
                        let is_visible = visible_tags.get(*entity).is_ok();
                        let row_str = format!(
                            "{:<30}  {}",
                            format!("mutation #{}", entry.id.as_str()),
                            if is_visible { "yes" } else { "no" },
                        );
                        spawn_content_row(right, &row_str, i % 2 == 0, theme::TEXT_BRIGHT);
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
                    spawn_empty_message(right, "No active effects.");
                } else {
                    spawn_list_header(
                        right,
                        &format!("{:<28}  {:>6}  {}", "Effect", "Intens", "Duration"),
                    );
                    for (i, entry) in effects.iter().enumerate() {
                        let duration_str = format!("{}t", entry.remaining.0);
                        let row_str = format!(
                            "{:<28}  {:>6}  {}",
                            format!("effect #{}", entry.effect_id.as_str()),
                            entry.intensity,
                            duration_str,
                        );
                        let color = if entry.intensity > 3 {
                            theme::TEXT_RED
                        } else if entry.intensity > 1 {
                            theme::TEXT_YELLOW
                        } else {
                            theme::TEXT_BRIGHT
                        };
                        spawn_content_row(right, &row_str, i % 2 == 0, color);
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
                    spawn_empty_message(right, "No bionics installed.");
                } else {
                    spawn_list_header(
                        right,
                        &format!("{:<30}  {:>8}  {}", "Bionic", "Power", "Active"),
                    );
                    for (i, (entity, entry)) in bionic_entries.iter().enumerate() {
                        let is_active = active_tags.get(*entity).is_ok();
                        let row_str = format!(
                            "{:<30}  {:>8}  {}",
                            format!("bionic #{}", entry.bionic_id.as_str()),
                            entry.power_used.0,
                            if is_active { "yes" } else { "no" },
                        );
                        let color = if is_active {
                            theme::TEXT_GREEN
                        } else {
                            theme::TEXT_BRIGHT
                        };
                        spawn_content_row(right, &row_str, i % 2 == 0, color);
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
                    spawn_empty_message(right, "No proficiencies known.");
                } else {
                    spawn_list_header(right, "Proficiency");
                    for (i, entry) in profs.iter().enumerate() {
                        let row_str = format!("proficiency #{}", entry.id.as_str());
                        spawn_content_row(right, &row_str, i % 2 == 0, theme::TEXT_BRIGHT);
                    }
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

pub fn character_sheet_input(
    mut reader: MessageReader<InputAction>,
    mut state: ResMut<CharacterSheetState>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    for action in actions {
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
                state.scroll = state.scroll.saturating_sub(1);
            }
            GameAction::NavigateDown => {
                state.scroll += 1;
            }
            GameAction::NavigatePageUp => {
                state.scroll = state.scroll.saturating_sub(10);
            }
            GameAction::NavigatePageDown => {
                state.scroll += 10;
            }
            GameAction::NavigateHome => {
                state.scroll = 0;
            }
            _ => {}
        }
    }
}

/// Feed `CharacterSheetState.scroll` (the focused-row index) into the pane's
/// `FocusedRow`, so the shared `scroll::scroll_to_focused_row` keeps the focused
/// row visible within the native `ScrollPosition` pane.
pub fn sync_character_scroll(
    state: Res<CharacterSheetState>,
    mut pane: Query<&mut crate::render::scroll::FocusedRow, With<CharSheetContentContainer>>,
) {
    if let Ok(mut focus) = pane.single_mut() {
        focus.0 = state.scroll;
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
            BackgroundColor(theme::SECTION_HEADER_BG),
            BorderColor::all(theme::DIVIDER),
        ))
        .with_child((
            Text::new(title),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(theme::TEXT_DIM),
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
            BackgroundColor(if alt == 0 {
                theme::PANEL_BG
            } else {
                theme::ROW_ALT_BG
            }),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
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

fn spawn_stat_row(parent: &mut ChildSpawnerCommands, name: &str, value: u32, alt: usize) {
    let bar = stat_bar(value);
    let color = stat_color(value);
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(if alt == 0 {
                theme::PANEL_BG
            } else {
                theme::ROW_ALT_BG
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
                TextColor(theme::TEXT_DIM),
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
            BackgroundColor(theme::HEADER_BG),
            BorderColor::all(theme::DIVIDER),
        ))
        .with_child((
            Text::new(label.to_string()),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(theme::TEXT_DIM),
        ));
}

fn spawn_content_row(parent: &mut ChildSpawnerCommands, text: &str, even: bool, color: Color) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(18.0), Val::Px(7.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(if even {
                theme::PANEL_BG
            } else {
                theme::ROW_ALT_BG
            }),
            BorderColor::all(theme::DIVIDER),
        ))
        .with_child((
            Text::new(text.to_string()),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(color),
        ));
}

fn spawn_empty_message(parent: &mut ChildSpawnerCommands, msg: &str) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(24.0), Val::Px(20.0)),
            ..default()
        },))
        .with_child((
            Text::new(msg.to_string()),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(theme::TEXT_DIM),
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

fn stat_color(value: u32) -> Color {
    if value >= 14 {
        theme::TEXT_GREEN
    } else if value >= 10 {
        theme::TAB_TEXT_ACTIVE
    } else if value >= 8 {
        theme::TEXT_BRIGHT
    } else if value >= 6 {
        theme::TEXT_YELLOW
    } else {
        theme::TEXT_RED
    }
}

fn temp_display(celsius: f64) -> (String, Color) {
    let s = format!("{:.1}°C", celsius);
    let color = if celsius >= 40.0 {
        theme::TEXT_RED
    } else if celsius >= 38.5 {
        theme::TEXT_ORANGE
    } else if celsius < 35.0 {
        theme::TAB_TEXT_ACTIVE
    } else {
        theme::TEXT_GREEN
    };
    (s, color)
}

fn morale_display(m: i32) -> (String, Color) {
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
        theme::TAB_TEXT_ACTIVE
    } else if m == 0 {
        theme::TEXT_BRIGHT
    } else if m >= -10 {
        theme::TEXT_YELLOW
    } else {
        theme::TEXT_RED
    };
    (format!("{} ({})", m, label), color)
}
