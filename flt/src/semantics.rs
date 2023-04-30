use flutter_sys::{FlutterSemanticsNode, FlutterTransformation, SemanticsUpdate};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::terminal_window::TerminalWindow;

#[derive(Debug)]
pub(crate) struct GraphNode {
    pub current: FlutterSemanticsNode,
    pub children: Vec<GraphNode>,
}

pub(crate) struct FlutterSemanticsTree {
    id_map: HashMap<i32, FlutterSemanticsNode>,
    adjacency_list: HashMap<i32, Vec<i32>>,
}

impl FlutterSemanticsTree {
    pub fn new() -> Self {
        Self {
            id_map: HashMap::new(),
            adjacency_list: HashMap::new(),
        }
    }
    pub fn update(&mut self, updates: Vec<SemanticsUpdate>) {
        for SemanticsUpdate { id, children, node } in updates {
            self.id_map.insert(id, node);

            self.adjacency_list.insert(id, children);
        }
    }

    pub fn as_graph(&self) -> GraphNode {
        self.as_graph_recur(ROOT_ID)
    }

    fn as_graph_recur(&self, id: i32) -> GraphNode {
        let current = Clone::clone(self.id_map.get(&id).unwrap());

        let children = self
            .adjacency_list
            .get(&id)
            .unwrap()
            .into_iter()
            .map(|child_id| self.as_graph_recur(*child_id))
            .collect();

        GraphNode { current, children }
    }
}

const ROOT_ID: i32 = 0;

pub(crate) fn draw_semantic_labels(
    terminal_window: &mut TerminalWindow,
    parent_merged_transform: FlutterTransformation,
    node: GraphNode,
) -> Result<(), crossterm::ErrorKind> {
    let current = node.current;

    let transform = current.transform.merge_with(&parent_merged_transform);

    if !current
        .flags
        .contains(&flutter_sys::FlutterSemanticsFlag::IsHidden)
        && !current.label.is_empty()
    {
        terminal_window.draw_text(
            (transform.transX * transform.scaleX).round() as usize,
            (transform.transY * transform.scaleY).round() as usize,
            &current.label,
        )?;
    }

    for child in node.children {
        draw_semantic_labels(terminal_window, transform, child)?;
    }

    Ok(())
}
