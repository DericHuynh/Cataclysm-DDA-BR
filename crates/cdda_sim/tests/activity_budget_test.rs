//! Activity/action arbitration through the production persistent headless schedule.
use bevy_app::App;
use bevy_ecs::prelude::*;
use cdda_components::{activity::*, actor::ActionPoints, intent::*, sim::GameTime};
use cdda_sim::runtime::{step_simulation, SimulationControl, SimulationPlugin};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(SimulationPlugin);
    app
}

#[test]
fn elapsed_time_waits_charge_fast_and_slow_actors_once_per_turn() {
    let mut app = app();
    let w = app.world_mut();
    let actors: Vec<_> = [25, 100, 300]
        .into_iter()
        .map(|speed| {
            w.spawn((
                ActionPoints { current: 0, speed },
                Waiting { turns: 2 },
                ActivityProgress::default(),
                ActionIntent::Wait,
            ))
            .id()
        })
        .collect();
    step_simulation(w);
    for &actor in &actors {
        assert_eq!(w.get::<ActivityProgress>(actor).unwrap().moves_left, 100);
        assert_eq!(w.get::<ActionPoints>(actor).unwrap().current, 0);
        assert!(w.get::<ActionIntent>(actor).is_some());
    }
    step_simulation(w);
    for &actor in &actors {
        assert!(w.get::<ActivityProgress>(actor).is_none());
        assert!(w.get::<Waiting>(actor).is_none());
        assert_eq!(w.get::<ActionPoints>(actor).unwrap().current, 0);
        assert!(w.get::<ActionOutcome>(actor).is_none());
    }
}

/// Master player_activity.cpp TIME branch truncates moves * remaining / 100.
/// These neutral-exertion expectations include a completion costing zero moves.
#[test]
fn master_partial_time_completion_truncates_cost_and_allows_the_next_action() {
    for (budget, remaining, expected) in [(25, 50, 13), (1, 1, 1), (101, 99, 2)] {
        for queued_action in [false, true] {
            let mut app = app();
            let w = app.world_mut();
            let actor = w
                .spawn((
                    ActionPoints {
                        current: 0,
                        speed: budget,
                    },
                    Waiting { turns: 1 },
                    ActivityProgress {
                        moves_total: 100,
                        moves_left: remaining,
                        phase: ActivityPhase::Active,
                    },
                ))
                .id();
            if queued_action {
                w.entity_mut(actor).insert(ActionIntent::Wait);
            }
            step_simulation(w);
            assert!(w.get::<ActivityProgress>(actor).is_none());
            assert_eq!(
                w.get::<ActionPoints>(actor).unwrap().current,
                expected - if queued_action { 100 } else { 0 }
            );
            assert_eq!(w.get::<ActionOutcome>(actor).is_some(), queued_action);
        }
    }
}

#[test]
fn partial_final_activity_spends_exact_work_then_action_uses_remainder() {
    let mut app = app();
    let w = app.world_mut();
    let actor = w
        .spawn((
            ActionPoints {
                current: 0,
                speed: 100,
            },
            Reloading {
                item_entity: Entity::PLACEHOLDER,
                ammo_entity: Entity::PLACEHOLDER,
                quantity: 1,
                speed_factor: 1.0,
            },
            ActivityProgress {
                moves_total: 150,
                moves_left: 150,
                phase: ActivityPhase::Active,
            },
            ActionIntent::Wait,
        ))
        .id();
    step_simulation(w);
    assert_eq!(w.get::<ActivityProgress>(actor).unwrap().moves_left, 50);
    assert_eq!(w.get::<ActionPoints>(actor).unwrap().current, 0);
    step_simulation(w);
    assert!(w.get::<ActivityProgress>(actor).is_none());
    assert_eq!(
        w.get::<ActionOutcome>(actor).unwrap().state,
        ActionOutcomeState::Completed
    );
    assert_eq!(
        w.get::<ActionPoints>(actor).unwrap().current,
        -50,
        "50 work + 100 wait from 100 AP"
    );
}

#[test]
fn suspended_activity_does_not_wake_turns_or_spend_budget_and_pause_retains_work() {
    let mut app = app();
    let actor = app
        .world_mut()
        .spawn((
            ActionPoints {
                current: 0,
                speed: 100,
            },
            Waiting { turns: 2 },
            ActivityProgress {
                moves_total: 200,
                moves_left: 200,
                phase: ActivityPhase::Suspended,
            },
        ))
        .id();
    app.update();
    app.update();
    assert_eq!(app.world().resource::<GameTime>().turn, 0);
    app.world_mut()
        .get_mut::<ActivityProgress>(actor)
        .unwrap()
        .phase = ActivityPhase::Active;
    app.world_mut().resource_mut::<SimulationControl>().paused = true;
    app.update();
    assert_eq!(app.world().resource::<GameTime>().turn, 0);
    app.world_mut().resource_mut::<SimulationControl>().paused = false;
    app.update();
    assert_eq!(
        app.world()
            .get::<ActivityProgress>(actor)
            .unwrap()
            .moves_left,
        100
    );
    assert_eq!(app.world().get::<ActionPoints>(actor).unwrap().current, 0);
}

#[test]
fn malformed_multiple_activity_types_cannot_double_spend_shared_progress() {
    let mut app = app();
    let actor = app
        .world_mut()
        .spawn((
            ActionPoints {
                current: 0,
                speed: 100,
            },
            Waiting { turns: 2 },
            Interacting {
                description: "invalid duplicate".into(),
                duration: 2,
            },
            ActivityProgress::default(),
        ))
        .id();
    step_simulation(app.world_mut());
    assert_eq!(app.world().get::<ActionPoints>(actor).unwrap().current, 100);
    assert_eq!(
        app.world().get::<ActivityProgress>(actor).unwrap().phase,
        ActivityPhase::Pending
    );
}

#[test]
fn more_than_sixty_four_actors_each_receive_their_activity_budget() {
    let mut app = app();
    let w = app.world_mut();
    let actors: Vec<_> = (0..100)
        .map(|_| {
            w.spawn((
                ActionPoints {
                    current: 0,
                    speed: 100,
                },
                Waiting { turns: 2 },
                ActivityProgress::default(),
            ))
            .id()
        })
        .collect();
    step_simulation(w);
    for actor in actors {
        assert_eq!(w.get::<ActionPoints>(actor).unwrap().current, 0);
        assert_eq!(w.get::<ActivityProgress>(actor).unwrap().moves_left, 100);
    }
}

#[test]
fn aiming_uses_speed_while_reading_and_interaction_use_elapsed_time() {
    let mut app = app();
    let w = app.world_mut();
    let aim = w
        .spawn((
            ActionPoints {
                current: 0,
                speed: 200,
            },
            ActivityProgress::default(),
            Aiming {
                target_aim_percent: 10,
                cur_aim: 0,
            },
        ))
        .id();
    let read = w
        .spawn((
            ActionPoints {
                current: 0,
                speed: 200,
            },
            ActivityProgress::default(),
            Reading {
                book_entity: Entity::PLACEHOLDER,
                skill_id: "test".into(),
                turns_read: 0,
                turns_total: 2,
            },
        ))
        .id();
    let interact = w
        .spawn((
            ActionPoints {
                current: 0,
                speed: 200,
            },
            ActivityProgress::default(),
            Interacting {
                description: "test".into(),
                duration: 2,
            },
        ))
        .id();
    step_simulation(w);
    assert!(w.get::<Aiming>(aim).is_none());
    assert!(w.get::<ActivityProgress>(aim).is_none());
    assert_eq!(w.get::<Reading>(read).unwrap().turns_read, 1);
    for actor in [aim, read, interact] {
        assert_eq!(w.get::<ActionPoints>(actor).unwrap().current, 0);
    }
    for actor in [read, interact] {
        assert_eq!(w.get::<ActivityProgress>(actor).unwrap().moves_left, 100);
    }
    step_simulation(w);
    for actor in [read, interact] {
        assert!(w.get::<ActivityProgress>(actor).is_none());
        assert_eq!(w.get::<ActionPoints>(actor).unwrap().current, 0);
    }
}
