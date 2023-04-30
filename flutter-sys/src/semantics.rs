use crate::{ffi::to_string, sys, tasks::PlatformTask, user_data::UserData};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

pub(crate) extern "C" fn update_semantics_callback(
    semantics_update: *const sys::FlutterSemanticsUpdate,
    user_data: *mut ::std::os::raw::c_void,
) {
    let user_data: &mut UserData = unsafe { std::mem::transmute(user_data) };

    let sys::FlutterSemanticsUpdate {
        nodes_count, nodes, ..
    } = unsafe { *semantics_update };

    let nodes = unsafe { std::slice::from_raw_parts(nodes, nodes_count) };

    let updates = nodes
        .into_iter()
        .map(
            |&sys::FlutterSemanticsNode {
                 id,
                 flags,
                 label,
                 value,
                 rect,
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

                SemanticsUpdate {
                    id,
                    children,
                    node: FlutterSemanticsNode {
                        label: to_string(label),
                        flags: to_flags(flags),
                        value: to_string(value),
                        rect,
                        transform,
                    },
                }
            },
        )
        .collect();

    user_data
        .platform_task_channel
        .send(PlatformTask::UpdateSemantics(updates))
        .unwrap();
}

#[derive(Debug)]
pub struct SemanticsUpdate {
    pub id: i32,
    pub children: Vec<i32>,
    pub node: FlutterSemanticsNode,
}

pub struct FlutterSemanticsTree {
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

impl Debug for FlutterSemanticsTree {
    /// Formats the nodes in a tree like structure.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.as_graph())
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct FlutterSemanticsNode {
    pub label: String,
    pub flags: HashSet<FlutterSemanticsFlag>,
    pub value: String,
    pub rect: sys::FlutterRect,
    pub transform: sys::FlutterTransformation,
}

#[derive(Debug)]
pub struct GraphNode {
    pub current: FlutterSemanticsNode,
    pub children: Vec<GraphNode>,
}

pub use sys::FlutterTransformation;

impl sys::FlutterTransformation {
    pub fn empty() -> Self {
        Self {
            scaleX: 1.0,
            scaleY: 1.0,
            transX: 0.0,
            transY: 0.0,
            // Don't know what these mean...
            skewX: 0.0,
            skewY: 0.0,
            pers0: 0.0,
            pers1: 0.0,
            pers2: 1.0,
        }
    }
    pub fn merge_with(&self, other: &Self) -> Self {
        Self {
            scaleX: self.scaleX * other.scaleX,
            scaleY: self.scaleY * other.scaleY,
            transX: self.transX + other.transX,
            transY: self.transY + other.transY,
            skewX: self.skewX * other.skewX,
            skewY: self.skewY * other.skewY,
            pers0: self.pers0 * other.pers0,
            pers1: self.pers1 * other.pers1,
            pers2: self.pers2 * other.pers2,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FlutterSemanticsFlag {
    HasCheckedState,
    IsChecked,
    IsSelected,
    IsButton,
    IsTextField,
    IsFocused,
    HasEnabledState,
    IsEnabled,
    IsInMutuallyExclusiveGroup,
    IsHeader,
    IsObscured,
    ScopesRoute,
    NamesRoute,
    IsHidden,
    IsImage,
    IsLiveRegion,
    HasToggledState,
    IsToggled,
    HasImplicitScrolling,
    IsMultiline,
    IsReadOnly,
    IsFocusable,
    IsLink,
    IsSlider,
    IsKeyboardKey,
    IsCheckStateMixed,
}

fn to_flags(bit_flag: sys::FlutterSemanticsFlag) -> HashSet<FlutterSemanticsFlag> {
    use FlutterSemanticsFlag::*;

    let mut result = HashSet::new();
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagHasCheckedState != 0 {
        result.insert(HasCheckedState);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsChecked != 0 {
        result.insert(IsChecked);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsSelected != 0 {
        result.insert(IsSelected);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsButton != 0 {
        result.insert(IsButton);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsTextField != 0 {
        result.insert(IsTextField);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsFocused != 0 {
        result.insert(IsFocused);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagHasEnabledState != 0 {
        result.insert(HasEnabledState);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsEnabled != 0 {
        result.insert(IsEnabled);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsInMutuallyExclusiveGroup != 0 {
        result.insert(IsInMutuallyExclusiveGroup);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsHeader != 0 {
        result.insert(IsHeader);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsObscured != 0 {
        result.insert(IsObscured);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagScopesRoute != 0 {
        result.insert(ScopesRoute);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagNamesRoute != 0 {
        result.insert(NamesRoute);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsHidden != 0 {
        result.insert(IsHidden);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsImage != 0 {
        result.insert(IsImage);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsLiveRegion != 0 {
        result.insert(IsLiveRegion);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagHasToggledState != 0 {
        result.insert(HasToggledState);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsToggled != 0 {
        result.insert(IsToggled);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagHasImplicitScrolling != 0 {
        result.insert(HasImplicitScrolling);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsMultiline != 0 {
        result.insert(IsMultiline);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsReadOnly != 0 {
        result.insert(IsReadOnly);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsFocusable != 0 {
        result.insert(IsFocusable);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsLink != 0 {
        result.insert(IsLink);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsSlider != 0 {
        result.insert(IsSlider);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsKeyboardKey != 0 {
        result.insert(IsKeyboardKey);
    }
    if bit_flag & sys::FlutterSemanticsFlag_kFlutterSemanticsFlagIsCheckStateMixed != 0 {
        result.insert(IsCheckStateMixed);
    }
    result
}

const ROOT_ID: i32 = 0;
