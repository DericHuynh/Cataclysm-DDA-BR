//! Exercises the persistent production schedule, not isolated run_system calls.
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_state::state::State;
use bevy_time::Time;
use cdda_components::activity::{ActivityPhase, ActivityProgress, Waiting};
use cdda_components::actor::{ActionPoints, IsAlive, StatusEffect};
use cdda_components::intent::{ActionIntent, ActionOutcome, ActionOutcomeState};
use cdda_components::schedule::{SimSet, SimulationTurn};
use cdda_components::sim::{GameTime, WorldPosition};
use cdda_core_types::core::coords::{WorldPos, ZLevel};
use cdda_sim::runtime::state::AppState;
use cdda_sim::runtime::{step_simulation, SimulationControl, SimulationMode, SimulationPlugin};
use std::time::Duration;

fn app(mode: SimulationMode) -> App {
    let mut app = App::new();
    app.add_plugins(SimulationPlugin);
    app.world_mut().resource_mut::<SimulationControl>().mode = mode;
    app
}

fn actor(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            ActionPoints {
                current: 0,
                speed: 100,
            },
            IsAlive,
            WorldPosition::new(WorldPos::new(0, 0, ZLevel::new(0))),
        ))
        .id()
}

fn effect(app: &mut App) -> Entity {
    app.world_mut()
        .spawn(StatusEffect {
            effect_id: cdda_components::EffectId::new("poison"),
            intensity: 1,
            remaining: cdda_components::Time::from_turns(10),
        })
        .id()
}

#[test]
fn render_updates_without_input_do_not_advance_any_simulation_time() {
    let mut app = app(SimulationMode::TurnBased);
    let actor = actor(&mut app);
    let effect = effect(&mut app);
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(app.world().resource::<GameTime>().turn, 0);
    assert_eq!(app.world().get::<ActionPoints>(actor).unwrap().current, 0);
    assert_eq!(
        app.world()
            .get::<StatusEffect>(effect)
            .unwrap()
            .remaining
            .as_turns(),
        10
    );
}

#[test]
fn one_declared_action_advances_once_and_commits_before_returning() {
    let mut app = app(SimulationMode::TurnBased);
    let actor = actor(&mut app);
    let effect = effect(&mut app);
    app.world_mut()
        .entity_mut(actor)
        .insert(ActionIntent::Move { dx: 1, dy: 0 });
    app.update();
    assert_eq!(app.world().resource::<GameTime>().turn, 1);
    assert_eq!(app.world().get::<WorldPosition>(actor).unwrap().get().x, 1);
    assert_eq!(app.world().get::<ActionPoints>(actor).unwrap().current, 0);
    assert_eq!(
        app.world().get::<ActionOutcome>(actor).unwrap().state,
        ActionOutcomeState::Completed
    );
    assert_eq!(
        app.world()
            .get::<StatusEffect>(effect)
            .unwrap()
            .remaining
            .as_turns(),
        9
    );
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(app.world().resource::<GameTime>().turn, 1);
}

#[test]
fn manual_steps_use_persistent_system_state_and_run_no_extra_frame_turns() {
    #[derive(Resource, Default)]
    struct Seen(Vec<u32>);
    fn observe(mut local: Local<u32>, mut seen: ResMut<Seen>) {
        *local += 1;
        seen.0.push(*local);
    }
    let mut app = app(SimulationMode::Manual);
    app.init_resource::<Seen>();
    app.add_systems(SimulationTurn, observe.in_set(SimSet::Effects));
    app.world_mut()
        .resource_mut::<SimulationControl>()
        .request_steps(3);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<GameTime>().turn, 3);
    assert_eq!(app.world().resource::<Seen>().0, vec![1, 2, 3]);
    assert!(step_simulation(app.world_mut()));
    assert_eq!(app.world().resource::<Seen>().0, vec![1, 2, 3, 4]);
}

#[test]
fn pause_gates_ai_intents_activities_effects_and_ap_together() {
    let mut app = app(SimulationMode::Manual);
    let actor = actor(&mut app);
    let effect = effect(&mut app);
    app.world_mut().entity_mut(actor).insert((
        ActionIntent::Wait,
        Waiting { turns: 5 },
        ActivityProgress {
            moves_total: 500,
            moves_left: 500,
            phase: ActivityPhase::Active,
        },
    ));
    app.world_mut().resource_mut::<SimulationControl>().paused = true;
    app.world_mut()
        .resource_mut::<SimulationControl>()
        .request_steps(2);
    app.update();
    assert!(!step_simulation(app.world_mut()));
    assert_eq!(app.world().resource::<GameTime>().turn, 0);
    assert!(app.world().get::<ActionIntent>(actor).is_some());
    assert_eq!(app.world().get::<ActionPoints>(actor).unwrap().current, 0);
    assert_eq!(
        app.world()
            .get::<ActivityProgress>(actor)
            .unwrap()
            .moves_left,
        500
    );
    assert_eq!(
        app.world()
            .get::<StatusEffect>(effect)
            .unwrap()
            .remaining
            .as_turns(),
        10
    );
    app.world_mut().resource_mut::<SimulationControl>().paused = false;
    app.update();
    assert_eq!(app.world().resource::<GameTime>().turn, 2);
    assert_eq!(
        app.world()
            .get::<StatusEffect>(effect)
            .unwrap()
            .remaining
            .as_turns(),
        8
    );
}

#[test]
fn application_state_uses_the_same_central_gate() {
    let mut app = app(SimulationMode::Manual);
    for state in [
        AppState::MainMenu,
        AppState::DataLoading,
        AppState::WorldGen,
        AppState::Paused,
        AppState::GameOver,
    ] {
        app.world_mut().insert_resource(State::new(state));
        assert!(!step_simulation(app.world_mut()));
    }
    app.world_mut()
        .insert_resource(State::new(AppState::InGame));
    assert!(step_simulation(app.world_mut()));
    assert_eq!(app.world().resource::<GameTime>().turn, 1);
}

fn wall_frame(app: &mut App, millis: u64) {
    let mut time = app.world_mut().resource_mut::<Time>();
    time.advance_by(Duration::from_millis(millis));
    app.update();
}

#[test]
fn real_time_frame_partition_does_not_change_turn_or_effect_counts() {
    fn run(frames: &[u64]) -> (u64, i64) {
        let mut app = app(SimulationMode::RealTime);
        app.insert_resource(Time::<()>::default());
        let effect = effect(&mut app);
        for &frame in frames {
            wall_frame(&mut app, frame);
        }
        (
            app.world().resource::<GameTime>().turn,
            app.world()
                .get::<StatusEffect>(effect)
                .unwrap()
                .remaining
                .as_turns(),
        )
    }
    assert_eq!(run(&[50; 10]), (5, 5));
    assert_eq!(run(&[500]), (5, 5));
}

#[test]
fn real_time_catchup_is_bounded_and_pause_does_not_bank_wall_time() {
    let mut app = app(SimulationMode::RealTime);
    app.insert_resource(Time::<()>::default());
    app.world_mut()
        .resource_mut::<SimulationControl>()
        .max_steps_per_update = 3;
    wall_frame(&mut app, 1000);
    assert_eq!(app.world().resource::<GameTime>().turn, 3);
    wall_frame(&mut app, 0);
    assert_eq!(app.world().resource::<GameTime>().turn, 6);
    app.world_mut().resource_mut::<SimulationControl>().paused = true;
    wall_frame(&mut app, 5000);
    app.world_mut().resource_mut::<SimulationControl>().paused = false;
    wall_frame(&mut app, 0);
    assert_eq!(app.world().resource::<GameTime>().turn, 6);
    wall_frame(&mut app, 100);
    assert_eq!(app.world().resource::<GameTime>().turn, 7);
}

#[test]
fn game_calendar_matches_definition_duration_units() {
    let hour: cdda_components::Time = serde_json::from_str("\"1 h\"").unwrap();
    assert_eq!(hour.as_turns() as u64, GameTime::TURNS_PER_HOUR);
    assert_eq!(
        GameTime {
            turn: hour.as_turns() as u64
        }
        .hours_elapsed(),
        1
    );
    assert_eq!(GameTime::TURNS_PER_DAY, 24 * hour.as_turns() as u64);
}

fn walker(app: &mut App, speed_ap: i32) -> Entity {
    app.world_mut()
        .spawn((
            ActionPoints {
                current: 0,
                speed: speed_ap,
            },
            IsAlive,
            WorldPosition::new(WorldPos::new(0, 0, ZLevel::new(0))),
        ))
        .id()
}

#[test]
fn faster_actors_commit_more_actions_per_world_turn() {
    use cdda_components::ai::PlannerBehaviourTree;
    let mut app = app(SimulationMode::Manual);
    // The BT stand-in re-declares Wander on every selection, so a fast actor
    // spends its full budget inside one world turn while a slow one cannot.
    let mut walker = |app: &mut App, speed_ap: i32| {
        app.world_mut()
            .spawn((
                ActionPoints {
                    current: 0,
                    speed: speed_ap,
                },
                IsAlive,
                PlannerBehaviourTree,
                WorldPosition::new(WorldPos::new(0, 0, ZLevel::new(0))),
            ))
            .id()
    };
    let fast = walker(&mut app, 200);
    let slow = walker(&mut app, 100);
    app.world_mut()
        .resource_mut::<SimulationControl>()
        .request_steps(1);
    app.update();
    assert_eq!(app.world().get::<WorldPosition>(fast).unwrap().get().x, 2);
    assert_eq!(app.world().get::<WorldPosition>(slow).unwrap().get().x, 1);
    assert_eq!(app.world().resource::<GameTime>().turn, 1);
}

#[test]
fn rejected_actions_do_not_replay_within_one_turn() {
    let mut app = app(SimulationMode::Manual);
    let actor = walker(&mut app, 500);
    app.world_mut().spawn((
        WorldPosition::new(WorldPos::new(1, 0, ZLevel::new(0))),
        cdda_components::sim::Solid,
    ));
    app.world_mut()
        .entity_mut(actor)
        .insert(ActionIntent::Move { dx: 1, dy: 0 });
    app.world_mut()
        .resource_mut::<SimulationControl>()
        .request_steps(1);
    app.update();
    assert_eq!(app.world().get::<WorldPosition>(actor).unwrap().get().x, 0);
    assert_eq!(app.world().get::<ActionPoints>(actor).unwrap().current, 500);
    assert_eq!(app.world().resource::<GameTime>().turn, 1);
}

#[test]
fn crafters_act_through_their_activity_not_the_action_budget() {
    use cdda_components::activity::ActivityPhase;
    let mut app = app(SimulationMode::Manual);
    let crafter = walker(&mut app, 100);
    app.world_mut()
        .entity_mut(crafter)
        .insert(ActionIntent::Wait);
    app.world_mut()
        .entity_mut(crafter)
        .insert(ActivityProgress {
            moves_total: 300,
            moves_left: 200,
            phase: ActivityPhase::Active,
        });
    app.world_mut()
        .resource_mut::<SimulationControl>()
        .request_steps(1);
    app.update();
    // Activity actors are excluded from budget selection: their intent waits
    // and their AP grant is untouched by action resolution this turn.
    assert!(app.world().get::<ActionIntent>(crafter).is_some());
    assert_eq!(
        app.world().get::<ActionPoints>(crafter).unwrap().current,
        100
    );
    assert_eq!(app.world().resource::<GameTime>().turn, 1);
}
