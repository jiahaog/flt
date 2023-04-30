use crate::{ffi::to_string, sys, user_data::UserData, EngineEvent, SemanticsUpdate};
use std::{collections::HashSet, fmt::Debug};

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
        .send(EngineEvent::UpdateSemantics(updates))
        .unwrap();
}

#[derive(Debug, Clone)]
pub struct FlutterSemanticsNode {
    pub label: String,
    pub flags: HashSet<FlutterSemanticsFlag>,
    pub value: String,
    pub rect: sys::FlutterRect,
    pub transform: sys::FlutterTransformation,
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

// TODO(jiahaog): Not sure if there's a better way to deal with bitflags.
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
