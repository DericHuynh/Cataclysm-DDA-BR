//! # Definition registry (placeholder)
//!
//! The old Vec-based `DefRegistry` that stored `ItemTemplate` etc. has been
//! removed.  Definitions now live as ECS entities in the main game World,
//! spawned by `cdda_sim::def_world::build_def_world` from
//! `cdda_data::DefRegistry`.
