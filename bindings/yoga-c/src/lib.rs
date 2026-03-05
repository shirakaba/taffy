#![allow(clippy::missing_safety_doc)]

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::OnceLock;

use taffy::geometry::{Rect, Size};
use taffy::prelude::{
    AlignContent, AlignItems, AvailableSpace, Dimension, Display, FlexDirection, FlexWrap, JustifyContent,
    LengthPercentage, LengthPercentageAuto, Position, Style,
};
use taffy::style::{
    BoxSizing, CompactLength, GridPlacement, GridTemplateComponent, MaxTrackSizingFunction, MinTrackSizingFunction,
    TrackSizingFunction,
};
use taffy::style_helpers::{minmax, TaffyGridLine, TaffyGridSpan};
use taffy::tree::Layout;
use taffy::{Overflow, TaffyTree};

const YG_ALIGN_AUTO: i32 = 0;
const YG_ALIGN_FLEX_START: i32 = 1;
const YG_ALIGN_CENTER: i32 = 2;
const YG_ALIGN_FLEX_END: i32 = 3;
const YG_ALIGN_STRETCH: i32 = 4;
const YG_ALIGN_BASELINE: i32 = 5;
const YG_ALIGN_SPACE_BETWEEN: i32 = 6;
const YG_ALIGN_SPACE_AROUND: i32 = 7;
const YG_ALIGN_SPACE_EVENLY: i32 = 8;
const YG_ALIGN_START: i32 = 9;
const YG_ALIGN_END: i32 = 10;

const YG_BOX_SIZING_BORDER_BOX: i32 = 0;
const YG_BOX_SIZING_CONTENT_BOX: i32 = 1;

const YG_DIRECTION_INHERIT: i32 = 0;
const YG_DIRECTION_LTR: i32 = 1;
const YG_DIRECTION_RTL: i32 = 2;

const YG_DISPLAY_FLEX: i32 = 0;
const YG_DISPLAY_NONE: i32 = 1;
const YG_DISPLAY_CONTENTS: i32 = 2;
const YG_DISPLAY_GRID: i32 = 3;

const YG_EDGE_LEFT: i32 = 0;
const YG_EDGE_TOP: i32 = 1;
const YG_EDGE_RIGHT: i32 = 2;
const YG_EDGE_BOTTOM: i32 = 3;
const YG_EDGE_START: i32 = 4;
const YG_EDGE_END: i32 = 5;
const YG_EDGE_HORIZONTAL: i32 = 6;
const YG_EDGE_VERTICAL: i32 = 7;
const YG_EDGE_ALL: i32 = 8;

const YG_FLEX_DIRECTION_COLUMN: i32 = 0;
const YG_FLEX_DIRECTION_COLUMN_REVERSE: i32 = 1;
const YG_FLEX_DIRECTION_ROW: i32 = 2;
const YG_FLEX_DIRECTION_ROW_REVERSE: i32 = 3;

const YG_GUTTER_COLUMN: i32 = 0;
const YG_GUTTER_ROW: i32 = 1;
const YG_GUTTER_ALL: i32 = 2;

const YG_JUSTIFY_AUTO: i32 = 0;
const YG_JUSTIFY_FLEX_START: i32 = 1;
const YG_JUSTIFY_CENTER: i32 = 2;
const YG_JUSTIFY_FLEX_END: i32 = 3;
const YG_JUSTIFY_SPACE_BETWEEN: i32 = 4;
const YG_JUSTIFY_SPACE_AROUND: i32 = 5;
const YG_JUSTIFY_SPACE_EVENLY: i32 = 6;
const YG_JUSTIFY_STRETCH: i32 = 7;
const YG_JUSTIFY_START: i32 = 8;
const YG_JUSTIFY_END: i32 = 9;

const YG_MEASURE_MODE_UNDEFINED: i32 = 0;
const YG_MEASURE_MODE_EXACTLY: i32 = 1;
const YG_MEASURE_MODE_AT_MOST: i32 = 2;

const YG_NODE_TYPE_DEFAULT: i32 = 0;
const YG_NODE_TYPE_TEXT: i32 = 1;

const YG_OVERFLOW_VISIBLE: i32 = 0;
const YG_OVERFLOW_HIDDEN: i32 = 1;
const YG_OVERFLOW_SCROLL: i32 = 2;

const YG_POSITION_TYPE_STATIC: i32 = 0;
const YG_POSITION_TYPE_RELATIVE: i32 = 1;
const YG_POSITION_TYPE_ABSOLUTE: i32 = 2;

const YG_UNIT_UNDEFINED: i32 = 0;
const YG_UNIT_POINT: i32 = 1;
const YG_UNIT_PERCENT: i32 = 2;
const YG_UNIT_AUTO: i32 = 3;
const YG_UNIT_MAX_CONTENT: i32 = 4;
const YG_UNIT_FIT_CONTENT: i32 = 5;
const YG_UNIT_STRETCH: i32 = 6;

const YG_WRAP_NO_WRAP: i32 = 0;
const YG_WRAP_WRAP: i32 = 1;
const YG_WRAP_WRAP_REVERSE: i32 = 2;

const YG_ERRATA_STRETCH_FLEX_BASIS: i32 = 1;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct YGSize {
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct YGValue {
    pub value: f32,
    pub unit: i32,
}

#[no_mangle]
pub static YGValueAuto: YGValue = YGValue {
    value: f32::NAN,
    unit: YG_UNIT_AUTO,
};

#[no_mangle]
pub static YGValueUndefined: YGValue = YGValue {
    value: f32::NAN,
    unit: YG_UNIT_UNDEFINED,
};

#[no_mangle]
pub static YGValueZero: YGValue = YGValue {
    value: 0.0,
    unit: YG_UNIT_POINT,
};

type YGMeasureFunc =
    unsafe extern "C" fn(*const YGNode, f32, i32, f32, i32) -> YGSize;
type YGBaselineFunc = unsafe extern "C" fn(*const YGNode, f32, f32) -> f32;
type YGDirtiedFunc = unsafe extern "C" fn(*const YGNode);
type YGLogger = unsafe extern "C" fn(*const YGConfig, *const YGNode, i32, *const i8, *mut c_void) -> i32;
type YGCloneNodeFunc = unsafe extern "C" fn(*const YGNode, *const YGNode, usize) -> *mut YGNode;

#[derive(Clone)]
struct YGLayout {
    final_layout: Layout,
    unrounded_layout: Layout,
    direction: i32,
    had_overflow: bool,
}

impl Default for YGLayout {
    fn default() -> Self {
        Self {
            final_layout: Layout::new(),
            unrounded_layout: Layout::new(),
            direction: YG_DIRECTION_LTR,
            had_overflow: false,
        }
    }
}

struct YGNode {
    style: Style,
    display_mode: i32,
    position_type_mode: i32,
    owner: *mut YGNode,
    children: Vec<*mut YGNode>,
    config: *mut YGConfig,
    context: *mut c_void,
    has_new_layout: bool,
    is_dirty: bool,
    dirtied_func: Option<YGDirtiedFunc>,
    measure_func: Option<YGMeasureFunc>,
    baseline_func: Option<YGBaselineFunc>,
    is_reference_baseline: bool,
    node_type: i32,
    always_forms_containing_block: bool,
    direction: i32,
    flex: Option<f32>,
    has_flex_grow: bool,
    has_flex_shrink: bool,
    position_edges: YogaEdgeValuesLpa,
    margin_edges: YogaEdgeValuesLpa,
    padding_edges: YogaEdgeValuesLp,
    border_edges: YogaEdgeValuesLp,
    layout: YGLayout,
}

struct YGConfig {
    use_web_defaults: bool,
    point_scale_factor: f32,
    errata: i32,
    logger: Option<YGLogger>,
    context: *mut c_void,
    clone_node_func: Option<YGCloneNodeFunc>,
    experimental_web_flex_basis: bool,
}

struct YGGridTrackList {
    tracks: Vec<TrackSizingFunction>,
}

enum YGGridTrackValue {
    Single(TrackSizingFunction),
    MinMax(MinTrackSizingFunction, MaxTrackSizingFunction),
}

const YOGA_EDGE_COUNT: usize = 9;

#[derive(Clone)]
struct YogaEdgeValuesLpa {
    values: [Option<LengthPercentageAuto>; YOGA_EDGE_COUNT],
}

impl Default for YogaEdgeValuesLpa {
    fn default() -> Self {
        Self {
            values: [None; YOGA_EDGE_COUNT],
        }
    }
}

impl YogaEdgeValuesLpa {
    fn set(&mut self, edge: i32, value: LengthPercentageAuto) {
        if (0..YOGA_EDGE_COUNT as i32).contains(&edge) {
            self.values[edge as usize] = Some(value);
        }
    }

    fn get_exact(&self, edge: i32) -> Option<LengthPercentageAuto> {
        if (0..YOGA_EDGE_COUNT as i32).contains(&edge) {
            self.values[edge as usize]
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct YogaEdgeValuesLp {
    values: [Option<LengthPercentage>; YOGA_EDGE_COUNT],
}

impl Default for YogaEdgeValuesLp {
    fn default() -> Self {
        Self {
            values: [None; YOGA_EDGE_COUNT],
        }
    }
}

impl YogaEdgeValuesLp {
    fn set(&mut self, edge: i32, value: LengthPercentage) {
        if (0..YOGA_EDGE_COUNT as i32).contains(&edge) {
            self.values[edge as usize] = Some(value);
        }
    }

    fn get_exact(&self, edge: i32) -> Option<LengthPercentage> {
        if (0..YOGA_EDGE_COUNT as i32).contains(&edge) {
            self.values[edge as usize]
        } else {
            None
        }
    }
}

fn config_default() -> *mut YGConfig {
    static DEFAULT: OnceLock<usize> = OnceLock::new();
    let ptr = *DEFAULT.get_or_init(|| {
        let boxed = Box::new(YGConfig {
            use_web_defaults: false,
            point_scale_factor: 1.0,
            errata: 0,
            logger: None,
            context: ptr::null_mut(),
            clone_node_func: None,
            experimental_web_flex_basis: false,
        });
        Box::into_raw(boxed) as usize
    });
    ptr as *mut YGConfig
}

fn default_style(use_web_defaults: bool) -> Style {
    let mut style = Style::DEFAULT;
    if !use_web_defaults {
        style.flex_direction = FlexDirection::Column;
        style.align_content = Some(AlignContent::FlexStart);
        style.flex_shrink = 0.0;
    }
    style
}

fn to_available_space(v: f32) -> AvailableSpace {
    if v.is_nan() {
        AvailableSpace::MaxContent
    } else {
        AvailableSpace::Definite(v)
    }
}

fn round_value_to_pixel_grid(value: f64, point_scale_factor: f64, force_ceil: bool, force_floor: bool) -> f32 {
    if point_scale_factor == 0.0 || point_scale_factor.is_nan() {
        return value as f32;
    }
    let mut scaled_value = value * point_scale_factor;
    let mut fractional = scaled_value % 1.0;
    if fractional < 0.0 {
        fractional += 1.0;
    }
    if fractional.abs() <= 0.0001 {
        scaled_value -= fractional;
    } else if (fractional - 1.0).abs() <= 0.0001 {
        scaled_value = scaled_value - fractional + 1.0;
    } else if force_ceil {
        scaled_value = scaled_value - fractional + 1.0;
    } else if force_floor {
        scaled_value -= fractional;
    } else {
        scaled_value = scaled_value - fractional + if fractional >= 0.5 { 1.0 } else { 0.0 };
    }
    (scaled_value / point_scale_factor) as f32
}

fn mark_dirty(node: *mut YGNode) {
    unsafe {
        let mut cursor = node;
        while !cursor.is_null() {
            if !(*cursor).is_dirty {
                (*cursor).is_dirty = true;
                if let Some(dirtied) = (*cursor).dirtied_func {
                    dirtied(cursor);
                }
            }
            cursor = (*cursor).owner;
        }
    }
}

fn map_align(align: i32) -> Option<AlignItems> {
    match align {
        YG_ALIGN_AUTO => None,
        YG_ALIGN_FLEX_START => Some(AlignItems::FlexStart),
        YG_ALIGN_CENTER => Some(AlignItems::Center),
        YG_ALIGN_FLEX_END => Some(AlignItems::FlexEnd),
        YG_ALIGN_STRETCH => Some(AlignItems::Stretch),
        YG_ALIGN_BASELINE => Some(AlignItems::Baseline),
        YG_ALIGN_START => Some(AlignItems::Start),
        YG_ALIGN_END => Some(AlignItems::End),
        _ => None,
    }
}

fn unmap_align(value: Option<AlignItems>, auto: bool) -> i32 {
    match value {
        None => {
            if auto {
                YG_ALIGN_AUTO
            } else {
                YG_ALIGN_STRETCH
            }
        }
        Some(AlignItems::FlexStart) => YG_ALIGN_FLEX_START,
        Some(AlignItems::Center) => YG_ALIGN_CENTER,
        Some(AlignItems::FlexEnd) => YG_ALIGN_FLEX_END,
        Some(AlignItems::Stretch) => YG_ALIGN_STRETCH,
        Some(AlignItems::Baseline) => YG_ALIGN_BASELINE,
        Some(AlignItems::Start) => YG_ALIGN_START,
        Some(AlignItems::End) => YG_ALIGN_END,
    }
}

fn map_align_content(align: i32) -> Option<AlignContent> {
    match align {
        YG_ALIGN_AUTO => None,
        YG_ALIGN_FLEX_START => Some(AlignContent::FlexStart),
        YG_ALIGN_CENTER => Some(AlignContent::Center),
        YG_ALIGN_FLEX_END => Some(AlignContent::FlexEnd),
        YG_ALIGN_STRETCH => Some(AlignContent::Stretch),
        YG_ALIGN_SPACE_BETWEEN => Some(AlignContent::SpaceBetween),
        YG_ALIGN_SPACE_AROUND => Some(AlignContent::SpaceAround),
        YG_ALIGN_SPACE_EVENLY => Some(AlignContent::SpaceEvenly),
        YG_ALIGN_START => Some(AlignContent::Start),
        YG_ALIGN_END => Some(AlignContent::End),
        _ => None,
    }
}

fn unmap_align_content(value: Option<AlignContent>, auto: bool) -> i32 {
    match value {
        None => {
            if auto {
                YG_ALIGN_AUTO
            } else {
                YG_ALIGN_FLEX_START
            }
        }
        Some(AlignContent::FlexStart) => YG_ALIGN_FLEX_START,
        Some(AlignContent::Center) => YG_ALIGN_CENTER,
        Some(AlignContent::FlexEnd) => YG_ALIGN_FLEX_END,
        Some(AlignContent::Stretch) => YG_ALIGN_STRETCH,
        Some(AlignContent::SpaceBetween) => YG_ALIGN_SPACE_BETWEEN,
        Some(AlignContent::SpaceAround) => YG_ALIGN_SPACE_AROUND,
        Some(AlignContent::SpaceEvenly) => YG_ALIGN_SPACE_EVENLY,
        Some(AlignContent::Start) => YG_ALIGN_START,
        Some(AlignContent::End) => YG_ALIGN_END,
    }
}

fn flip_align_items_for_rtl(value: Option<AlignItems>) -> Option<AlignItems> {
    value.map(|v| match v {
        AlignItems::Start => AlignItems::End,
        AlignItems::End => AlignItems::Start,
        AlignItems::FlexStart => AlignItems::FlexEnd,
        AlignItems::FlexEnd => AlignItems::FlexStart,
        _ => v,
    })
}

fn flip_align_content_for_rtl(value: Option<AlignContent>) -> Option<AlignContent> {
    value.map(|v| match v {
        AlignContent::Start => AlignContent::End,
        AlignContent::End => AlignContent::Start,
        AlignContent::FlexStart => AlignContent::FlexEnd,
        AlignContent::FlexEnd => AlignContent::FlexStart,
        _ => v,
    })
}

fn map_justify(justify: i32) -> Option<JustifyContent> {
    match justify {
        YG_JUSTIFY_AUTO => None,
        YG_JUSTIFY_FLEX_START => Some(JustifyContent::FlexStart),
        YG_JUSTIFY_CENTER => Some(JustifyContent::Center),
        YG_JUSTIFY_FLEX_END => Some(JustifyContent::FlexEnd),
        YG_JUSTIFY_SPACE_BETWEEN => Some(JustifyContent::SpaceBetween),
        YG_JUSTIFY_SPACE_AROUND => Some(JustifyContent::SpaceAround),
        YG_JUSTIFY_SPACE_EVENLY => Some(JustifyContent::SpaceEvenly),
        YG_JUSTIFY_STRETCH => Some(JustifyContent::Stretch),
        YG_JUSTIFY_START => Some(JustifyContent::Start),
        YG_JUSTIFY_END => Some(JustifyContent::End),
        _ => None,
    }
}

fn unmap_justify(value: Option<JustifyContent>, auto: bool) -> i32 {
    match value {
        None => {
            if auto {
                YG_JUSTIFY_AUTO
            } else {
                YG_JUSTIFY_FLEX_START
            }
        }
        Some(JustifyContent::FlexStart) => YG_JUSTIFY_FLEX_START,
        Some(JustifyContent::Center) => YG_JUSTIFY_CENTER,
        Some(JustifyContent::FlexEnd) => YG_JUSTIFY_FLEX_END,
        Some(JustifyContent::SpaceBetween) => YG_JUSTIFY_SPACE_BETWEEN,
        Some(JustifyContent::SpaceAround) => YG_JUSTIFY_SPACE_AROUND,
        Some(JustifyContent::SpaceEvenly) => YG_JUSTIFY_SPACE_EVENLY,
        Some(JustifyContent::Stretch) => YG_JUSTIFY_STRETCH,
        Some(JustifyContent::Start) => YG_JUSTIFY_START,
        Some(JustifyContent::End) => YG_JUSTIFY_END,
    }
}

fn map_flex_direction(value: i32) -> FlexDirection {
    match value {
        YG_FLEX_DIRECTION_COLUMN => FlexDirection::Column,
        YG_FLEX_DIRECTION_COLUMN_REVERSE => FlexDirection::ColumnReverse,
        YG_FLEX_DIRECTION_ROW_REVERSE => FlexDirection::RowReverse,
        _ => FlexDirection::Row,
    }
}

fn unmap_flex_direction(value: FlexDirection) -> i32 {
    match value {
        FlexDirection::Column => YG_FLEX_DIRECTION_COLUMN,
        FlexDirection::ColumnReverse => YG_FLEX_DIRECTION_COLUMN_REVERSE,
        FlexDirection::Row => YG_FLEX_DIRECTION_ROW,
        FlexDirection::RowReverse => YG_FLEX_DIRECTION_ROW_REVERSE,
    }
}

fn map_wrap(value: i32) -> FlexWrap {
    match value {
        YG_WRAP_WRAP => FlexWrap::Wrap,
        YG_WRAP_WRAP_REVERSE => FlexWrap::WrapReverse,
        _ => FlexWrap::NoWrap,
    }
}

fn unmap_wrap(value: FlexWrap) -> i32 {
    match value {
        FlexWrap::NoWrap => YG_WRAP_NO_WRAP,
        FlexWrap::Wrap => YG_WRAP_WRAP,
        FlexWrap::WrapReverse => YG_WRAP_WRAP_REVERSE,
    }
}

fn map_position_type(value: i32) -> Position {
    match value {
        YG_POSITION_TYPE_ABSOLUTE => Position::Absolute,
        _ => Position::Relative,
    }
}

fn unmap_position_type(value: Position) -> i32 {
    match value {
        Position::Absolute => YG_POSITION_TYPE_ABSOLUTE,
        Position::Relative => YG_POSITION_TYPE_RELATIVE,
    }
}

fn map_display(value: i32) -> Display {
    match value {
        YG_DISPLAY_NONE => Display::None,
        YG_DISPLAY_GRID => Display::Grid,
        _ => Display::Flex,
    }
}

fn unmap_display(value: Display) -> i32 {
    match value {
        Display::None => YG_DISPLAY_NONE,
        Display::Grid => YG_DISPLAY_GRID,
        Display::Flex => YG_DISPLAY_FLEX,
        #[allow(unreachable_patterns)]
        _ => YG_DISPLAY_FLEX,
    }
}

fn map_overflow(value: i32) -> Overflow {
    match value {
        YG_OVERFLOW_HIDDEN => Overflow::Hidden,
        YG_OVERFLOW_SCROLL => Overflow::Scroll,
        _ => Overflow::Visible,
    }
}

fn unmap_overflow(value: Overflow) -> i32 {
    match value {
        Overflow::Hidden => YG_OVERFLOW_HIDDEN,
        Overflow::Scroll => YG_OVERFLOW_SCROLL,
        Overflow::Visible | Overflow::Clip => YG_OVERFLOW_VISIBLE,
    }
}

fn map_box_sizing(value: i32) -> BoxSizing {
    if value == YG_BOX_SIZING_CONTENT_BOX {
        BoxSizing::ContentBox
    } else {
        BoxSizing::BorderBox
    }
}

fn unmap_box_sizing(value: BoxSizing) -> i32 {
    match value {
        BoxSizing::BorderBox => YG_BOX_SIZING_BORDER_BOX,
        BoxSizing::ContentBox => YG_BOX_SIZING_CONTENT_BOX,
    }
}

fn edge_to_rect_indexes(edge: i32, direction: i32) -> [Option<usize>; 4] {
    // left, top, right, bottom
    match edge {
        YG_EDGE_LEFT => [Some(0), None, None, None],
        YG_EDGE_TOP => [None, Some(1), None, None],
        YG_EDGE_RIGHT => [None, None, Some(2), None],
        YG_EDGE_BOTTOM => [None, None, None, Some(3)],
        YG_EDGE_START => {
            if direction == YG_DIRECTION_RTL {
                [None, None, Some(2), None]
            } else {
                [Some(0), None, None, None]
            }
        }
        YG_EDGE_END => {
            if direction == YG_DIRECTION_RTL {
                [Some(0), None, None, None]
            } else {
                [None, None, Some(2), None]
            }
        }
        YG_EDGE_HORIZONTAL => [Some(0), None, Some(2), None],
        YG_EDGE_VERTICAL => [None, Some(1), None, Some(3)],
        YG_EDGE_ALL => [Some(0), Some(1), Some(2), Some(3)],
        _ => [None, None, None, None],
    }
}

fn set_lpa(rect: &mut Rect<LengthPercentageAuto>, edge: i32, direction: i32, value: LengthPercentageAuto) {
    let idx = edge_to_rect_indexes(edge, direction);
    if idx[0].is_some() {
        rect.left = value;
    }
    if idx[1].is_some() {
        rect.top = value;
    }
    if idx[2].is_some() {
        rect.right = value;
    }
    if idx[3].is_some() {
        rect.bottom = value;
    }
}

fn set_lp(rect: &mut Rect<LengthPercentage>, edge: i32, direction: i32, value: LengthPercentage) {
    let idx = edge_to_rect_indexes(edge, direction);
    if idx[0].is_some() {
        rect.left = value;
    }
    if idx[1].is_some() {
        rect.top = value;
    }
    if idx[2].is_some() {
        rect.right = value;
    }
    if idx[3].is_some() {
        rect.bottom = value;
    }
}

fn set_dim(size: &mut Size<Dimension>, edge: i32, value: Dimension) {
    match edge {
        YG_EDGE_LEFT | YG_EDGE_RIGHT | YG_EDGE_START | YG_EDGE_END | YG_EDGE_HORIZONTAL => size.width = value,
        YG_EDGE_TOP | YG_EDGE_BOTTOM | YG_EDGE_VERTICAL => size.height = value,
        YG_EDGE_ALL => {
            size.width = value;
            size.height = value;
        }
        _ => {}
    }
}

fn get_lpa(rect: &Rect<LengthPercentageAuto>, edge: i32, direction: i32) -> LengthPercentageAuto {
    match edge {
        YG_EDGE_LEFT => rect.left,
        YG_EDGE_TOP => rect.top,
        YG_EDGE_RIGHT => rect.right,
        YG_EDGE_BOTTOM => rect.bottom,
        YG_EDGE_START => {
            if direction == YG_DIRECTION_RTL {
                rect.right
            } else {
                rect.left
            }
        }
        YG_EDGE_END => {
            if direction == YG_DIRECTION_RTL {
                rect.left
            } else {
                rect.right
            }
        }
        _ => LengthPercentageAuto::auto(),
    }
}

fn get_lp(rect: &Rect<LengthPercentage>, edge: i32, direction: i32) -> LengthPercentage {
    match edge {
        YG_EDGE_LEFT => rect.left,
        YG_EDGE_TOP => rect.top,
        YG_EDGE_RIGHT => rect.right,
        YG_EDGE_BOTTOM => rect.bottom,
        YG_EDGE_START => {
            if direction == YG_DIRECTION_RTL {
                rect.right
            } else {
                rect.left
            }
        }
        YG_EDGE_END => {
            if direction == YG_DIRECTION_RTL {
                rect.left
            } else {
                rect.right
            }
        }
        _ => LengthPercentage::length(f32::NAN),
    }
}

fn choose_lpa(
    edges: &YogaEdgeValuesLpa,
    physical_edge: i32,
    direction: i32,
    fallback: LengthPercentageAuto,
) -> LengthPercentageAuto {
    match physical_edge {
        YG_EDGE_LEFT => {
            if direction == YG_DIRECTION_LTR {
                if let Some(v) = edges.get_exact(YG_EDGE_START) {
                    return v;
                }
            } else if direction == YG_DIRECTION_RTL {
                if let Some(v) = edges.get_exact(YG_EDGE_END) {
                    return v;
                }
            }
            edges
                .get_exact(YG_EDGE_LEFT)
                .or_else(|| edges.get_exact(YG_EDGE_HORIZONTAL))
                .or_else(|| edges.get_exact(YG_EDGE_ALL))
                .unwrap_or(fallback)
        }
        YG_EDGE_RIGHT => {
            if direction == YG_DIRECTION_LTR {
                if let Some(v) = edges.get_exact(YG_EDGE_END) {
                    return v;
                }
            } else if direction == YG_DIRECTION_RTL {
                if let Some(v) = edges.get_exact(YG_EDGE_START) {
                    return v;
                }
            }
            edges
                .get_exact(YG_EDGE_RIGHT)
                .or_else(|| edges.get_exact(YG_EDGE_HORIZONTAL))
                .or_else(|| edges.get_exact(YG_EDGE_ALL))
                .unwrap_or(fallback)
        }
        YG_EDGE_TOP => edges
            .get_exact(YG_EDGE_TOP)
            .or_else(|| edges.get_exact(YG_EDGE_VERTICAL))
            .or_else(|| edges.get_exact(YG_EDGE_ALL))
            .unwrap_or(fallback),
        YG_EDGE_BOTTOM => edges
            .get_exact(YG_EDGE_BOTTOM)
            .or_else(|| edges.get_exact(YG_EDGE_VERTICAL))
            .or_else(|| edges.get_exact(YG_EDGE_ALL))
            .unwrap_or(fallback),
        _ => fallback,
    }
}

fn choose_lp(
    edges: &YogaEdgeValuesLp,
    physical_edge: i32,
    direction: i32,
    fallback: LengthPercentage,
) -> LengthPercentage {
    match physical_edge {
        YG_EDGE_LEFT => {
            if direction == YG_DIRECTION_LTR {
                if let Some(v) = edges.get_exact(YG_EDGE_START) {
                    return v;
                }
            } else if direction == YG_DIRECTION_RTL {
                if let Some(v) = edges.get_exact(YG_EDGE_END) {
                    return v;
                }
            }
            edges
                .get_exact(YG_EDGE_LEFT)
                .or_else(|| edges.get_exact(YG_EDGE_HORIZONTAL))
                .or_else(|| edges.get_exact(YG_EDGE_ALL))
                .unwrap_or(fallback)
        }
        YG_EDGE_RIGHT => {
            if direction == YG_DIRECTION_LTR {
                if let Some(v) = edges.get_exact(YG_EDGE_END) {
                    return v;
                }
            } else if direction == YG_DIRECTION_RTL {
                if let Some(v) = edges.get_exact(YG_EDGE_START) {
                    return v;
                }
            }
            edges
                .get_exact(YG_EDGE_RIGHT)
                .or_else(|| edges.get_exact(YG_EDGE_HORIZONTAL))
                .or_else(|| edges.get_exact(YG_EDGE_ALL))
                .unwrap_or(fallback)
        }
        YG_EDGE_TOP => edges
            .get_exact(YG_EDGE_TOP)
            .or_else(|| edges.get_exact(YG_EDGE_VERTICAL))
            .or_else(|| edges.get_exact(YG_EDGE_ALL))
            .unwrap_or(fallback),
        YG_EDGE_BOTTOM => edges
            .get_exact(YG_EDGE_BOTTOM)
            .or_else(|| edges.get_exact(YG_EDGE_VERTICAL))
            .or_else(|| edges.get_exact(YG_EDGE_ALL))
            .unwrap_or(fallback),
        _ => fallback,
    }
}

fn resolve_inset_rect(edges: &YogaEdgeValuesLpa, direction: i32) -> Rect<LengthPercentageAuto> {
    Rect {
        left: choose_lpa(edges, YG_EDGE_LEFT, direction, LengthPercentageAuto::auto()),
        top: choose_lpa(edges, YG_EDGE_TOP, direction, LengthPercentageAuto::auto()),
        right: choose_lpa(edges, YG_EDGE_RIGHT, direction, LengthPercentageAuto::auto()),
        bottom: choose_lpa(edges, YG_EDGE_BOTTOM, direction, LengthPercentageAuto::auto()),
    }
}

fn resolve_margin_rect(edges: &YogaEdgeValuesLpa, direction: i32) -> Rect<LengthPercentageAuto> {
    Rect {
        left: choose_lpa(edges, YG_EDGE_LEFT, direction, LengthPercentageAuto::length(0.0)),
        top: choose_lpa(edges, YG_EDGE_TOP, direction, LengthPercentageAuto::length(0.0)),
        right: choose_lpa(edges, YG_EDGE_RIGHT, direction, LengthPercentageAuto::length(0.0)),
        bottom: choose_lpa(edges, YG_EDGE_BOTTOM, direction, LengthPercentageAuto::length(0.0)),
    }
}

fn resolve_lp_rect(edges: &YogaEdgeValuesLp, direction: i32) -> Rect<LengthPercentage> {
    Rect {
        left: choose_lp(edges, YG_EDGE_LEFT, direction, LengthPercentage::length(0.0)),
        top: choose_lp(edges, YG_EDGE_TOP, direction, LengthPercentage::length(0.0)),
        right: choose_lp(edges, YG_EDGE_RIGHT, direction, LengthPercentage::length(0.0)),
        bottom: choose_lp(edges, YG_EDGE_BOTTOM, direction, LengthPercentage::length(0.0)),
    }
}

fn compact_to_yg_value(raw: taffy::style::CompactLength) -> YGValue {
    match raw.tag() {
        taffy::style::CompactLength::LENGTH_TAG => YGValue {
            value: raw.value(),
            unit: YG_UNIT_POINT,
        },
        taffy::style::CompactLength::PERCENT_TAG => YGValue {
            value: raw.value() * 100.0,
            unit: YG_UNIT_PERCENT,
        },
        taffy::style::CompactLength::AUTO_TAG => YGValue {
            value: f32::NAN,
            unit: YG_UNIT_AUTO,
        },
        taffy::style::CompactLength::MAX_CONTENT_TAG => YGValue {
            value: f32::NAN,
            unit: YG_UNIT_MAX_CONTENT,
        },
        taffy::style::CompactLength::FIT_CONTENT_PX_TAG | taffy::style::CompactLength::FIT_CONTENT_PERCENT_TAG => YGValue {
            value: f32::NAN,
            unit: YG_UNIT_FIT_CONTENT,
        },
        taffy::style::CompactLength::FR_TAG => YGValue {
            value: raw.value(),
            unit: YG_UNIT_UNDEFINED,
        },
        _ => YGValue {
            value: f32::NAN,
            unit: YG_UNIT_UNDEFINED,
        },
    }
}

fn dim_to_yg_value(value: Dimension) -> YGValue {
    compact_to_yg_value(value.into_raw())
}

fn dim_max_content() -> Dimension {
    unsafe { Dimension::from_raw(CompactLength::max_content()) }
}

fn dim_fit_content() -> Dimension {
    unsafe { Dimension::from_raw(CompactLength::fit_content_px(f32::INFINITY)) }
}

fn dim_stretch() -> Dimension {
    // Taffy does not currently expose a stretch unit for dimensions.
    Dimension::auto()
}

fn lpa_to_yg_value(value: LengthPercentageAuto) -> YGValue {
    compact_to_yg_value(value.into_raw())
}

fn lp_to_yg_value(value: LengthPercentage) -> YGValue {
    compact_to_yg_value(value.into_raw())
}

fn resolve_flex_grow(node: &YGNode) -> f32 {
    if node.owner.is_null() {
        return 0.0;
    }
    if node.has_flex_grow {
        return node.style.flex_grow;
    }
    match node.flex {
        Some(v) if v > 0.0 => v,
        _ => 0.0,
    }
}

fn resolve_flex_shrink(node: &YGNode) -> f32 {
    if node.owner.is_null() {
        return 0.0;
    }
    if node.has_flex_shrink {
        return node.style.flex_shrink;
    }
    let use_web_defaults = unsafe { !node.config.is_null() && (*node.config).use_web_defaults };
    match node.flex {
        Some(v) if !use_web_defaults && v < 0.0 => -v,
        _ => {
            if use_web_defaults {
                1.0
            } else {
                0.0
            }
        }
    }
}

fn resolve_flex_basis(node: &YGNode, mut style: Style) -> Style {
    let use_web_defaults = unsafe { !node.config.is_null() && (*node.config).use_web_defaults };
    if node.flex.is_some_and(|v| v > 0.0) && style.flex_basis.is_auto() {
        style.flex_basis = if use_web_defaults {
            Dimension::auto()
        } else {
            Dimension::length(0.0)
        };
    }
    style
}

fn resolve_direction(node_direction: i32, owner_direction: i32) -> i32 {
    if node_direction == YG_DIRECTION_INHERIT {
        if owner_direction == YG_DIRECTION_INHERIT {
            YG_DIRECTION_LTR
        } else {
            owner_direction
        }
    } else {
        node_direction
    }
}

unsafe fn node_global_location(mut node: *const YGNode) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    while !node.is_null() {
        x += (*node).layout.unrounded_layout.location.x;
        y += (*node).layout.unrounded_layout.location.y;
        node = (*node).owner;
    }
    (x, y)
}

unsafe fn absolute_containing_block(mut owner: *mut YGNode) -> *mut YGNode {
    while !owner.is_null() {
        let o = &*owner;
        if o.owner.is_null() || o.position_type_mode != YG_POSITION_TYPE_STATIC || o.always_forms_containing_block {
            return owner;
        }
        owner = o.owner;
    }
    owner
}

struct BuildNodeResult {
    ids: Vec<taffy::NodeId>,
    mapping: Vec<(*mut YGNode, taffy::NodeId)>,
    contents_nodes: Vec<*mut YGNode>,
}

fn build_taffy_tree(
    tree: &mut TaffyTree<*mut YGNode>,
    node: *mut YGNode,
    owner_direction: i32,
) -> BuildNodeResult {
    unsafe {
        let n = &mut *node;
        let direction = resolve_direction(n.direction, owner_direction);
        let mut style = n.style.clone();
        style.inset = resolve_inset_rect(&n.position_edges, direction);
        if n.position_type_mode == YG_POSITION_TYPE_STATIC {
            style.inset = Rect::auto();
        }
        style.margin = resolve_margin_rect(&n.margin_edges, direction);
        style.padding = resolve_lp_rect(&n.padding_edges, direction);
        style.border = resolve_lp_rect(&n.border_edges, direction);
        if direction == YG_DIRECTION_RTL {
            style.flex_direction = match style.flex_direction {
                FlexDirection::Row => FlexDirection::RowReverse,
                FlexDirection::RowReverse => FlexDirection::Row,
                other => other,
            };
        }
        style.flex_grow = resolve_flex_grow(n);
        style.flex_shrink = resolve_flex_shrink(n);
        style = resolve_flex_basis(n, style);
        n.layout.direction = direction;

        let mut child_ids = Vec::with_capacity(n.children.len());
        let mut mapping = Vec::new();
        let mut contents_nodes = Vec::new();
        for child in n.children.iter().copied() {
            let result = build_taffy_tree(tree, child, direction);
            child_ids.extend(result.ids);
            mapping.extend(result.mapping);
            contents_nodes.extend(result.contents_nodes);
        }

        if n.display_mode == YG_DISPLAY_CONTENTS {
            contents_nodes.push(node);
            return BuildNodeResult {
                ids: child_ids,
                mapping,
                contents_nodes,
            };
        }

        let id = if child_ids.is_empty() {
            tree.new_leaf_with_context(style, node).unwrap()
        } else {
            tree.new_with_children(style, &child_ids).unwrap()
        };

        mapping.push((node, id));
        BuildNodeResult {
            ids: vec![id],
            mapping,
            contents_nodes,
        }
    }
}

#[no_mangle]
pub extern "C" fn YGFloatIsUndefined(value: f32) -> bool {
    value.is_nan()
}

#[no_mangle]
pub extern "C" fn YGRoundValueToPixelGrid(
    value: f64,
    point_scale_factor: f64,
    force_ceil: bool,
    force_floor: bool,
) -> f32 {
    round_value_to_pixel_grid(value, point_scale_factor, force_ceil, force_floor)
}

#[no_mangle]
pub extern "C" fn YGConfigNew() -> *mut YGConfig {
    Box::into_raw(Box::new(YGConfig {
        use_web_defaults: false,
        point_scale_factor: 1.0,
        errata: 0,
        logger: None,
        context: ptr::null_mut(),
        clone_node_func: None,
        experimental_web_flex_basis: false,
    }))
}

#[no_mangle]
pub extern "C" fn YGConfigFree(config: *mut YGConfig) {
    if !config.is_null() {
        unsafe {
            drop(Box::from_raw(config));
        }
    }
}

#[no_mangle]
pub extern "C" fn YGConfigGetDefault() -> *const YGConfig {
    config_default()
}

#[no_mangle]
pub extern "C" fn YGConfigSetUseWebDefaults(config: *mut YGConfig, enabled: bool) {
    unsafe {
        if let Some(c) = config.as_mut() {
            c.use_web_defaults = enabled;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGConfigGetUseWebDefaults(config: *const YGConfig) -> bool {
    unsafe { config.as_ref().map(|c| c.use_web_defaults).unwrap_or(false) }
}

#[no_mangle]
pub extern "C" fn YGConfigSetPointScaleFactor(config: *mut YGConfig, pixels_in_point: f32) {
    unsafe {
        if let Some(c) = config.as_mut() {
            c.point_scale_factor = pixels_in_point;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGConfigGetPointScaleFactor(config: *const YGConfig) -> f32 {
    unsafe { config.as_ref().map(|c| c.point_scale_factor).unwrap_or(1.0) }
}

#[no_mangle]
pub extern "C" fn YGConfigSetErrata(config: *mut YGConfig, errata: i32) {
    unsafe {
        if let Some(c) = config.as_mut() {
            c.errata = errata;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGConfigGetErrata(config: *const YGConfig) -> i32 {
    unsafe { config.as_ref().map(|c| c.errata).unwrap_or(0) }
}

#[no_mangle]
pub extern "C" fn YGConfigSetLogger(config: *mut YGConfig, logger: Option<YGLogger>) {
    unsafe {
        if let Some(c) = config.as_mut() {
            c.logger = logger;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGConfigSetContext(config: *mut YGConfig, context: *mut c_void) {
    unsafe {
        if let Some(c) = config.as_mut() {
            c.context = context;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGConfigGetContext(config: *const YGConfig) -> *mut c_void {
    unsafe { config.as_ref().map(|c| c.context).unwrap_or(ptr::null_mut()) }
}

#[no_mangle]
pub extern "C" fn YGConfigSetExperimentalFeatureEnabled(config: *mut YGConfig, _feature: i32, enabled: bool) {
    unsafe {
        if let Some(c) = config.as_mut() {
            c.experimental_web_flex_basis = enabled;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGConfigIsExperimentalFeatureEnabled(config: *const YGConfig, _feature: i32) -> bool {
    unsafe {
        config
            .as_ref()
            .map(|c| c.experimental_web_flex_basis)
            .unwrap_or(false)
    }
}

#[no_mangle]
pub extern "C" fn YGConfigSetCloneNodeFunc(config: *mut YGConfig, callback: Option<YGCloneNodeFunc>) {
    unsafe {
        if let Some(c) = config.as_mut() {
            c.clone_node_func = callback;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeNew() -> *mut YGNode {
    YGNodeNewWithConfig(config_default())
}

#[no_mangle]
pub extern "C" fn YGNodeNewWithConfig(config: *const YGConfig) -> *mut YGNode {
    let config = if config.is_null() {
        config_default()
    } else {
        config as *mut YGConfig
    };
    let style = unsafe { default_style((*config).use_web_defaults) };
    Box::into_raw(Box::new(YGNode {
        style,
        display_mode: YG_DISPLAY_FLEX,
        position_type_mode: YG_POSITION_TYPE_RELATIVE,
        owner: ptr::null_mut(),
        children: Vec::new(),
        config,
        context: ptr::null_mut(),
        has_new_layout: true,
        is_dirty: true,
        dirtied_func: None,
        measure_func: None,
        baseline_func: None,
        is_reference_baseline: false,
        node_type: YG_NODE_TYPE_DEFAULT,
        always_forms_containing_block: false,
        direction: YG_DIRECTION_INHERIT,
        flex: None,
        has_flex_grow: false,
        has_flex_shrink: false,
        position_edges: YogaEdgeValuesLpa::default(),
        margin_edges: YogaEdgeValuesLpa::default(),
        padding_edges: YogaEdgeValuesLp::default(),
        border_edges: YogaEdgeValuesLp::default(),
        layout: YGLayout::default(),
    }))
}

unsafe fn node_clone_shallow(node: *const YGNode) -> *mut YGNode {
    let n = &*node;
    Box::into_raw(Box::new(YGNode {
        style: n.style.clone(),
        display_mode: n.display_mode,
        position_type_mode: n.position_type_mode,
        owner: ptr::null_mut(),
        children: n.children.clone(),
        config: n.config,
        context: n.context,
        has_new_layout: true,
        is_dirty: true,
        dirtied_func: n.dirtied_func,
        measure_func: n.measure_func,
        baseline_func: n.baseline_func,
        is_reference_baseline: n.is_reference_baseline,
        node_type: n.node_type,
        always_forms_containing_block: n.always_forms_containing_block,
        direction: n.direction,
        flex: n.flex,
        has_flex_grow: n.has_flex_grow,
        has_flex_shrink: n.has_flex_shrink,
        position_edges: n.position_edges.clone(),
        margin_edges: n.margin_edges.clone(),
        padding_edges: n.padding_edges.clone(),
        border_edges: n.border_edges.clone(),
        layout: YGLayout::default(),
    }))
}

#[no_mangle]
pub extern "C" fn YGNodeClone(node: *const YGNode) -> *mut YGNode {
    if node.is_null() {
        return ptr::null_mut();
    }
    unsafe { node_clone_shallow(node) }
}

#[no_mangle]
pub extern "C" fn YGNodeFinalize(node: *mut YGNode) {
    if !node.is_null() {
        unsafe {
            drop(Box::from_raw(node));
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeFree(node: *mut YGNode) {
    if node.is_null() {
        return;
    }
    unsafe {
        if !(*node).owner.is_null() {
            YGNodeRemoveChild((*node).owner, node);
        }
        for child in (*node).children.iter().copied() {
            (*child).owner = ptr::null_mut();
        }
        (*node).children.clear();
        drop(Box::from_raw(node));
    }
}

#[no_mangle]
pub extern "C" fn YGNodeFreeRecursive(node: *mut YGNode) {
    if node.is_null() {
        return;
    }
    unsafe {
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            let children = (*current).children.clone();
            for child in children {
                stack.push(child);
            }
            (*current).children.clear();
            (*current).owner = ptr::null_mut();
            drop(Box::from_raw(current));
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeReset(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            if !n.children.is_empty() || !n.owner.is_null() {
                return;
            }
            let use_web_defaults = !n.config.is_null() && (*n.config).use_web_defaults;
            let context = n.context;
            *n = YGNode {
                style: default_style(use_web_defaults),
                display_mode: YG_DISPLAY_FLEX,
                position_type_mode: YG_POSITION_TYPE_RELATIVE,
                owner: ptr::null_mut(),
                children: Vec::new(),
                config: n.config,
                context,
                has_new_layout: true,
                is_dirty: true,
                dirtied_func: None,
                measure_func: None,
                baseline_func: None,
                is_reference_baseline: false,
                node_type: YG_NODE_TYPE_DEFAULT,
                always_forms_containing_block: false,
                direction: YG_DIRECTION_INHERIT,
                flex: None,
                has_flex_grow: false,
                has_flex_shrink: false,
                position_edges: YogaEdgeValuesLpa::default(),
                margin_edges: YogaEdgeValuesLpa::default(),
                padding_edges: YogaEdgeValuesLp::default(),
                border_edges: YogaEdgeValuesLp::default(),
                layout: YGLayout::default(),
            };
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeCalculateLayout(
    node: *mut YGNode,
    available_width: f32,
    available_height: f32,
    owner_direction: i32,
) {
    if node.is_null() {
        return;
    }

    let _ = catch_unwind(AssertUnwindSafe(|| {
        let mut tree: TaffyTree<*mut YGNode> = TaffyTree::new();
        tree.disable_rounding();

        let build = build_taffy_tree(&mut tree, node, owner_direction);
        let root_id = if build.ids.is_empty() {
            tree.new_leaf_with_context(Style::DEFAULT, ptr::null_mut()).unwrap()
        } else if build.ids.len() == 1 {
            build.ids[0]
        } else {
            tree.new_with_children(Style::DEFAULT, &build.ids).unwrap()
        };

        let _ = tree.compute_layout_with_measure(
            root_id,
            Size {
                width: to_available_space(available_width),
                height: to_available_space(available_height),
            },
            |known, available, _node_id, context, _style| {
                let node_ptr = context.copied().unwrap_or(ptr::null_mut());
                if node_ptr.is_null() {
                    return Size::ZERO;
                }
                unsafe {
                    let n = &*node_ptr;
                    if let Some(measure) = n.measure_func {
                        let (w, wm) = if let Some(v) = known.width {
                            (v, YG_MEASURE_MODE_EXACTLY)
                        } else {
                            match available.width {
                                AvailableSpace::Definite(v) => (v, YG_MEASURE_MODE_AT_MOST),
                                _ => (f32::NAN, YG_MEASURE_MODE_UNDEFINED),
                            }
                        };
                        let (h, hm) = if let Some(v) = known.height {
                            (v, YG_MEASURE_MODE_EXACTLY)
                        } else {
                            match available.height {
                                AvailableSpace::Definite(v) => (v, YG_MEASURE_MODE_AT_MOST),
                                _ => (f32::NAN, YG_MEASURE_MODE_UNDEFINED),
                            }
                        };
                        let measured = measure(node_ptr, w, wm, h, hm);
                        Size {
                            width: measured.width.max(0.0),
                            height: measured.height.max(0.0),
                        }
                    } else {
                        Size::ZERO
                    }
                }
            },
        );

        let BuildNodeResult {
            mapping,
            contents_nodes,
            ..
        } = build;
        for (node_ptr, id) in mapping.iter().copied() {
            unsafe {
                if let Ok(layout) = tree.layout(id) {
                    (*node_ptr).layout.unrounded_layout = *layout;
                    (*node_ptr).layout.final_layout = *layout;
                    if (*node_ptr).owner.is_null() {
                        // Yoga reports root offsets including root margins.
                        let margin = resolve_margin_rect(&(*node_ptr).margin_edges, (*node_ptr).layout.direction);
                        let margin_left = margin.left.resolve_to_option(0.0, |_ptr, _ctx| 0.0).unwrap_or(0.0);
                        let margin_top = margin.top.resolve_to_option(0.0, |_ptr, _ctx| 0.0).unwrap_or(0.0);
                        (*node_ptr).layout.unrounded_layout.location.x += margin_left;
                        (*node_ptr).layout.unrounded_layout.location.y += margin_top;
                        (*node_ptr).layout.final_layout.location.x += margin_left;
                        (*node_ptr).layout.final_layout.location.y += margin_top;

                        // Yoga also applies root insets even when the node has no owner.
                        let inset = resolve_inset_rect(&(*node_ptr).position_edges, (*node_ptr).layout.direction);
                        let containing_width = if available_width.is_nan() { 0.0 } else { available_width };
                        let containing_height = if available_height.is_nan() { 0.0 } else { available_height };

                        let left = inset.left.resolve_to_option(containing_width, |_ptr, _ctx| 0.0);
                        let right = inset.right.resolve_to_option(containing_width, |_ptr, _ctx| 0.0);
                        let top = inset.top.resolve_to_option(containing_height, |_ptr, _ctx| 0.0);
                        let bottom = inset.bottom.resolve_to_option(containing_height, |_ptr, _ctx| 0.0);

                        if let Some(v) = left {
                            (*node_ptr).layout.unrounded_layout.location.x += v;
                            (*node_ptr).layout.final_layout.location.x += v;
                        } else if let Some(v) = right {
                            let dx = containing_width - (*node_ptr).layout.unrounded_layout.size.width - v;
                            (*node_ptr).layout.unrounded_layout.location.x += dx;
                            (*node_ptr).layout.final_layout.location.x += dx;
                        }

                        if let Some(v) = top {
                            (*node_ptr).layout.unrounded_layout.location.y += v;
                            (*node_ptr).layout.final_layout.location.y += v;
                        } else if let Some(v) = bottom {
                            let dy = containing_height - (*node_ptr).layout.unrounded_layout.size.height - v;
                            (*node_ptr).layout.unrounded_layout.location.y += dy;
                            (*node_ptr).layout.final_layout.location.y += dy;
                        }
                    }
                    (*node_ptr).layout.had_overflow = false;
                    (*node_ptr).has_new_layout = true;
                    (*node_ptr).is_dirty = false;
                }
            }
        }

        for node_ptr in contents_nodes {
            unsafe {
                (*node_ptr).layout.unrounded_layout = Layout::new();
                (*node_ptr).layout.final_layout = Layout::new();
                (*node_ptr).layout.direction = resolve_direction((*node_ptr).direction, owner_direction);
                (*node_ptr).layout.had_overflow = false;
                (*node_ptr).has_new_layout = true;
                (*node_ptr).is_dirty = false;
            }
        }

        let pre_mirror_globals: Vec<(*mut YGNode, (f32, f32))> = mapping
            .iter()
            .map(|(node_ptr, _id)| unsafe { (*node_ptr, node_global_location(*node_ptr)) })
            .collect();

        for (node_ptr, _id) in mapping.iter().copied() {
            unsafe {
                let n = &mut *node_ptr;
                if n.style.position != Position::Absolute || n.owner.is_null() || n.display_mode == YG_DISPLAY_NONE {
                    continue;
                }

                let direct_owner = &*n.owner;
                let containing_ptr = absolute_containing_block(n.owner);
                if containing_ptr.is_null() {
                    continue;
                }
                let containing = &*containing_ptr;
                let use_static_cb_compat = containing_ptr != n.owner;

                let owner_width = containing.layout.unrounded_layout.size.width;
                let owner_height = containing.layout.unrounded_layout.size.height;
                let mut width = n.layout.unrounded_layout.size.width;
                let mut height = n.layout.unrounded_layout.size.height;
                if !owner_width.is_finite() || !owner_height.is_finite() || !width.is_finite() || !height.is_finite() {
                    continue;
                }

                let owner_border = containing.layout.final_layout.border;
                let owner_padding = containing.layout.final_layout.padding;
                let content_left = owner_border.left + owner_padding.left;
                let content_top = owner_border.top + owner_padding.top;
                let content_width =
                    (owner_width - owner_border.left - owner_border.right - owner_padding.left - owner_padding.right).max(0.0);
                let content_height =
                    (owner_height - owner_border.top - owner_border.bottom - owner_padding.top - owner_padding.bottom).max(0.0);
                let containing_padding_box_width = (owner_width - owner_border.left - owner_border.right).max(0.0);
                let containing_padding_box_height = (owner_height - owner_border.top - owner_border.bottom).max(0.0);
                let inset_resolve_width = if use_static_cb_compat {
                    containing_padding_box_width
                } else {
                    content_width
                };
                let inset_resolve_height = if use_static_cb_compat {
                    containing_padding_box_height
                } else {
                    content_height
                };
                let margin_resolve_width = if use_static_cb_compat {
                    containing_padding_box_width
                } else {
                    content_width
                };
                let margin_resolve_height = if use_static_cb_compat {
                    containing_padding_box_height
                } else {
                    content_height
                };
                let width_value = dim_to_yg_value(n.style.size.width);
                let height_value = dim_to_yg_value(n.style.size.height);
                let width_is_percent = width_value.unit == YG_UNIT_PERCENT;
                let height_is_percent = height_value.unit == YG_UNIT_PERCENT;
                let has_percent_position = [
                    YG_EDGE_LEFT,
                    YG_EDGE_RIGHT,
                    YG_EDGE_START,
                    YG_EDGE_END,
                    YG_EDGE_TOP,
                    YG_EDGE_BOTTOM,
                    YG_EDGE_HORIZONTAL,
                    YG_EDGE_VERTICAL,
                    YG_EDGE_ALL,
                ]
                    .iter()
                    .any(|edge| {
                        n.position_edges
                            .get_exact(*edge)
                            .map(|v| lpa_to_yg_value(v).unit == YG_UNIT_PERCENT)
                            .unwrap_or(false)
                    });
                let has_percent_margin = [
                    YG_EDGE_LEFT,
                    YG_EDGE_RIGHT,
                    YG_EDGE_START,
                    YG_EDGE_END,
                    YG_EDGE_TOP,
                    YG_EDGE_BOTTOM,
                    YG_EDGE_HORIZONTAL,
                    YG_EDGE_VERTICAL,
                    YG_EDGE_ALL,
                ]
                    .iter()
                    .any(|edge| {
                        n.margin_edges
                            .get_exact(*edge)
                            .map(|v| lpa_to_yg_value(v).unit == YG_UNIT_PERCENT)
                            .unwrap_or(false)
                    });
                let has_percent_padding = [
                    YG_EDGE_LEFT,
                    YG_EDGE_RIGHT,
                    YG_EDGE_START,
                    YG_EDGE_END,
                    YG_EDGE_TOP,
                    YG_EDGE_BOTTOM,
                    YG_EDGE_HORIZONTAL,
                    YG_EDGE_VERTICAL,
                    YG_EDGE_ALL,
                ]
                    .iter()
                    .any(|edge| {
                        n.padding_edges
                            .get_exact(*edge)
                            .map(|v| lp_to_yg_value(v).unit == YG_UNIT_PERCENT)
                            .unwrap_or(false)
                    });
                let has_percent_border = [
                    YG_EDGE_LEFT,
                    YG_EDGE_RIGHT,
                    YG_EDGE_START,
                    YG_EDGE_END,
                    YG_EDGE_TOP,
                    YG_EDGE_BOTTOM,
                    YG_EDGE_HORIZONTAL,
                    YG_EDGE_VERTICAL,
                    YG_EDGE_ALL,
                ]
                    .iter()
                    .any(|edge| {
                        n.border_edges
                            .get_exact(*edge)
                            .map(|v| lp_to_yg_value(v).unit == YG_UNIT_PERCENT)
                            .unwrap_or(false)
                    });
                let use_containing_for_auto_axes = width_is_percent
                    || height_is_percent
                    || has_percent_position
                    || has_percent_margin
                    || has_percent_padding
                    || has_percent_border;

                if use_static_cb_compat {
                    if width_is_percent {
                        width = containing_padding_box_width * (width_value.value / 100.0);
                        n.layout.unrounded_layout.size.width = width;
                        n.layout.final_layout.size.width = width;
                    }
                    if height_is_percent {
                        height = containing_padding_box_height * (height_value.value / 100.0);
                        n.layout.unrounded_layout.size.height = height;
                        n.layout.final_layout.size.height = height;
                    }
                }

                let direct_owner_width = direct_owner.layout.unrounded_layout.size.width;
                let direct_owner_height = direct_owner.layout.unrounded_layout.size.height;
                let direct_owner_border = direct_owner.layout.final_layout.border;
                let direct_owner_padding = direct_owner.layout.final_layout.padding;
                let direct_content_left = direct_owner_border.left + direct_owner_padding.left;
                let direct_content_top = direct_owner_border.top + direct_owner_padding.top;
                let direct_content_width = (direct_owner_width
                    - direct_owner_border.left
                    - direct_owner_border.right
                    - direct_owner_padding.left
                    - direct_owner_padding.right)
                    .max(0.0);
                let direct_content_height = (direct_owner_height
                    - direct_owner_border.top
                    - direct_owner_border.bottom
                    - direct_owner_padding.top
                    - direct_owner_padding.bottom)
                    .max(0.0);

                let (direct_owner_global_x, direct_owner_global_y) = node_global_location(n.owner);
                let (containing_global_x, containing_global_y) = node_global_location(containing_ptr);
                let owner_space_offset_x = containing_global_x - direct_owner_global_x;
                let owner_space_offset_y = containing_global_y - direct_owner_global_y;

                let owner_flex_direction = direct_owner.style.flex_direction;
                let main_axis_is_row =
                    matches!(owner_flex_direction, FlexDirection::Row | FlexDirection::RowReverse);
                let main_axis_is_reverse =
                    matches!(owner_flex_direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse);
                let cross_axis_is_reverse = matches!(direct_owner.style.flex_wrap, FlexWrap::WrapReverse);

                #[derive(Copy, Clone)]
                enum YogaAxisAlign {
                    Start,
                    End,
                    Center,
                }

                fn flip_axis_align(value: YogaAxisAlign) -> YogaAxisAlign {
                    match value {
                        YogaAxisAlign::Start => YogaAxisAlign::End,
                        YogaAxisAlign::End => YogaAxisAlign::Start,
                        YogaAxisAlign::Center => YogaAxisAlign::Center,
                    }
                }

                let main_align = match direct_owner.style.justify_content {
                    Some(JustifyContent::Center) => YogaAxisAlign::Center,
                    Some(JustifyContent::FlexEnd) | Some(JustifyContent::End) => YogaAxisAlign::End,
                    _ => YogaAxisAlign::Start,
                };
                let cross_align = match n.style.align_self.or(direct_owner.style.align_items) {
                    Some(AlignItems::Center) => YogaAxisAlign::Center,
                    Some(AlignItems::FlexEnd) | Some(AlignItems::End) => YogaAxisAlign::End,
                    _ => YogaAxisAlign::Start,
                };

                let resolved_main_align = if main_axis_is_reverse {
                    flip_axis_align(main_align)
                } else {
                    main_align
                };
                let resolved_cross_align = if cross_axis_is_reverse {
                    flip_axis_align(cross_align)
                } else {
                    cross_align
                };

                let axis_x_align = if main_axis_is_row {
                    resolved_main_align
                } else {
                    resolved_cross_align
                };
                let axis_y_align = if main_axis_is_row {
                    resolved_cross_align
                } else {
                    resolved_main_align
                };

                let place_on_axis = |origin: f32,
                                     available: f32,
                                     child: f32,
                                     align: YogaAxisAlign,
                                     logical_start_is_low: bool,
                                     margin_start: f32,
                                     margin_end: f32| {
                    let free = (available - child - margin_start - margin_end).max(0.0);
                    let logical_pos = match align {
                        YogaAxisAlign::Center => margin_start + (free / 2.0),
                        YogaAxisAlign::Start => margin_start,
                        YogaAxisAlign::End => margin_start + free,
                    };
                    if logical_start_is_low {
                        origin + logical_pos
                    } else {
                        origin + (available - logical_pos - child)
                    }
                };

                let start_raw = n.position_edges.get_exact(YG_EDGE_START);
                let end_raw = n.position_edges.get_exact(YG_EDGE_END);
                let left_raw = n.position_edges.get_exact(YG_EDGE_LEFT);
                let right_raw = n.position_edges.get_exact(YG_EDGE_RIGHT);
                let top_raw = n.position_edges.get_exact(YG_EDGE_TOP);
                let bottom_raw = n.position_edges.get_exact(YG_EDGE_BOTTOM);

                let start = start_raw.and_then(|v| v.resolve_to_option(inset_resolve_width, |_ptr, _ctx| 0.0));
                let end = end_raw.and_then(|v| v.resolve_to_option(inset_resolve_width, |_ptr, _ctx| 0.0));
                let left = left_raw.and_then(|v| v.resolve_to_option(inset_resolve_width, |_ptr, _ctx| 0.0));
                let right = right_raw.and_then(|v| v.resolve_to_option(inset_resolve_width, |_ptr, _ctx| 0.0));
                let top = top_raw.and_then(|v| v.resolve_to_option(inset_resolve_height, |_ptr, _ctx| 0.0));
                let bottom = bottom_raw.and_then(|v| v.resolve_to_option(inset_resolve_height, |_ptr, _ctx| 0.0));
                let resolved_margin = resolve_margin_rect(&n.margin_edges, n.layout.direction);
                let margin_left = resolved_margin
                    .left
                    .resolve_to_option(margin_resolve_width, |_ptr, _ctx| 0.0)
                    .unwrap_or(0.0);
                let margin_right = resolved_margin
                    .right
                    .resolve_to_option(margin_resolve_width, |_ptr, _ctx| 0.0)
                    .unwrap_or(0.0);
                let margin_top = resolved_margin
                    .top
                    .resolve_to_option(margin_resolve_height, |_ptr, _ctx| 0.0)
                    .unwrap_or(0.0);
                let margin_bottom = resolved_margin
                    .bottom
                    .resolve_to_option(margin_resolve_height, |_ptr, _ctx| 0.0)
                    .unwrap_or(0.0);

                let has_horizontal_non_auto = start.is_some()
                    || end.is_some()
                    || left.is_some()
                    || right.is_some();
                let has_vertical_non_auto = top.is_some() || bottom.is_some();

                if let Some(start_inset) = start {
                    // `start` wins over `end` for RTL with definite width (Yoga behavior).
                    let x = if n.layout.direction == YG_DIRECTION_RTL {
                        content_left + content_width - width - start_inset
                    } else {
                        content_left + start_inset
                    } + owner_space_offset_x;
                    n.layout.unrounded_layout.location.x = x;
                    n.layout.final_layout.location.x = x;
                } else if n.layout.direction == YG_DIRECTION_RTL
                    && start_raw.is_none()
                    && end_raw.is_none()
                    && left.is_some()
                    && right.is_some()
                {
                    // Yoga favors the physical right inset for this RTL left+right absolute case.
                    let x =
                        content_left + content_width - width - right.unwrap_or(0.0) - n.layout.final_layout.margin.right
                            + owner_space_offset_x;
                    n.layout.unrounded_layout.location.x = x;
                    n.layout.final_layout.location.x = x;
                } else if use_static_cb_compat && left.is_some() {
                    let left_inset = left.unwrap_or(0.0);
                    let x = content_left + left_inset + owner_space_offset_x;
                    n.layout.unrounded_layout.location.x = x;
                    n.layout.final_layout.location.x = x;
                } else if use_static_cb_compat && end.is_some() {
                    let end_inset = end.unwrap_or(0.0);
                    let x = if n.layout.direction == YG_DIRECTION_RTL {
                        content_left + end_inset
                    } else {
                        content_left + content_width - width - end_inset
                    } + owner_space_offset_x;
                    n.layout.unrounded_layout.location.x = x;
                    n.layout.final_layout.location.x = x;
                } else if use_static_cb_compat && right.is_some() {
                    let right_inset = right.unwrap_or(0.0);
                    let x = content_left + content_width - width - right_inset + owner_space_offset_x;
                    n.layout.unrounded_layout.location.x = x;
                    n.layout.final_layout.location.x = x;
                } else if !has_horizontal_non_auto {
                    let (align_origin_x, align_width) = if use_static_cb_compat && use_containing_for_auto_axes {
                        (content_left, content_width)
                    } else if use_static_cb_compat {
                        (direct_content_left, direct_content_width)
                    } else {
                        (content_left, content_width)
                    };
                    let x_logical_start_is_low = n.layout.direction != YG_DIRECTION_RTL;
                    let x_margin_start = if x_logical_start_is_low {
                        margin_left
                    } else {
                        margin_right
                    };
                    let x_margin_end = if x_logical_start_is_low {
                        margin_right
                    } else {
                        margin_left
                    };
                    let x = place_on_axis(
                        align_origin_x,
                        align_width,
                        width,
                        axis_x_align,
                        x_logical_start_is_low,
                        x_margin_start,
                        x_margin_end,
                    ) + owner_space_offset_x;
                    n.layout.unrounded_layout.location.x = x;
                    n.layout.final_layout.location.x = x;
                }

                if use_static_cb_compat && top.is_some() {
                    let top_inset = top.unwrap_or(0.0);
                    let y = content_top + top_inset + owner_space_offset_y;
                    n.layout.unrounded_layout.location.y = y;
                    n.layout.final_layout.location.y = y;
                } else if use_static_cb_compat && bottom.is_some() {
                    let bottom_inset = bottom.unwrap_or(0.0);
                    let y = content_top + content_height - height - bottom_inset + owner_space_offset_y;
                    n.layout.unrounded_layout.location.y = y;
                    n.layout.final_layout.location.y = y;
                } else if !has_vertical_non_auto {
                    let (align_origin_y, align_height) = if use_static_cb_compat && use_containing_for_auto_axes {
                        (content_top, content_height)
                    } else if use_static_cb_compat {
                        (direct_content_top, direct_content_height)
                    } else {
                        (content_top, content_height)
                    };
                    let y = place_on_axis(
                        align_origin_y,
                        align_height,
                        height,
                        axis_y_align,
                        true,
                        margin_top,
                        margin_bottom,
                    ) + owner_space_offset_y;
                    n.layout.unrounded_layout.location.y = y;
                    n.layout.final_layout.location.y = y;
                }
            }
        }

        for (node_ptr, _id) in mapping.iter().copied() {
            unsafe {
                let n = &mut *node_ptr;
                if n.owner.is_null() || n.display_mode == YG_DISPLAY_NONE {
                    continue;
                }

                let owner = &*n.owner;
                if owner.display_mode == YG_DISPLAY_CONTENTS {
                    continue;
                }
                if owner.layout.direction != YG_DIRECTION_RTL
                    || !matches!(owner.style.flex_direction, FlexDirection::Column | FlexDirection::ColumnReverse)
                {
                    continue;
                }
                if owner.padding_edges.get_exact(YG_EDGE_START).is_some()
                    || owner.padding_edges.get_exact(YG_EDGE_END).is_some()
                    || owner.border_edges.get_exact(YG_EDGE_START).is_some()
                    || owner.border_edges.get_exact(YG_EDGE_END).is_some()
                {
                    continue;
                }
                if matches!(owner.style.flex_wrap, FlexWrap::WrapReverse) {
                    continue;
                }
                let start = n.position_edges.get_exact(YG_EDGE_START);
                let end = n.position_edges.get_exact(YG_EDGE_END);
                let left = n.position_edges.get_exact(YG_EDGE_LEFT);
                let right = n.position_edges.get_exact(YG_EDGE_RIGHT);
                if start.is_some() || end.is_some() || left.is_some() || right.is_some() {
                    continue;
                }
                let margin_start = n.margin_edges.get_exact(YG_EDGE_START);
                let margin_end = n.margin_edges.get_exact(YG_EDGE_END);
                let margin_left = n.margin_edges.get_exact(YG_EDGE_LEFT);
                let margin_right = n.margin_edges.get_exact(YG_EDGE_RIGHT);
                let has_horizontal_margin = margin_start.is_some()
                    || margin_end.is_some()
                    || margin_left.is_some()
                    || margin_right.is_some();

                let owner_width = owner.layout.unrounded_layout.size.width;
                let width = n.layout.unrounded_layout.size.width;
                let x = n.layout.unrounded_layout.location.x;
                if !owner_width.is_finite() || !width.is_finite() || !x.is_finite() {
                    continue;
                }
                if n.style.position == Position::Absolute {
                    continue;
                }

                let mut mirrored_x = owner_width - x - width;
                if has_horizontal_margin {
                    mirrored_x += n.layout.final_layout.margin.left - n.layout.final_layout.margin.right;
                }
                n.layout.unrounded_layout.location.x = mirrored_x;
                n.layout.final_layout.location.x = mirrored_x;
            }
        }

        for (node_ptr, _id) in mapping.iter().copied() {
            unsafe {
                let n = &mut *node_ptr;
                if n.owner.is_null() || n.display_mode == YG_DISPLAY_NONE || n.style.position == Position::Absolute {
                    continue;
                }

                let owner = &*n.owner;
                if owner.display_mode == YG_DISPLAY_CONTENTS
                    || owner.layout.direction != YG_DIRECTION_RTL
                    || owner.style.flex_wrap != FlexWrap::NoWrap
                    || owner.style.flex_direction != FlexDirection::Row
                {
                    continue;
                }
                if !matches!(
                    owner.style.justify_content,
                    Some(JustifyContent::SpaceBetween | JustifyContent::SpaceAround | JustifyContent::SpaceEvenly)
                ) {
                    continue;
                }

                let owner_width = owner.layout.unrounded_layout.size.width;
                if !owner_width.is_finite() {
                    continue;
                }

                let total_outer_width: f32 = owner
                    .children
                    .iter()
                    .copied()
                    .filter(|child_ptr| {
                        let child = &**child_ptr;
                        child.display_mode != YG_DISPLAY_NONE && child.style.position != Position::Absolute
                    })
                    .map(|child_ptr| {
                        let child = &*child_ptr;
                        let width = child.layout.unrounded_layout.size.width;
                        let margin = child.layout.final_layout.margin;
                        if width.is_finite() {
                            width + margin.left + margin.right
                        } else {
                            0.0
                        }
                    })
                    .sum();
                let overflow = total_outer_width - owner_width;
                if overflow <= 0.0001 {
                    continue;
                }

                n.layout.unrounded_layout.location.x -= overflow;
                n.layout.final_layout.location.x -= overflow;
            }
        }

        for (node_ptr, _id) in mapping.iter().copied() {
            unsafe {
                let n = &mut *node_ptr;
                if n.owner.is_null() || n.display_mode == YG_DISPLAY_NONE || n.style.position == Position::Absolute {
                    continue;
                }

                let owner = &*n.owner;
                if owner.display_mode == YG_DISPLAY_CONTENTS
                    || owner.style.flex_wrap != FlexWrap::NoWrap
                    || !matches!(owner.style.flex_direction, FlexDirection::Column | FlexDirection::ColumnReverse)
                    || owner.style.justify_content != Some(JustifyContent::Center)
                {
                    continue;
                }

                let margin_left_auto = n
                    .margin_edges
                    .get_exact(YG_EDGE_LEFT)
                    .map(|v| lpa_to_yg_value(v).unit == YG_UNIT_AUTO)
                    .unwrap_or(false);
                let margin_right_auto = n
                    .margin_edges
                    .get_exact(YG_EDGE_RIGHT)
                    .map(|v| lpa_to_yg_value(v).unit == YG_UNIT_AUTO)
                    .unwrap_or(false);
                if !margin_left_auto && !margin_right_auto {
                    continue;
                }

                let owner_width = owner.layout.unrounded_layout.size.width;
                let child_width = n.layout.unrounded_layout.size.width;
                if !owner_width.is_finite() || !child_width.is_finite() {
                    continue;
                }
                if child_width <= owner_width {
                    continue;
                }

                let margin_left = n.layout.final_layout.margin.left;
                let margin_right = n.layout.final_layout.margin.right;
                let x = if owner.layout.direction == YG_DIRECTION_RTL {
                    let rtl_right_margin = if margin_right_auto { 0.0 } else { margin_right };
                    owner_width - child_width - rtl_right_margin
                } else if margin_left_auto {
                    0.0
                } else {
                    margin_left
                };

                n.layout.unrounded_layout.location.x = x;
                n.layout.final_layout.location.x = x;
            }
        }

        for (node_ptr, _id) in mapping.iter().copied() {
            unsafe {
                let n = &mut *node_ptr;
                if n.owner.is_null() || n.display_mode == YG_DISPLAY_NONE || n.style.position == Position::Absolute {
                    continue;
                }
                if n.layout.direction != YG_DIRECTION_RTL {
                    continue;
                }

                let owner = &*n.owner;
                if owner.position_type_mode != YG_POSITION_TYPE_STATIC {
                    continue;
                }
                let containing_ptr = absolute_containing_block(n.owner);
                if containing_ptr.is_null() || containing_ptr == n.owner {
                    continue;
                }
                let containing = &*containing_ptr;
                let owner_width = owner.layout.unrounded_layout.size.width;
                let containing_width = containing.layout.unrounded_layout.size.width;
                if !owner_width.is_finite() || !containing_width.is_finite() {
                    continue;
                }
                let basis_delta = containing_width - owner_width;
                if basis_delta.abs() <= 0.0001 {
                    continue;
                }

                let mut percent_adjust = 0.0f32;
                for edge in [YG_EDGE_LEFT, YG_EDGE_RIGHT, YG_EDGE_START, YG_EDGE_END] {
                    if let Some(value) = n.position_edges.get_exact(edge) {
                        let yg_value = lpa_to_yg_value(value);
                        if yg_value.unit == YG_UNIT_PERCENT {
                            percent_adjust += basis_delta * (yg_value.value / 100.0);
                        }
                    }
                }
                if percent_adjust.abs() <= 0.0001 {
                    continue;
                }
                n.layout.unrounded_layout.location.x += percent_adjust;
                n.layout.final_layout.location.x += percent_adjust;
            }
        }

        for (node_ptr, _id) in mapping.iter().copied() {
            unsafe {
                let n = &mut *node_ptr;
                if n.style.position != Position::Absolute || n.owner.is_null() || n.display_mode == YG_DISPLAY_NONE {
                    continue;
                }

                let direct_owner = &*n.owner;
                if direct_owner.style.position == Position::Absolute || direct_owner.display_mode == YG_DISPLAY_NONE {
                    continue;
                }
                if direct_owner.owner.is_null() {
                    continue;
                }
                let owner_parent = &*direct_owner.owner;
                if owner_parent.display_mode == YG_DISPLAY_CONTENTS
                    || owner_parent.layout.direction != YG_DIRECTION_RTL
                    || !matches!(
                        owner_parent.style.flex_direction,
                        FlexDirection::Column | FlexDirection::ColumnReverse
                    )
                    || matches!(owner_parent.style.flex_wrap, FlexWrap::WrapReverse)
                {
                    continue;
                }

                let owner_start = direct_owner.position_edges.get_exact(YG_EDGE_START);
                let owner_end = direct_owner.position_edges.get_exact(YG_EDGE_END);
                let owner_left = direct_owner.position_edges.get_exact(YG_EDGE_LEFT);
                let owner_right = direct_owner.position_edges.get_exact(YG_EDGE_RIGHT);
                if owner_start.is_some() || owner_end.is_some() || owner_left.is_some() || owner_right.is_some() {
                    continue;
                }
                let owner_margin_start = direct_owner.margin_edges.get_exact(YG_EDGE_START);
                let owner_margin_end = direct_owner.margin_edges.get_exact(YG_EDGE_END);
                let owner_margin_left = direct_owner.margin_edges.get_exact(YG_EDGE_LEFT);
                let owner_margin_right = direct_owner.margin_edges.get_exact(YG_EDGE_RIGHT);
                if (owner_margin_start.is_some()
                    || owner_margin_end.is_some()
                    || owner_margin_left.is_some()
                    || owner_margin_right.is_some())
                    && owner_parent.position_type_mode != YG_POSITION_TYPE_STATIC
                {
                    continue;
                }

                let containing_ptr = absolute_containing_block(n.owner);
                if containing_ptr.is_null() || containing_ptr == n.owner {
                    continue;
                }
                let child_has_percent_border = [
                    YG_EDGE_LEFT,
                    YG_EDGE_RIGHT,
                    YG_EDGE_START,
                    YG_EDGE_END,
                    YG_EDGE_TOP,
                    YG_EDGE_BOTTOM,
                    YG_EDGE_HORIZONTAL,
                    YG_EDGE_VERTICAL,
                    YG_EDGE_ALL,
                ]
                .iter()
                .any(|edge| {
                    n.border_edges
                        .get_exact(*edge)
                        .map(|v| lp_to_yg_value(v).unit == YG_UNIT_PERCENT)
                        .unwrap_or(false)
                });
                if child_has_percent_border {
                    continue;
                }
                let pre_owner_global_x = pre_mirror_globals
                    .iter()
                    .find(|(p, _)| *p == n.owner)
                    .map(|(_, pos)| pos.0)
                    .unwrap_or_else(|| node_global_location(n.owner).0);
                let current_owner_global_x = node_global_location(n.owner).0;
                let owner_shift_x = pre_owner_global_x - current_owner_global_x;
                if owner_shift_x.abs() <= 0.0001 {
                    continue;
                }
                n.layout.unrounded_layout.location.x += owner_shift_x;
                n.layout.final_layout.location.x += owner_shift_x;
            }
        }
    }));
}

#[no_mangle]
pub extern "C" fn YGNodeGetHasNewLayout(node: *const YGNode) -> bool {
    unsafe { node.as_ref().map(|n| n.has_new_layout).unwrap_or(false) }
}

#[no_mangle]
pub extern "C" fn YGNodeSetHasNewLayout(node: *mut YGNode, has_new_layout: bool) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.has_new_layout = has_new_layout;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeIsDirty(node: *const YGNode) -> bool {
    unsafe { node.as_ref().map(|n| n.is_dirty).unwrap_or(false) }
}

#[no_mangle]
pub extern "C" fn YGNodeMarkDirty(node: *mut YGNode) {
    if !node.is_null() {
        mark_dirty(node);
    }
}

#[no_mangle]
pub extern "C" fn YGNodeSetDirtiedFunc(node: *mut YGNode, dirtied_func: Option<YGDirtiedFunc>) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.dirtied_func = dirtied_func;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeGetDirtiedFunc(node: *const YGNode) -> Option<YGDirtiedFunc> {
    unsafe { node.as_ref().and_then(|n| n.dirtied_func) }
}

#[no_mangle]
pub extern "C" fn YGNodeInsertChild(node: *mut YGNode, child: *mut YGNode, index: usize) {
    if node.is_null() || child.is_null() {
        return;
    }
    unsafe {
        if let Some(prev) = (*child).owner.as_mut() {
            let _ = prev;
            YGNodeRemoveChild((*child).owner, child);
        }
        (*child).owner = node;
        let idx = index.min((*node).children.len());
        (*node).children.insert(idx, child);
        mark_dirty(node);
    }
}

#[no_mangle]
pub extern "C" fn YGNodeSwapChild(node: *mut YGNode, child: *mut YGNode, index: usize) {
    if node.is_null() || child.is_null() {
        return;
    }
    unsafe {
        if index >= (*node).children.len() {
            return;
        }
        let old = (&(*node).children)[index];
        (*old).owner = ptr::null_mut();
        (*child).owner = node;
        (&mut (*node).children)[index] = child;
        mark_dirty(node);
    }
}

#[no_mangle]
pub extern "C" fn YGNodeRemoveChild(node: *mut YGNode, child: *mut YGNode) {
    if node.is_null() || child.is_null() {
        return;
    }
    unsafe {
        if let Some(idx) = (*node).children.iter().position(|n| *n == child) {
            (*node).children.remove(idx);
            (*child).owner = ptr::null_mut();
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeRemoveAllChildren(node: *mut YGNode) {
    if node.is_null() {
        return;
    }
    unsafe {
        for child in (*node).children.iter().copied() {
            (*child).owner = ptr::null_mut();
        }
        (*node).children.clear();
        mark_dirty(node);
    }
}

#[no_mangle]
pub extern "C" fn YGNodeSetChildren(owner: *mut YGNode, children: *const *mut YGNode, count: usize) {
    if owner.is_null() {
        return;
    }
    unsafe {
        YGNodeRemoveAllChildren(owner);
        let slice = std::slice::from_raw_parts(children, count);
        for (index, child) in slice.iter().copied().enumerate() {
            YGNodeInsertChild(owner, child, index);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeGetChild(node: *mut YGNode, index: usize) -> *mut YGNode {
    unsafe {
        node.as_ref()
            .and_then(|n| n.children.get(index).copied())
            .unwrap_or(ptr::null_mut())
    }
}

#[no_mangle]
pub extern "C" fn YGNodeGetChildCount(node: *const YGNode) -> usize {
    unsafe { node.as_ref().map(|n| n.children.len()).unwrap_or(0) }
}

#[no_mangle]
pub extern "C" fn YGNodeGetOwner(node: *mut YGNode) -> *mut YGNode {
    unsafe { node.as_ref().map(|n| n.owner).unwrap_or(ptr::null_mut()) }
}

#[no_mangle]
pub extern "C" fn YGNodeGetParent(node: *mut YGNode) -> *mut YGNode {
    YGNodeGetOwner(node)
}

#[no_mangle]
pub extern "C" fn YGNodeSetConfig(node: *mut YGNode, config: *mut YGConfig) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.config = if config.is_null() { config_default() } else { config };
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeGetConfig(node: *mut YGNode) -> *const YGConfig {
    unsafe { node.as_ref().map(|n| n.config as *const YGConfig).unwrap_or(ptr::null()) }
}

#[no_mangle]
pub extern "C" fn YGNodeSetContext(node: *mut YGNode, context: *mut c_void) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.context = context;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeGetContext(node: *const YGNode) -> *mut c_void {
    unsafe { node.as_ref().map(|n| n.context).unwrap_or(ptr::null_mut()) }
}

#[no_mangle]
pub extern "C" fn YGNodeSetMeasureFunc(node: *mut YGNode, measure_func: Option<YGMeasureFunc>) {
    unsafe {
        if let Some(n) = node.as_mut() {
            if measure_func.is_some() {
                n.node_type = YG_NODE_TYPE_TEXT;
            } else {
                n.node_type = YG_NODE_TYPE_DEFAULT;
            }
            n.measure_func = measure_func;
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeHasMeasureFunc(node: *const YGNode) -> bool {
    unsafe { node.as_ref().map(|n| n.measure_func.is_some()).unwrap_or(false) }
}

#[no_mangle]
pub extern "C" fn YGNodeSetBaselineFunc(node: *mut YGNode, baseline_func: Option<YGBaselineFunc>) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.baseline_func = baseline_func;
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeHasBaselineFunc(node: *const YGNode) -> bool {
    unsafe { node.as_ref().map(|n| n.baseline_func.is_some()).unwrap_or(false) }
}

#[no_mangle]
pub extern "C" fn YGNodeSetIsReferenceBaseline(node: *mut YGNode, is_reference_baseline: bool) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.is_reference_baseline = is_reference_baseline;
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeIsReferenceBaseline(node: *const YGNode) -> bool {
    unsafe {
        node.as_ref()
            .map(|n| n.is_reference_baseline)
            .unwrap_or(false)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeSetNodeType(node: *mut YGNode, node_type: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.node_type = node_type;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeGetNodeType(node: *const YGNode) -> i32 {
    unsafe { node.as_ref().map(|n| n.node_type).unwrap_or(YG_NODE_TYPE_DEFAULT) }
}

#[no_mangle]
pub extern "C" fn YGNodeSetAlwaysFormsContainingBlock(node: *mut YGNode, always_forms_containing_block: bool) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.always_forms_containing_block = always_forms_containing_block;
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeGetAlwaysFormsContainingBlock(node: *const YGNode) -> bool {
    unsafe {
        node.as_ref()
            .map(|n| n.always_forms_containing_block)
            .unwrap_or(false)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeCanUseCachedMeasurement(
    _width_mode: i32,
    _available_width: f32,
    _height_mode: i32,
    _available_height: f32,
    _last_width_mode: i32,
    _last_available_width: f32,
    _last_height_mode: i32,
    _last_available_height: f32,
    _last_computed_width: f32,
    _last_computed_height: f32,
    _margin_row: f32,
    _margin_column: f32,
    _config: *mut YGConfig,
) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn YGNodeCopyStyle(dst_node: *mut YGNode, src_node: *const YGNode) {
    unsafe {
        if let (Some(dst), Some(src)) = (dst_node.as_mut(), src_node.as_ref()) {
            dst.style = src.style.clone();
            dst.display_mode = src.display_mode;
            dst.position_type_mode = src.position_type_mode;
            dst.flex = src.flex;
            dst.has_flex_grow = src.has_flex_grow;
            dst.has_flex_shrink = src.has_flex_shrink;
            dst.direction = src.direction;
            dst.position_edges = src.position_edges.clone();
            dst.margin_edges = src.margin_edges.clone();
            dst.padding_edges = src.padding_edges.clone();
            dst.border_edges = src.border_edges.clone();
            mark_dirty(dst_node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetDirection(node: *mut YGNode, direction: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.direction = direction;
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetDirection(node: *const YGNode) -> i32 {
    unsafe { node.as_ref().map(|n| n.direction).unwrap_or(YG_DIRECTION_INHERIT) }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexDirection(node: *mut YGNode, flex_direction: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_direction = map_flex_direction(flex_direction);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetFlexDirection(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_flex_direction(n.style.flex_direction))
            .unwrap_or(YG_FLEX_DIRECTION_ROW)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetJustifyContent(node: *mut YGNode, justify_content: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.justify_content = map_justify(justify_content);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetJustifyContent(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_justify(n.style.justify_content, false))
            .unwrap_or(YG_JUSTIFY_FLEX_START)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetJustifyItems(node: *mut YGNode, justify_items: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.justify_items = map_align(justify_items);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetJustifyItems(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_align(n.style.justify_items, true))
            .unwrap_or(YG_ALIGN_AUTO)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetJustifySelf(node: *mut YGNode, justify_self: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.justify_self = map_align(justify_self);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetJustifySelf(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_align(n.style.justify_self, true))
            .unwrap_or(YG_ALIGN_AUTO)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetAlignContent(node: *mut YGNode, align_content: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.align_content = map_align_content(align_content);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetAlignContent(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_align_content(n.style.align_content, false))
            .unwrap_or(YG_ALIGN_FLEX_START)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetAlignItems(node: *mut YGNode, align_items: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.align_items = map_align(align_items);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetAlignItems(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_align(n.style.align_items, false))
            .unwrap_or(YG_ALIGN_STRETCH)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetAlignSelf(node: *mut YGNode, align_self: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.align_self = map_align(align_self);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetAlignSelf(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_align(n.style.align_self, true))
            .unwrap_or(YG_ALIGN_AUTO)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetPositionType(node: *mut YGNode, position_type: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.position_type_mode = position_type;
            n.style.position = map_position_type(position_type);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetPositionType(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| n.position_type_mode)
            .unwrap_or(YG_POSITION_TYPE_RELATIVE)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexWrap(node: *mut YGNode, flex_wrap: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_wrap = map_wrap(flex_wrap);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetFlexWrap(node: *const YGNode) -> i32 {
    unsafe { node.as_ref().map(|n| unmap_wrap(n.style.flex_wrap)).unwrap_or(YG_WRAP_NO_WRAP) }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetOverflow(node: *mut YGNode, overflow: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            let value = map_overflow(overflow);
            n.style.overflow.x = value;
            n.style.overflow.y = value;
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetOverflow(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_overflow(n.style.overflow.x))
            .unwrap_or(YG_OVERFLOW_VISIBLE)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetDisplay(node: *mut YGNode, display: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.display_mode = display;
            n.style.display = map_display(display);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetDisplay(node: *const YGNode) -> i32 {
    unsafe { node.as_ref().map(|n| n.display_mode).unwrap_or(YG_DISPLAY_FLEX) }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlex(node: *mut YGNode, flex: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.flex = Some(flex);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetFlex(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .and_then(|n| n.flex)
            .unwrap_or(f32::NAN)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexGrow(node: *mut YGNode, flex_grow: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_grow = flex_grow;
            n.has_flex_grow = true;
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetFlexGrow(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref().map(|n| {
            if n.has_flex_grow {
                n.style.flex_grow
            } else {
                0.0
            }
        }).unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexShrink(node: *mut YGNode, flex_shrink: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_shrink = flex_shrink;
            n.has_flex_shrink = true;
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetFlexShrink(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref().map(|n| {
            if n.has_flex_shrink {
                n.style.flex_shrink
            } else if !n.config.is_null() && (*n.config).use_web_defaults {
                1.0
            } else {
                0.0
            }
        }).unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexBasis(node: *mut YGNode, flex_basis: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_basis = Dimension::length(flex_basis);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexBasisPercent(node: *mut YGNode, flex_basis: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_basis = Dimension::percent(flex_basis / 100.0);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexBasisAuto(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_basis = Dimension::auto();
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexBasisMaxContent(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_basis = dim_max_content();
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexBasisFitContent(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_basis = dim_fit_content();
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetFlexBasisStretch(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.flex_basis = dim_stretch();
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetFlexBasis(node: *const YGNode) -> YGValue {
    unsafe { node.as_ref().map(|n| dim_to_yg_value(n.style.flex_basis)).unwrap_or(YGValueUndefined) }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetPosition(node: *mut YGNode, edge: i32, position: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.position_edges.set(edge, LengthPercentageAuto::length(position));
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetPositionPercent(node: *mut YGNode, edge: i32, position: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.position_edges.set(edge, LengthPercentageAuto::percent(position / 100.0));
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetPositionAuto(node: *mut YGNode, edge: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.position_edges.set(edge, LengthPercentageAuto::auto());
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetPosition(node: *const YGNode, edge: i32) -> YGValue {
    unsafe {
        node.as_ref()
            .map(|n| n.position_edges.get_exact(edge).map(lpa_to_yg_value).unwrap_or(YGValueUndefined))
            .unwrap_or(YGValueUndefined)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetMargin(node: *mut YGNode, edge: i32, margin: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.margin_edges.set(edge, LengthPercentageAuto::length(margin));
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetMarginPercent(node: *mut YGNode, edge: i32, margin: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.margin_edges.set(edge, LengthPercentageAuto::percent(margin / 100.0));
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetMarginAuto(node: *mut YGNode, edge: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.margin_edges.set(edge, LengthPercentageAuto::auto());
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetMargin(node: *const YGNode, edge: i32) -> YGValue {
    unsafe {
        node.as_ref()
            .map(|n| n.margin_edges.get_exact(edge).map(lpa_to_yg_value).unwrap_or(YGValueUndefined))
            .unwrap_or(YGValueUndefined)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetPadding(node: *mut YGNode, edge: i32, padding: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.padding_edges.set(edge, LengthPercentage::length(padding));
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetPaddingPercent(node: *mut YGNode, edge: i32, padding: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.padding_edges.set(edge, LengthPercentage::percent(padding / 100.0));
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetPadding(node: *const YGNode, edge: i32) -> YGValue {
    unsafe {
        node.as_ref()
            .map(|n| n.padding_edges.get_exact(edge).map(lp_to_yg_value).unwrap_or(YGValueUndefined))
            .unwrap_or(YGValueUndefined)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetBorder(node: *mut YGNode, edge: i32, border: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.border_edges.set(edge, LengthPercentage::length(border));
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetBorder(node: *const YGNode, edge: i32) -> f32 {
    unsafe {
        node.as_ref()
            .map(|n| n.border_edges.get_exact(edge).map(|v| v.into_raw().value()).unwrap_or(f32::NAN))
            .unwrap_or(f32::NAN)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetGap(node: *mut YGNode, gutter: i32, gap_length: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            let value = LengthPercentage::length(gap_length);
            match gutter {
                YG_GUTTER_COLUMN => n.style.gap.width = value,
                YG_GUTTER_ROW => n.style.gap.height = value,
                _ => {
                    n.style.gap.width = value;
                    n.style.gap.height = value;
                }
            }
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetGapPercent(node: *mut YGNode, gutter: i32, gap_length: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            let value = LengthPercentage::percent(gap_length / 100.0);
            match gutter {
                YG_GUTTER_COLUMN => n.style.gap.width = value,
                YG_GUTTER_ROW => n.style.gap.height = value,
                _ => {
                    n.style.gap.width = value;
                    n.style.gap.height = value;
                }
            }
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetGap(node: *const YGNode, gutter: i32) -> YGValue {
    unsafe {
        node.as_ref()
            .map(|n| {
                let value = match gutter {
                    YG_GUTTER_COLUMN => n.style.gap.width,
                    _ => n.style.gap.height,
                };
                lp_to_yg_value(value)
            })
            .unwrap_or(YGValueUndefined)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetBoxSizing(node: *mut YGNode, box_sizing: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.box_sizing = map_box_sizing(box_sizing);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetBoxSizing(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| unmap_box_sizing(n.style.box_sizing))
            .unwrap_or(YG_BOX_SIZING_BORDER_BOX)
    }
}

macro_rules! define_dimension_setters {
    ($set:ident, $set_pct:ident, $set_auto:ident, $set_max_content:ident, $set_fit_content:ident, $set_stretch:ident, $get:ident, $($field:tt)+) => {
        #[no_mangle]
        pub extern "C" fn $set(node: *mut YGNode, value: f32) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = Dimension::length(value);
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_pct(node: *mut YGNode, value: f32) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = Dimension::percent(value / 100.0);
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_auto(node: *mut YGNode) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = Dimension::auto();
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_max_content(node: *mut YGNode) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = dim_max_content();
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_fit_content(node: *mut YGNode) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = dim_fit_content();
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_stretch(node: *mut YGNode) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = dim_stretch();
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $get(node: *const YGNode) -> YGValue {
            unsafe {
                node.as_ref()
                    .map(|n| dim_to_yg_value(n.style.$($field)+))
                    .unwrap_or(YGValueUndefined)
            }
        }
    };
}

macro_rules! define_dimension_setters_no_auto {
    ($set:ident, $set_pct:ident, $set_max_content:ident, $set_fit_content:ident, $set_stretch:ident, $get:ident, $($field:tt)+) => {
        #[no_mangle]
        pub extern "C" fn $set(node: *mut YGNode, value: f32) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = Dimension::length(value);
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_pct(node: *mut YGNode, value: f32) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = Dimension::percent(value / 100.0);
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_max_content(node: *mut YGNode) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = dim_max_content();
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_fit_content(node: *mut YGNode) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = dim_fit_content();
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $set_stretch(node: *mut YGNode) {
            unsafe {
                if let Some(n) = node.as_mut() {
                    n.style.$($field)+ = dim_stretch();
                    mark_dirty(node);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn $get(node: *const YGNode) -> YGValue {
            unsafe {
                node.as_ref()
                    .map(|n| dim_to_yg_value(n.style.$($field)+))
                    .unwrap_or(YGValueUndefined)
            }
        }
    };
}

define_dimension_setters!(
    YGNodeStyleSetWidth,
    YGNodeStyleSetWidthPercent,
    YGNodeStyleSetWidthAuto,
    YGNodeStyleSetWidthMaxContent,
    YGNodeStyleSetWidthFitContent,
    YGNodeStyleSetWidthStretch,
    YGNodeStyleGetWidth,
    size.width
);

define_dimension_setters!(
    YGNodeStyleSetHeight,
    YGNodeStyleSetHeightPercent,
    YGNodeStyleSetHeightAuto,
    YGNodeStyleSetHeightMaxContent,
    YGNodeStyleSetHeightFitContent,
    YGNodeStyleSetHeightStretch,
    YGNodeStyleGetHeight,
    size.height
);

define_dimension_setters_no_auto!(
    YGNodeStyleSetMinWidth,
    YGNodeStyleSetMinWidthPercent,
    YGNodeStyleSetMinWidthMaxContent,
    YGNodeStyleSetMinWidthFitContent,
    YGNodeStyleSetMinWidthStretch,
    YGNodeStyleGetMinWidth,
    min_size.width
);

define_dimension_setters_no_auto!(
    YGNodeStyleSetMinHeight,
    YGNodeStyleSetMinHeightPercent,
    YGNodeStyleSetMinHeightMaxContent,
    YGNodeStyleSetMinHeightFitContent,
    YGNodeStyleSetMinHeightStretch,
    YGNodeStyleGetMinHeight,
    min_size.height
);

define_dimension_setters_no_auto!(
    YGNodeStyleSetMaxWidth,
    YGNodeStyleSetMaxWidthPercent,
    YGNodeStyleSetMaxWidthMaxContent,
    YGNodeStyleSetMaxWidthFitContent,
    YGNodeStyleSetMaxWidthStretch,
    YGNodeStyleGetMaxWidth,
    max_size.width
);

define_dimension_setters_no_auto!(
    YGNodeStyleSetMaxHeight,
    YGNodeStyleSetMaxHeightPercent,
    YGNodeStyleSetMaxHeightMaxContent,
    YGNodeStyleSetMaxHeightFitContent,
    YGNodeStyleSetMaxHeightStretch,
    YGNodeStyleGetMaxHeight,
    max_size.height
);

#[no_mangle]
pub extern "C" fn YGNodeStyleSetAspectRatio(node: *mut YGNode, aspect_ratio: f32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.aspect_ratio = if aspect_ratio.is_nan() { None } else { Some(aspect_ratio) };
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleGetAspectRatio(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .and_then(|n| n.style.aspect_ratio)
            .unwrap_or(f32::NAN)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridColumnStart(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_column.start = GridPlacement::from_line_index(value as i16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridColumnStartAuto(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_column.start = GridPlacement::Auto;
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridColumnStartSpan(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_column.start = GridPlacement::from_span(value as u16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleGetGridColumnStart(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| match n.style.grid_column.start {
                GridPlacement::Line(v) => v.as_i16() as i32,
                _ => 0,
            })
            .unwrap_or(0)
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridColumnEnd(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_column.end = GridPlacement::from_line_index(value as i16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridColumnEndAuto(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_column.end = GridPlacement::Auto;
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridColumnEndSpan(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_column.end = GridPlacement::from_span(value as u16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleGetGridColumnEnd(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| match n.style.grid_column.end {
                GridPlacement::Line(v) => v.as_i16() as i32,
                _ => 0,
            })
            .unwrap_or(0)
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridRowStart(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_row.start = GridPlacement::from_line_index(value as i16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridRowStartAuto(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_row.start = GridPlacement::Auto;
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridRowStartSpan(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_row.start = GridPlacement::from_span(value as u16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleGetGridRowStart(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| match n.style.grid_row.start {
                GridPlacement::Line(v) => v.as_i16() as i32,
                _ => 0,
            })
            .unwrap_or(0)
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridRowEnd(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_row.end = GridPlacement::from_line_index(value as i16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridRowEndAuto(node: *mut YGNode) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_row.end = GridPlacement::Auto;
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridRowEndSpan(node: *mut YGNode, value: i32) {
    unsafe {
        if let Some(n) = node.as_mut() {
            n.style.grid_row.end = GridPlacement::from_span(value as u16);
            mark_dirty(node);
        }
    }
}
#[no_mangle]
pub extern "C" fn YGNodeStyleGetGridRowEnd(node: *const YGNode) -> i32 {
    unsafe {
        node.as_ref()
            .map(|n| match n.style.grid_row.end {
                GridPlacement::Line(v) => v.as_i16() as i32,
                _ => 0,
            })
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn YGGridTrackListCreate() -> *mut YGGridTrackList {
    Box::into_raw(Box::new(YGGridTrackList { tracks: Vec::new() }))
}

#[no_mangle]
pub extern "C" fn YGGridTrackListFree(list: *mut YGGridTrackList) {
    if !list.is_null() {
        unsafe {
            drop(Box::from_raw(list));
        }
    }
}

#[no_mangle]
pub extern "C" fn YGGridTrackListAddTrack(list: *mut YGGridTrackList, track_value: *mut YGGridTrackValue) {
    unsafe {
        if let (Some(list), Some(track)) = (list.as_mut(), track_value.as_ref()) {
            let tsf = match track {
                YGGridTrackValue::Single(v) => *v,
                YGGridTrackValue::MinMax(min, max) => minmax(*min, *max),
            };
            list.tracks.push(tsf);
            drop(Box::from_raw(track_value));
        }
    }
}

#[no_mangle]
pub extern "C" fn YGPoints(points: f32) -> *mut YGGridTrackValue {
    Box::into_raw(Box::new(YGGridTrackValue::Single(minmax(
        MinTrackSizingFunction::length(points),
        MaxTrackSizingFunction::length(points),
    ))))
}

#[no_mangle]
pub extern "C" fn YGPercent(percent: f32) -> *mut YGGridTrackValue {
    Box::into_raw(Box::new(YGGridTrackValue::Single(minmax(
        MinTrackSizingFunction::percent(percent / 100.0),
        MaxTrackSizingFunction::percent(percent / 100.0),
    ))))
}

#[no_mangle]
pub extern "C" fn YGFr(fr: f32) -> *mut YGGridTrackValue {
    Box::into_raw(Box::new(YGGridTrackValue::Single(minmax(
        MinTrackSizingFunction::auto(),
        MaxTrackSizingFunction::fr(fr),
    ))))
}

#[no_mangle]
pub extern "C" fn YGAuto() -> *mut YGGridTrackValue {
    Box::into_raw(Box::new(YGGridTrackValue::Single(minmax(
        MinTrackSizingFunction::auto(),
        MaxTrackSizingFunction::auto(),
    ))))
}

#[no_mangle]
pub extern "C" fn YGMinMax(
    min: *mut YGGridTrackValue,
    max: *mut YGGridTrackValue,
) -> *mut YGGridTrackValue {
    unsafe {
        let min_value = match min.as_ref() {
            Some(YGGridTrackValue::Single(v)) => v.min_sizing_function(),
            Some(YGGridTrackValue::MinMax(min, _)) => *min,
            None => MinTrackSizingFunction::auto(),
        };
        let max_value = match max.as_ref() {
            Some(YGGridTrackValue::Single(v)) => v.max_sizing_function(),
            Some(YGGridTrackValue::MinMax(_, max)) => *max,
            None => MaxTrackSizingFunction::auto(),
        };
        if !min.is_null() {
            drop(Box::from_raw(min));
        }
        if !max.is_null() {
            drop(Box::from_raw(max));
        }
        Box::into_raw(Box::new(YGGridTrackValue::MinMax(min_value, max_value)))
    }
}

fn set_grid_track_list<S: taffy::style::CheapCloneStr>(
    target: &mut Vec<GridTemplateComponent<S>>,
    tracks: &[TrackSizingFunction],
) {
    target.clear();
    target.extend(tracks.iter().copied().map(GridTemplateComponent::Single));
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridTemplateRows(node: *mut YGNode, track_list: *mut YGGridTrackList) {
    unsafe {
        if let (Some(node), Some(list)) = (node.as_mut(), track_list.as_ref()) {
            set_grid_track_list(&mut node.style.grid_template_rows, &list.tracks);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridTemplateColumns(node: *mut YGNode, track_list: *mut YGGridTrackList) {
    unsafe {
        if let (Some(node), Some(list)) = (node.as_mut(), track_list.as_ref()) {
            set_grid_track_list(&mut node.style.grid_template_columns, &list.tracks);
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridAutoRows(node: *mut YGNode, track_list: *mut YGGridTrackList) {
    unsafe {
        if let (Some(node), Some(list)) = (node.as_mut(), track_list.as_ref()) {
            node.style.grid_auto_rows = list.tracks.clone();
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeStyleSetGridAutoColumns(node: *mut YGNode, track_list: *mut YGGridTrackList) {
    unsafe {
        if let (Some(node), Some(list)) = (node.as_mut(), track_list.as_ref()) {
            node.style.grid_auto_columns = list.tracks.clone();
            mark_dirty(node);
        }
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetLeft(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .map(|n| {
                let psf = if n.config.is_null() { 1.0 } else { (*n.config).point_scale_factor as f64 };
                round_value_to_pixel_grid(n.layout.unrounded_layout.location.x as f64, psf, false, false)
            })
            .unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetTop(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .map(|n| {
                let psf = if n.config.is_null() { 1.0 } else { (*n.config).point_scale_factor as f64 };
                round_value_to_pixel_grid(n.layout.unrounded_layout.location.y as f64, psf, false, false)
            })
            .unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetRight(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .map(|n| n.layout.final_layout.location.x + n.layout.final_layout.size.width)
            .unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetBottom(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .map(|n| n.layout.final_layout.location.y + n.layout.final_layout.size.height)
            .unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetWidth(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .map(|n| {
                let psf = if n.config.is_null() { 1.0 } else { (*n.config).point_scale_factor as f64 };
                round_value_to_pixel_grid(n.layout.unrounded_layout.size.width as f64, psf, false, false)
            })
            .unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetHeight(node: *const YGNode) -> f32 {
    unsafe {
        node.as_ref()
            .map(|n| {
                let psf = if n.config.is_null() { 1.0 } else { (*n.config).point_scale_factor as f64 };
                round_value_to_pixel_grid(n.layout.unrounded_layout.size.height as f64, psf, false, false)
            })
            .unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetDirection(node: *const YGNode) -> i32 {
    unsafe { node.as_ref().map(|n| n.layout.direction).unwrap_or(YG_DIRECTION_LTR) }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetHadOverflow(node: *const YGNode) -> bool {
    unsafe {
        node.as_ref()
            .map(|n| n.layout.had_overflow)
            .unwrap_or(false)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetMargin(node: *const YGNode, edge: i32) -> f32 {
    unsafe {
        node.as_ref().map(|n| {
            match edge {
                YG_EDGE_LEFT => n.layout.final_layout.margin.left,
                YG_EDGE_TOP => n.layout.final_layout.margin.top,
                YG_EDGE_RIGHT => n.layout.final_layout.margin.right,
                YG_EDGE_BOTTOM => n.layout.final_layout.margin.bottom,
                YG_EDGE_START => {
                    if n.layout.direction == YG_DIRECTION_RTL {
                        n.layout.final_layout.margin.right
                    } else {
                        n.layout.final_layout.margin.left
                    }
                }
                YG_EDGE_END => {
                    if n.layout.direction == YG_DIRECTION_RTL {
                        n.layout.final_layout.margin.left
                    } else {
                        n.layout.final_layout.margin.right
                    }
                }
                _ => 0.0,
            }
        }).unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetBorder(node: *const YGNode, edge: i32) -> f32 {
    unsafe {
        node.as_ref().map(|n| {
            match edge {
                YG_EDGE_LEFT => n.layout.final_layout.border.left,
                YG_EDGE_TOP => n.layout.final_layout.border.top,
                YG_EDGE_RIGHT => n.layout.final_layout.border.right,
                YG_EDGE_BOTTOM => n.layout.final_layout.border.bottom,
                YG_EDGE_START => {
                    if n.layout.direction == YG_DIRECTION_RTL {
                        n.layout.final_layout.border.right
                    } else {
                        n.layout.final_layout.border.left
                    }
                }
                YG_EDGE_END => {
                    if n.layout.direction == YG_DIRECTION_RTL {
                        n.layout.final_layout.border.left
                    } else {
                        n.layout.final_layout.border.right
                    }
                }
                _ => 0.0,
            }
        }).unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetPadding(node: *const YGNode, edge: i32) -> f32 {
    unsafe {
        node.as_ref().map(|n| {
            match edge {
                YG_EDGE_LEFT => n.layout.final_layout.padding.left,
                YG_EDGE_TOP => n.layout.final_layout.padding.top,
                YG_EDGE_RIGHT => n.layout.final_layout.padding.right,
                YG_EDGE_BOTTOM => n.layout.final_layout.padding.bottom,
                YG_EDGE_START => {
                    if n.layout.direction == YG_DIRECTION_RTL {
                        n.layout.final_layout.padding.right
                    } else {
                        n.layout.final_layout.padding.left
                    }
                }
                YG_EDGE_END => {
                    if n.layout.direction == YG_DIRECTION_RTL {
                        n.layout.final_layout.padding.left
                    } else {
                        n.layout.final_layout.padding.right
                    }
                }
                _ => 0.0,
            }
        }).unwrap_or(0.0)
    }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetRawHeight(node: *const YGNode) -> f32 {
    unsafe { node.as_ref().map(|n| n.layout.unrounded_layout.size.height).unwrap_or(0.0) }
}

#[no_mangle]
pub extern "C" fn YGNodeLayoutGetRawWidth(node: *const YGNode) -> f32 {
    unsafe { node.as_ref().map(|n| n.layout.unrounded_layout.size.width).unwrap_or(0.0) }
}
