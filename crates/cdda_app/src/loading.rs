//! Asynchronous disk/JSON work and frame-separated ECS publication.
use bevy_ecs::prelude::*;
use bevy_state::state::NextState;
use cdda_components::progress::{OperationReport, ReportEvent, ReportLevel};
use cdda_data::{DefRegistry, Loader};
use cdda_sim::runtime::state::{AppState, StartupConfig};
use std::sync::{mpsc, Mutex};

enum WorkerMessage {
    Report(ReportEvent),
    Ready(DefRegistry, cdda_data::raw_values::RawDefinitionValues),
}
#[derive(Resource)]
pub struct LoadingJob(Mutex<mpsc::Receiver<WorkerMessage>>);
#[derive(Resource)]
enum Publication {
    Validate(DefRegistry),
    Definitions(DefRegistry, cdda_overmap::registry::TerrainRegistry),
    Registries(
        DefRegistry,
        cdda_overmap::registry::TerrainRegistry,
        cdda_data::def_world::DefinitionWorld,
    ),
}

pub fn publish_report(world: &mut World, event: ReportEvent) {
    // Exactly the same record can be rendered or consumed in a headless run.
    eprintln!("{event}");
    world.resource_mut::<OperationReport>().record(event);
}

pub fn begin_loading(world: &mut World) {
    world.insert_resource(OperationReport::default());
    world.remove_resource::<Publication>();
    if let Some(mut next) =
        world.get_resource_mut::<NextState<cdda_overmap_gen::pipeline::OvermapGenPhase>>()
    {
        next.set(cdda_overmap_gen::pipeline::OvermapGenPhase::Idle);
    }
    publish_report(
        world,
        ReportEvent::progress("Preparing", "Starting definition loader"),
    );
    let dirs = world.resource::<StartupConfig>().data_dirs.clone();
    let (sender, receiver) = mpsc::sync_channel(128);
    world.insert_resource(LoadingJob(Mutex::new(receiver)));
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut loader = Loader::new(dirs);
            let registry = loader.load_reported(|event| {
                let _ = sender.send(WorkerMessage::Report(event));
            });
            if let Ok(registry) = registry {
                if registry.total_count() == 0 {
                    let _ = sender.send(WorkerMessage::Report(
                        ReportEvent::progress(
                            "Loading failed",
                            "No usable definitions were loaded",
                        )
                        .level(ReportLevel::Error),
                    ));
                    return;
                }
                let mut raw = cdda_data::raw_values::RawDefinitionValues::new();
                for (kind, defs) in loader.raw_by_type() {
                    raw.values.insert(
                        kind.clone(),
                        defs.iter()
                            .filter_map(|def| {
                                def.id.as_ref().map(|id| (id.clone(), def.value.clone()))
                            })
                            .collect(),
                    );
                }
                let _ = sender.send(WorkerMessage::Ready(registry, raw));
            }
        }));
        if result.is_err() {
            let _ = sender.send(WorkerMessage::Report(
                ReportEvent::progress("Loading failed", "Definition worker stopped unexpectedly")
                    .level(ReportLevel::Error),
            ));
        }
    });
}

pub fn poll_loading(world: &mut World) {
    if world.resource::<OperationReport>().failed() {
        world.remove_resource::<Publication>();
    }
    if world.resource::<OperationReport>().cancelled {
        return;
    }
    if let Some(publication) = world.remove_resource::<Publication>() {
        match publication {
            Publication::Validate(registry) => {
                match crate::startup::build_terrain_registry(
                    &registry,
                    world.get_resource::<cdda_overmap::registry::TerrainRegistry>(),
                ) {
                    Ok(terrain) => {
                        world.insert_resource(Publication::Definitions(registry, terrain));
                        publish_report(
                            world,
                            ReportEvent::progress(
                                "Building definition entities",
                                "Converting resolved definitions into ECS capabilities",
                            ),
                        );
                    }
                    Err(error) => publish_report(
                        world,
                        ReportEvent::progress("Publication failed", error.to_string())
                            .level(ReportLevel::Error),
                    ),
                }
            }
            Publication::Definitions(registry, terrain) => {
                let definitions = cdda_data::def_world::build_def_world(world, &registry, true);
                world.insert_resource(Publication::Registries(registry, terrain, definitions));
                publish_report(
                    world,
                    ReportEvent::progress(
                        "Building registries",
                        "Flags, schemas, terrain, cities and region settings",
                    ),
                );
            }
            Publication::Registries(registry, terrain, definitions) => {
                crate::startup::finish_registry_publication(
                    world,
                    &registry,
                    registry.total_count(),
                    terrain,
                    definitions,
                );
            }
        }
        return;
    }
    let Some(job) = world.get_resource::<LoadingJob>() else {
        return;
    };
    let mut messages = Vec::new();
    let mut disconnected = false;
    {
        let receiver = job.0.lock().unwrap();
        for _ in 0..64 {
            match receiver.try_recv() {
                Ok(message) => messages.push(message),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
    }
    for message in messages {
        match message {
            WorkerMessage::Report(event) => publish_report(world, event),
            WorkerMessage::Ready(registry, raw) => {
                world.insert_resource(raw);
                world.insert_resource(Publication::Validate(registry));
                world.remove_resource::<LoadingJob>();
                // Render the publication phase before the exclusive ECS commit.
                publish_report(
                    world,
                    ReportEvent::progress(
                        "Validating registries",
                        "Checking terrain identities before publication",
                    ),
                );
            }
        }
    }
    if disconnected && world.contains_resource::<LoadingJob>() {
        world.remove_resource::<LoadingJob>();
        if !world.resource::<OperationReport>().failed()
            && !world.contains_resource::<Publication>()
        {
            publish_report(
                world,
                ReportEvent::progress(
                    "Loading failed",
                    "Definition worker disconnected without a result",
                )
                .level(ReportLevel::Error),
            );
        }
    }
}

pub fn retry_loading(world: &mut World) {
    begin_loading(world);
}
pub fn leave_loading(world: &mut World) {
    // Dropping the receiver also releases a producer waiting on the bounded queue.
    world.remove_resource::<LoadingJob>();
    world.remove_resource::<Publication>();
    world.resource_mut::<OperationReport>().cancelled = true;
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
}

pub fn loading_commands(
    world: &mut World,
    mut cursor: Local<
        bevy_ecs::message::MessageCursor<cdda_components::progress::OperationCommand>,
    >,
) {
    use cdda_components::progress::OperationCommand;
    let commands: Vec<_> = cursor
        .read(world.resource::<Messages<OperationCommand>>())
        .copied()
        .collect();
    let state = world.resource::<bevy_state::state::State<AppState>>().get();
    if !matches!(state, AppState::DataLoading | AppState::WorldGen) {
        return;
    }
    for command in commands {
        match command {
            OperationCommand::Retry if world.resource::<OperationReport>().failed() => {
                if *world.resource::<bevy_state::state::State<AppState>>().get()
                    == AppState::DataLoading
                {
                    retry_loading(world);
                }
                world
                    .resource_mut::<NextState<AppState>>()
                    .set(AppState::DataLoading);
            }
            OperationCommand::ReturnToMenu => leave_loading(world),
            _ => {}
        }
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<OperationReport>();
        world.init_resource::<NextState<AppState>>();
        world
    }
    #[test]
    fn fatal_diagnostics_are_drained_across_frames_and_never_publish() {
        let mut world = world();
        let (sender, receiver) = mpsc::channel();
        for i in 0..150 {
            sender
                .send(WorkerMessage::Report(
                    ReportEvent::progress("Parsing", format!("bad file {i}"))
                        .level(ReportLevel::Error),
                ))
                .unwrap();
        }
        drop(sender);
        world.insert_resource(LoadingJob(Mutex::new(receiver)));
        for _ in 0..4 {
            poll_loading(&mut world);
        }
        assert_eq!(world.resource::<OperationReport>().errors, 150);
        assert_eq!(world.resource::<OperationReport>().history.len(), 128);
        assert!(!world.contains_resource::<Publication>());
        assert!(!world.contains_resource::<LoadingJob>());
        assert!(matches!(
            world.resource::<NextState<AppState>>(),
            NextState::Unchanged
        ));
    }
    #[test]
    fn cancellation_discards_ready_work_and_does_not_advance() {
        let mut world = world();
        world.insert_resource(Publication::Validate(DefRegistry::empty()));
        leave_loading(&mut world);
        poll_loading(&mut world);
        assert!(world.resource::<OperationReport>().cancelled);
        assert!(!world.contains_resource::<Publication>());
        assert!(matches!(
            world.resource::<NextState<AppState>>(),
            NextState::Pending(AppState::MainMenu)
        ));
    }
    #[test]
    fn disconnected_worker_becomes_a_visible_error() {
        let mut world = world();
        let (sender, receiver) = mpsc::channel();
        drop(sender);
        world.insert_resource(LoadingJob(Mutex::new(receiver)));
        poll_loading(&mut world);
        assert!(world.resource::<OperationReport>().failed());
    }
}
