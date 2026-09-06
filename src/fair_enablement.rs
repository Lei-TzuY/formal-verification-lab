use crate::fairness::WeakFairness;
use crate::graph::{ReachableGraph, SnapshotEdge};
use std::collections::HashMap;
use std::hash::Hash;

/// Project complete model-side action enablement into an already materialized
/// product state id space.
///
/// Targets are self references because consumers need only action-label
/// presence for weak-fairness enablement checks. Executable recurrent edges
/// always come from the retained product residual itself.
pub(crate) fn complete_enablement_graph<S, P, Project>(
    model_graph: &ReachableGraph<S>,
    product: &ReachableGraph<P>,
    project_model_state: Project,
) -> Option<ReachableGraph<P>>
where
    S: Clone + Eq + Hash,
    P: Clone,
    Project: Fn(&P) -> &S,
{
    let model_ids = model_graph
        .states
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, state)| (state, id))
        .collect::<HashMap<_, _>>();

    let outgoing = product
        .states
        .iter()
        .enumerate()
        .map(|(product_id, state)| {
            let model_id = model_ids.get(project_model_state(state)).copied()?;
            Some(
                model_graph.outgoing[model_id]
                    .iter()
                    .map(|edge| SnapshotEdge {
                        action: edge.action.clone(),
                        target: product_id,
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Option<Vec<_>>>()?;

    Some(ReachableGraph {
        states: product.states.clone(),
        outgoing,
        initial_ids: product.initial_ids.clone(),
    })
}

/// Project conservative bounded-model enablement knowledge into product state
/// ids without turning an unknown successor vector into a disabled-action fact.
///
/// `complete_enabled_actions[id] = Some(actions)` is exact knowledge from one
/// evaluated successor vector. `None` means enablement is unknown, so every
/// configured fair action is conservatively represented as enabled. This is the
/// M33 provenance rule shared by every staged weak-fair temporal consumer.
pub(crate) fn bounded_enablement_graph<S, P, Project>(
    model_graph: &ReachableGraph<S>,
    complete_enabled_actions: &[Option<Vec<String>>],
    product: &ReachableGraph<P>,
    fairness: &WeakFairness,
    project_model_state: Project,
) -> Option<ReachableGraph<P>>
where
    S: Clone + Eq + Hash,
    P: Clone,
    Project: Fn(&P) -> &S,
{
    if complete_enabled_actions.len() != model_graph.states.len() {
        return None;
    }

    let model_ids = model_graph
        .states
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, state)| (state, id))
        .collect::<HashMap<_, _>>();

    let outgoing = product
        .states
        .iter()
        .enumerate()
        .map(|(product_id, state)| {
            let model_id = model_ids.get(project_model_state(state)).copied()?;
            let actions = complete_enabled_actions[model_id]
                .as_ref()
                .cloned()
                .unwrap_or_else(|| fairness.actions().to_vec());
            Some(
                actions
                    .into_iter()
                    .map(|action| SnapshotEdge {
                        action,
                        target: product_id,
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Option<Vec<_>>>()?;

    Some(ReachableGraph {
        states: product.states.clone(),
        outgoing,
        initial_ids: product.initial_ids.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ProductState {
        model: usize,
    }

    fn model_graph() -> ReachableGraph<usize> {
        ReachableGraph {
            states: vec![0, 1],
            outgoing: vec![
                vec![SnapshotEdge {
                    action: "fair".to_owned(),
                    target: 1,
                }],
                Vec::new(),
            ],
            initial_ids: vec![0],
        }
    }

    fn product_graph() -> ReachableGraph<ProductState> {
        ReachableGraph {
            states: vec![ProductState { model: 0 }, ProductState { model: 1 }],
            outgoing: vec![Vec::new(), Vec::new()],
            initial_ids: vec![0],
        }
    }

    #[test]
    fn complete_projection_uses_model_enablement_not_retained_product_edges() {
        let projected = complete_enablement_graph(&model_graph(), &product_graph(), |state| {
            &state.model
        })
        .unwrap();

        assert_eq!(projected.outgoing[0].len(), 1);
        assert_eq!(projected.outgoing[0][0].action, "fair");
        assert!(projected.outgoing[1].is_empty());
    }

    #[test]
    fn unknown_bounded_enablement_conservatively_keeps_every_fair_action_enabled() {
        let fairness = WeakFairness::new(["fair", "other"]).unwrap();
        let projected = bounded_enablement_graph(
            &model_graph(),
            &[None, Some(Vec::new())],
            &product_graph(),
            &fairness,
            |state| &state.model,
        )
        .unwrap();

        assert_eq!(
            projected.outgoing[0]
                .iter()
                .map(|edge| edge.action.as_str())
                .collect::<Vec<_>>(),
            vec!["fair", "other"]
        );
        assert!(projected.outgoing[1].is_empty());
    }
}
