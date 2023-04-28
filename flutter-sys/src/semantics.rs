use std::collections::HashMap;

use crate::{ffi::to_string, sys, task_runner::UserData, EmbedderCallbacks};

pub(crate) extern "C" fn update_semantics_callback<T: EmbedderCallbacks>(
    semantics_update: *const sys::FlutterSemanticsUpdate,
    user_data: *mut ::std::os::raw::c_void,
) {
    let user_data: &mut UserData<T> = unsafe { std::mem::transmute(user_data) };

    let sys::FlutterSemanticsUpdate {
        nodes_count, nodes, ..
    } = unsafe { *semantics_update };

    let nodes = unsafe { std::slice::from_raw_parts(nodes, nodes_count) };

    let updates = nodes
        .into_iter()
        .map(
            |&sys::FlutterSemanticsNode {
                 id,
                 label,
                 transform,
                 children_in_traversal_order,
                 child_count,
                 ..
             }| {
                let children = if children_in_traversal_order == std::ptr::null() {
                    &[]
                } else {
                    unsafe { std::slice::from_raw_parts(children_in_traversal_order, child_count) }
                }
                .to_vec();

                (
                    id,
                    children,
                    FlutterSemanticsNode {
                        label: to_string(label),
                        transform,
                    },
                )
            },
        )
        .collect();

    user_data.semantics_tree.update(updates);

    user_data.semantics_tree.write_to(&mut user_data.callbacks);

    // println!("tree map: {:?}", user_data.semantics_tree);

    // let tree = FlutterSemanticsTree::from_nodes(nodes);
    // println!("root: {:?}", tree.root());
}

#[derive(Debug)]
pub struct FlutterSemanticsTree {
    id_map: HashMap<i32, FlutterSemanticsNode>,
    adjacency_list: HashMap<i32, i32>,
}

impl FlutterSemanticsTree {
    pub fn new() -> Self {
        Self {
            id_map: HashMap::new(),
            adjacency_list: HashMap::new(),
        }
    }
    pub fn update(&mut self, updates: Vec<(i32, Vec<i32>, FlutterSemanticsNode)>) {
        for (id, children, node) in updates {
            self.id_map.insert(id, node);

            for child in children {
                self.adjacency_list.insert(id, child);
            }
        }
    }

    fn write_to<T: EmbedderCallbacks>(&self, callbacks: &mut T) -> () {
        callbacks.draw_text(0, 5, "blahhhhhht");
    }

    // fn as_tree()
}

// struct FlutterSemanticsTree<'a> {
//     map: HashMap<i32, &'a sys::FlutterSemanticsNode>,
// }

// impl<'a> FlutterSemanticsTree<'a> {
//     fn from_nodes(nodes: &'a [sys::FlutterSemanticsNode]) -> Self {
//         Self {
//             map: dbg!(nodes.into_iter().map(|node| (node.id, node)).collect()),
//         }
//     }

//     fn root(&self) -> FlutterSemanticsNode {
//         let root_id = 0;

//         self.root_recur(root_id)
//     }

//     fn root_recur(&self, id: i32) -> FlutterSemanticsNode {
//         println!("blah {:?}", id);
//         let &&sys::FlutterSemanticsNode {
//             children_in_traversal_order,
//             child_count,
//             label,
//             ..
//         } = self.map.get(&id).expect("Node ID must always be present");

//         let children = if children_in_traversal_order == std::ptr::null() {
//             vec![]
//         } else {
//             let children_ids =
//                 unsafe { std::slice::from_raw_parts(children_in_traversal_order, child_count) };

//             children_ids
//                 .into_iter()
//                 .map(|id| self.root_recur(*id))
//                 .collect()
//         };

//         FlutterSemanticsNode {
//             // children,
//             label: to_string(label),
//         }
//     }
// }
#[derive(Debug)]
pub struct FlutterSemanticsNode {
    label: String,
    transform: sys::FlutterTransformation,
}
