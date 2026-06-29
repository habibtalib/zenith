//! Shared attached-effect wrapping for container subtrees.

use crate::ir::{MaskSpec, SceneCommand};

use super::super::paint::{NodeEffect, emit_node_with_effects};

pub(super) fn emit_wrapped_container(
    commands: &mut Vec<SceneCommand>,
    draws: Vec<SceneCommand>,
    effect: Option<NodeEffect>,
    mask: Option<MaskSpec>,
    connector_strokes: &mut Vec<usize>,
    local_connector_strokes: Vec<usize>,
) {
    let leading_commands = match (&mask, &effect) {
        (None, None) => 0,
        (None, Some(_)) | (Some(_), None) => 1,
        (Some(_), Some(_)) => 0,
    };
    let propagate_connectors = !matches!((&mask, &effect), (Some(_), Some(_)));
    let base = commands.len();
    emit_node_with_effects(commands, draws, effect, mask);
    if propagate_connectors {
        connector_strokes.extend(
            local_connector_strokes
                .into_iter()
                .map(|idx| base + leading_commands + idx),
        );
    }
}
