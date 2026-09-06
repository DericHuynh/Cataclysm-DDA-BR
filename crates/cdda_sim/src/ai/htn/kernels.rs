//! The first native kernel set: Rust meanings for the stock CDDA verbs.
//!
//! Each kernel is a typed compiler: it validates its own argument schema,
//! resolves static references through the compile context (unknown item ids /
//! categories are compile errors), and emits either a
//! [`PredicateSpec`](super::kernel::PredicateSpec) (conditions the planner
//! evaluates) or an [`OperatorSpec`](super::kernel::OperatorSpec)
//! (preconditions + **predicted** effects + cost estimate + a `submit` hook
//! that binds a concrete simulation request).
//!
//! Prediction is planning-only: predicted effects feed the search's scratchpad
//! and are never written to the world by the executor — the simulation owns
//! what actually happens.

use std::sync::Arc;

use cdda_htn::graph::PrimitiveBuilder;
use cdda_htn::state::PlanState;
use cdda_components::intent::ActionIntent;
use cdda_core_types::core::coords::WorldPos;
use serde_json::Value;

use super::kernel::{
    opt_str, require_i32, CompileCtx, CompileError, KernelRegistry, OperatorSpec, PredicateSpec,
};
use super::model::{InventoryModel, NavigationModel, NearbyModel};

/// Register the stock kernels under their `cdda:` names.
pub fn register_default_kernels(kernels: &mut KernelRegistry) {
    kernels
        .predicate("cdda:hungry", compile_hungry)
        .predicate("cdda:thirsty", compile_thirsty)
        .predicate("cdda:has_items", compile_has_items)
        .predicate("cdda:adjacent", compile_adjacent)
        .operator("cdda:wait", compile_wait)
        .operator("cdda:approach", compile_approach)
        .operator("cdda:pickup", compile_pickup);
}

// ---------------------------------------------------------------------------
// Predicates
// ---------------------------------------------------------------------------

/// `cdda:hungry { "minimum": 40 }` — the actor's hunger meets the threshold.
fn compile_hungry(
    args: &Value,
    _ctx: &CompileCtx<'_>,
) -> Result<PredicateSpec, CompileError> {
    let minimum = require_i32(args, "minimum", "<kernel>", "when", 0).map_err(fixup)?;
    Ok(PredicateSpec {
        attach: Box::new(move |sink| {
            sink.add(move |n: &super::model::NeedsModel| n.hunger >= minimum);
        }),
    })
}

/// `cdda:thirsty { "minimum": 40 }`.
fn compile_thirsty(
    args: &Value,
    _ctx: &CompileCtx<'_>,
) -> Result<PredicateSpec, CompileError> {
    let minimum = require_i32(args, "minimum", "<kernel>", "when", 0).map_err(fixup)?;
    Ok(PredicateSpec {
        attach: Box::new(move |sink| {
            sink.add(move |n: &super::model::NeedsModel| n.thirst >= minimum);
        }),
    })
}

/// `cdda:has_items { "item" | "item_category", "count": 1, "scope": "carried" | "nearby" }`.
///
/// Contract rules pinned here:
/// - **One of** `item` / `item_category` must be given (never both, never
///   neither). Category membership alone does not prove suitability — the
///   consuming operation still validates at execution.
/// - `scope` selects which simulated model answers: the shared carried
///   inventory (default) or the nearby snapshot. Overlapping selectors always
///   answer from the SAME model, so counts cannot drift apart.
fn compile_has_items(
    args: &Value,
    ctx: &CompileCtx<'_>,
) -> Result<PredicateSpec, CompileError> {
    let item = opt_str(args, "item", "<kernel>", "when", 0).map_err(fixup)?;
    let category = opt_str(args, "item_category", "<kernel>", "when", 0).map_err(fixup)?;
    let count = args
        .get("count")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(1);
    let scope = opt_str(args, "scope", "<kernel>", "when", 0)
        .map_err(fixup)?
        .unwrap_or("carried");

    let (item, category) = selection_args(item, category, ctx, "has_items")?;

    match scope {
        "carried" => Ok(PredicateSpec {
            attach: Box::new(move |sink| {
                let (item, category) = (item.clone(), category.clone());
                sink.add(move |inv: &InventoryModel| {
                    let matches = |i: &super::model::ObservedItem| match (&item, &category) {
                        (Some(d), _) => &i.def == d,
                        (None, Some(c)) => i.category.as_deref() == Some(c.as_str()),
                        (None, None) => false,
                    };
                    inv.items.iter().filter(|i| matches(i)).map(|i| i.count).sum::<i32>()
                        >= count
                });
            }),
        }),
        "nearby" => Ok(PredicateSpec {
            attach: Box::new(move |sink| {
                let (item, category) = (item.clone(), category.clone());
                sink.add(move |near: &NearbyModel| {
                    near.count(item.as_deref(), category.as_deref()) >= count
                });
            }),
        }),
        other => Err(CompileError::at_def(
            "<kernel>",
            format!("`has_items` scope must be `carried` or `nearby`, got `{other}`"),
        )),
    }
}

/// `cdda:adjacent { "item" | "item_category" }` — a matching item stands
/// within Chebyshev distance 1 (same z). The gate the recursive fetch idiom
/// flips from `approach` to `pickup`.
fn compile_adjacent(
    args: &Value,
    ctx: &CompileCtx<'_>,
) -> Result<PredicateSpec, CompileError> {
    let item = opt_str(args, "item", "<kernel>", "when", 0).map_err(fixup)?;
    let category = opt_str(args, "item_category", "<kernel>", "when", 0).map_err(fixup)?;
    let (item, category) = selection_args(item, category, ctx, "adjacent")?;
    Ok(PredicateSpec {
        attach: Box::new(move |sink| {
            let (item, category) = (item.clone(), category.clone());
            sink.add2(
                move |nav: &NavigationModel, near: &NearbyModel| {
                    adjacent_match(nav, near, item.as_deref(), category.as_deref()).is_some()
                },
            );
        }),
    })
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// `cdda:wait {}` — burn the turn. Completes as soon as the simulation
/// resolves it; nothing to bind.
fn compile_wait(
    _args: &Value,
    _ctx: &CompileCtx<'_>,
) -> Result<OperatorSpec, CompileError> {
    Ok(OperatorSpec {
        define: Box::new(|p: &mut PrimitiveBuilder<'_>| {
            p.cost(1.0);
        }),
        submit: Arc::new(|_state: &PlanState| Some(ActionIntent::Wait)),
    })
}

/// `cdda:approach { "item" | "item_category" }` — one tile toward the nearest
/// matching nearby item. Predicted effect: step the navigation model one tile
/// closer; the simulation decides whether the step actually happens.
fn compile_approach(
    args: &Value,
    ctx: &CompileCtx<'_>,
) -> Result<OperatorSpec, CompileError> {
    let (item, category) = selection_args_from(args, ctx, "approach")?;
    let (item_def, cat_def) = (item.clone(), category.clone());
    let (item_sub, cat_sub) = (item, category);
    Ok(OperatorSpec {
        define: Box::new(move |p: &mut PrimitiveBuilder<'_>| {
            let (item, category) = (item_def.clone(), cat_def.clone());
            p.cost(2.0)
                .effect(move |nav: &mut NavigationModel, near: &NearbyModel| {
                    let Some(target) = near.nearest(item.as_deref(), category.as_deref()) else {
                        return;
                    };
                    let (Some(here), Some(there)) = (nav.pos, target.pos) else {
                        return;
                    };
                    let dx = (there.x - here.x).signum();
                    let dy = (there.y - here.y).signum();
                    if dx != 0 || dy != 0 {
                        nav.pos = Some(WorldPos::new(here.x + dx, here.y + dy, here.z));
                    }
                });
        }),
        submit: Arc::new(move |state: &PlanState| {
            step_toward(&item_sub, &cat_sub, state)
        }),
    })
}

/// `cdda:pickup { "item" | "item_category" }` — pick up an adjacent matching
/// ground item. Predicted effect: move the item from the nearby model into
/// the carried inventory (the simulation does the real relationship work).
fn compile_pickup(
    args: &Value,
    ctx: &CompileCtx<'_>,
) -> Result<OperatorSpec, CompileError> {
    let (item, category) = selection_args_from(args, ctx, "pickup")?;
    let (item_pre, cat_pre) = (item.clone(), category.clone());
    let (item_eff, cat_eff) = (item.clone(), category.clone());
    let (item_sub, cat_sub) = (item, category);
    Ok(OperatorSpec {
        define: Box::new(move |p: &mut PrimitiveBuilder<'_>| {
            let (item, category) = (item_pre.clone(), cat_pre.clone());
            let p = p
                .cost(1.0)
                .precondition(move |nav: &NavigationModel, near: &NearbyModel| {
                    adjacent_match(nav, near, item.as_deref(), category.as_deref()).is_some()
                });
            let (item, category) = (item_eff.clone(), cat_eff.clone());
            p.effect(
                move |inv: &mut InventoryModel, near: &mut NearbyModel, nav: &NavigationModel| {
                    if adjacent_match(nav, near, item.as_deref(), category.as_deref()).is_some() {
                        near.take_into(inv, item.as_deref(), category.as_deref());
                    }
                },
            );
        }),
        submit: Arc::new(move |state: &PlanState| {
            let (item, category) = (item_sub.clone(), cat_sub.clone());
            let Some(nav) = state.get::<NavigationModel>() else {
                return None;
            };
            let Some(near) = state.get::<NearbyModel>() else {
                return None;
            };
            let target = adjacent_match(nav, near, item.as_deref(), category.as_deref())?;
            target.entity.map(|e| ActionIntent::Pickup { item: e })
        }),
    })
}

// ---------------------------------------------------------------------------
// Shared geometry / selection helpers
// ---------------------------------------------------------------------------

/// One-tile step toward the nearest match, as a simulation request.
fn step_toward(item: &Option<String>, category: &Option<String>, state: &PlanState) -> Option<ActionIntent> {
    let nav = state.get::<NavigationModel>()?;
    let near = state.get::<NearbyModel>()?;
    let target = near.nearest(item.as_deref(), category.as_deref())?;
    let there = target.pos?;
    let here = nav.pos?;
    let dx = (there.x - here.x).signum();
    let dy = (there.y - here.y).signum();
    if dx == 0 && dy == 0 {
        return None; // already adjacent — the pickup step binds instead
    }
    Some(ActionIntent::Move { dx, dy })
}

/// The nearest matching item within Chebyshev distance 1 (same z).
fn adjacent_match<'m>(
    nav: &NavigationModel,
    near: &'m NearbyModel,
    item: Option<&str>,
    category: Option<&str>,
) -> Option<&'m super::model::ObservedItem> {
    let here = nav.pos?;
    near.items.iter().find(|i| {
        let Some(there) = i.pos else {
            return false;
        };
        there.z == here.z
            && (there.x - here.x).abs() <= 1
            && (there.y - here.y).abs() <= 1
            && match (item, category) {
                (Some(d), _) => i.def == d,
                (None, Some(c)) => i.category.as_deref() == Some(c),
                (None, None) => false,
            }
    })
}

/// Validate `{"item"?, "item_category"?}` — exactly one, and the referenced
/// definition must exist at compile time.
fn selection_args(
    item: Option<&str>,
    category: Option<&str>,
    ctx: &CompileCtx<'_>,
    kernel: &str,
) -> Result<(Option<String>, Option<String>), CompileError> {
    match (item, category) {
        (Some(_), Some(_)) => Err(CompileError::at_def(
            "<kernel>",
            format!("`{kernel}` takes `item` OR `item_category`, not both"),
        )),
        (None, None) => Err(CompileError::at_def(
            "<kernel>",
            format!("`{kernel}` needs `item` or `item_category`"),
        )),
        (Some(id), None) => {
            ctx.require_item(id, "<kernel>", kernel, 0)?;
            Ok((Some(id.to_string()), None))
        }
        (None, Some(cat)) => {
            ctx.require_category(cat, "<kernel>", kernel, 0)?;
            Ok((None, Some(cat.to_string())))
        }
    }
}

fn selection_args_from(
    args: &Value,
    ctx: &CompileCtx<'_>,
    kernel: &str,
) -> Result<(Option<String>, Option<String>), CompileError> {
    let item = opt_str(args, "item", "<kernel>", kernel, 0).map_err(fixup)?;
    let category = opt_str(args, "item_category", "<kernel>", kernel, 0).map_err(fixup)?;
    selection_args(item, category, ctx, kernel)
}

/// Schema errors carry a placeholder def; the compiler pins the real
/// def/method/step location at the referencing site.
fn fixup(e: CompileError) -> CompileError {
    CompileError {
        def: "<kernel>".into(),
        method: None,
        step: None,
        ..e
    }
}
