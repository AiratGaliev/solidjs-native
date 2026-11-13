//! Taffy-backed layout computed exclusively from committed sn_core::UiTree state.
//!
//! Taffy remains an implementation detail of this crate. The public API exposes
//! logical-unit geometry, revision metadata and an intrinsic text measurement
//! contract; it does not expose Taffy node/style/cache types.

use std::collections::{BTreeMap, BTreeSet};

use sn_core::{
    Node, NodeId, NodeKind, PropertyValue, SessionGeneration, SurfaceId, TreeError, UiTree,
};
use taffy::Direction;
use taffy::prelude::{
    AlignContent, AlignItems, AvailableSpace, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, Layout, LengthPercentage, LengthPercentageAuto, NodeId as TaffyNodeId,
    Position, Rect, Size, Style,
};
use taffy::tree::LayoutOutput;
use taffy::{
    Cache, CacheTree, LayoutBlockContainer, LayoutContainingBlock, LayoutFlexboxContainer,
    LayoutGridContainer, LayoutPartialTree, TraversePartialTree, compute_block_layout,
    compute_cached_layout, compute_flexbox_layout, compute_grid_layout, compute_hidden_layout,
    compute_leaf_layout, compute_oof_layout, compute_root_layout,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Width in logical points / dp-like units.
    pub width: f32,
    /// Height in logical points / dp-like units.
    pub height: f32,
}

impl Viewport {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    fn validate(self) -> Result<Self, LayoutError> {
        if !self.width.is_finite()
            || !self.height.is_finite()
            || self.width < 0.0
            || self.height < 0.0
        {
            return Err(LayoutError::InvalidViewport);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutRect {
    /// Local x coordinate in the containing block's coordinate space, in logical units.
    pub x: f32,
    /// Local y coordinate in the containing block's coordinate space, in logical units.
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntrinsicSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntrinsicAvailableSpace {
    Definite(f32),
    MinContent,
    MaxContent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntrinsicConstraints {
    pub known_width: Option<f32>,
    pub known_height: Option<f32>,
    pub available_width: IntrinsicAvailableSpace,
    pub available_height: IntrinsicAvailableSpace,
}

/// Platform text measurement contract. Returned dimensions use logical units.
/// Increment revision whenever font metrics, shaping configuration or another
/// global intrinsic-measurement input changes.
pub trait TextMeasurer {
    fn measure(
        &self,
        node_id: NodeId,
        text: &str,
        constraints: IntrinsicConstraints,
    ) -> IntrinsicSize;

    fn revision(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStats {
    /// Nodes whose cached Taffy measurements were cleared during this commit.
    /// Ancestors are included because their results depend on dirty descendants.
    pub invalidated_nodes: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSnapshot {
    pub tree_revision: u64,
    pub root: NodeId,
    pub viewport: Viewport,
    pub nodes: BTreeMap<NodeId, LayoutRect>,
    pub stats: LayoutStats,
}

impl LayoutSnapshot {
    pub fn node(&self, id: NodeId) -> Option<LayoutRect> {
        self.nodes.get(&id).copied()
    }
}

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error(transparent)]
    Tree(#[from] TreeError),
    #[error("the layout root must be a committed UiTree root")]
    InvalidRoot,
    #[error("the layout root must be an element node")]
    TextRoot,
    #[error("viewport dimensions must be finite and non-negative")]
    InvalidViewport,
    #[error("tree revision moved backwards within the same Surface session")]
    StaleRevision,
    #[error("unsupported position value {value:?} on node {node:?}")]
    UnsupportedPosition { node: NodeId, value: String },
    #[error("invalid layout property {property:?} with value {value:?} on node {node:?}")]
    InvalidStyle {
        node: NodeId,
        property: String,
        value: String,
    },
    #[error("intrinsic text measurer returned a non-finite or negative size for node {0:?}")]
    InvalidIntrinsicMeasurement(NodeId),
}

/// Stateful layout facade. The sidecar owns Taffy caches and computed geometry;
/// the committed UiTree remains authoritative for node identity and hierarchy.
#[derive(Default)]
pub struct LayoutEngine {
    identity: Option<(SurfaceId, SessionGeneration)>,
    root: Option<NodeId>,
    last_tree_revision: Option<u64>,
    last_measure_revision: Option<u64>,
    states: BTreeMap<NodeId, NodeState>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute layout for one committed tree root.
    ///
    /// Call after UiTree::apply_batch succeeds. The shared tree reference means
    /// this subsystem cannot observe or mutate an uncommitted staging tree.
    pub fn compute(
        &mut self,
        tree: &UiTree,
        root: NodeId,
        viewport: Viewport,
        text_measurer: &dyn TextMeasurer,
    ) -> Result<LayoutSnapshot, LayoutError> {
        let viewport = viewport.validate()?;
        if !tree.roots().contains(&root) {
            return Err(LayoutError::InvalidRoot);
        }
        if matches!(&tree.node(root)?.kind, NodeKind::Text) {
            return Err(LayoutError::TextRoot);
        }

        let identity = (tree.surface_id(), tree.session_generation());
        if self.identity != Some(identity) || self.root != Some(root) {
            self.states.clear();
            self.last_tree_revision = None;
            self.last_measure_revision = None;
            self.identity = Some(identity);
            self.root = Some(root);
        }

        if self
            .last_tree_revision
            .is_some_and(|previous| tree.revision() < previous)
        {
            return Err(LayoutError::StaleRevision);
        }

        // Derive inputs before changing cached state. Invalid input cannot
        // partially replace the previous sidecar.
        let specs = collect_specs(tree, root)?;
        let measure_revision = text_measurer.revision();
        let invalidated = self.synchronize(specs, measure_revision);

        let mut workspace = LayoutWorkspace {
            nodes: std::mem::take(&mut self.states),
            text_measurer,
            measurement_errors: BTreeSet::new(),
        };

        compute_root_layout(
            &mut workspace,
            TaffyNodeId::new(root.get()),
            Size {
                width: AvailableSpace::Definite(viewport.width),
                height: AvailableSpace::Definite(viewport.height),
            },
        );

        let measurement_error = workspace.measurement_errors.iter().next().copied();
        // A zero fallback is used only to let Taffy's algorithm unwind safely.
        // Never cache a failed measurement for any text node or its ancestors.
        for invalid_node in workspace.measurement_errors.iter().copied() {
            let mut current = Some(invalid_node);
            while let Some(id) = current {
                let parent = workspace.nodes.get(&id).and_then(|state| state.parent);
                if let Some(state) = workspace.nodes.get_mut(&id) {
                    state.cache.clear();
                }
                current = parent;
            }
        }
        let nodes = workspace
            .nodes
            .iter()
            .map(|(id, state)| {
                (
                    *id,
                    LayoutRect {
                        x: state.layout.location.x,
                        y: state.layout.location.y,
                        width: state.layout.size.width,
                        height: state.layout.size.height,
                    },
                )
            })
            .collect();

        self.states = workspace.nodes;
        self.last_tree_revision = Some(tree.revision());
        self.last_measure_revision = Some(measure_revision);

        if let Some(node_id) = measurement_error {
            return Err(LayoutError::InvalidIntrinsicMeasurement(node_id));
        }

        Ok(LayoutSnapshot {
            tree_revision: tree.revision(),
            root,
            viewport,
            nodes,
            stats: LayoutStats {
                invalidated_nodes: invalidated,
            },
        })
    }

    fn synchronize(
        &mut self,
        specs: BTreeMap<NodeId, NodeSpec>,
        measure_revision: u64,
    ) -> Vec<NodeId> {
        let measure_revision_changed = self
            .last_measure_revision
            .is_some_and(|previous| previous != measure_revision);

        let mut next_states = BTreeMap::new();
        let mut dirty = BTreeSet::new();

        for (id, spec) in specs {
            let mut previous = self.states.remove(&id);
            let changed = previous.as_ref().is_none_or(|old| {
                old.style != spec.style
                    || old.parent != spec.parent
                    || old.children != spec.children
                    || old.text != spec.text
                    || old.is_text != spec.is_text
            });

            let mut cache = previous
                .as_mut()
                .map(|old| std::mem::replace(&mut old.cache, Cache::new()))
                .unwrap_or_default();

            if changed || (measure_revision_changed && spec.is_text) {
                cache.clear();
                dirty.insert(id);
            }

            let (layout, hoisted_children) = previous
                .map(|old| (old.layout, old.hoisted_children))
                .unwrap_or_else(|| (Layout::with_order(0), Vec::new()));

            next_states.insert(
                id,
                NodeState {
                    style: spec.style,
                    parent: spec.parent,
                    children: spec.children,
                    text: spec.text,
                    is_text: spec.is_text,
                    cache,
                    layout,
                    hoisted_children,
                },
            );
        }

        // A removed node invalidates its former parent. Parent child-list
        // comparison catches insertions and reparenting.
        for (_removed_id, old) in std::mem::take(&mut self.states) {
            if let Some(parent) = old.parent {
                dirty.insert(parent);
            }
        }

        // Dirty state propagates toward the root because each ancestor's
        // measured size/placement may depend on a changed descendant.
        let mut pending: Vec<NodeId> = dirty.iter().copied().collect();
        while let Some(id) = pending.pop() {
            if let Some(parent) = next_states.get(&id).and_then(|state| state.parent) {
                if dirty.insert(parent) {
                    pending.push(parent);
                }
            }
        }

        for id in &dirty {
            if let Some(state) = next_states.get_mut(id) {
                state.cache.clear();
            }
        }

        self.states = next_states;
        dirty
            .into_iter()
            .filter(|id| self.states.contains_key(id))
            .collect()
    }
}

struct NodeSpec {
    style: Style,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    text: Option<String>,
    is_text: bool,
}

struct NodeState {
    style: Style,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    text: Option<String>,
    is_text: bool,
    cache: Cache,
    layout: Layout,
    hoisted_children: Vec<TaffyNodeId>,
}

fn collect_specs(tree: &UiTree, root: NodeId) -> Result<BTreeMap<NodeId, NodeSpec>, LayoutError> {
    let mut specs = BTreeMap::new();
    let mut pending = vec![root];

    while let Some(id) = pending.pop() {
        if specs.contains_key(&id) {
            continue;
        }

        let node = tree.node(id)?;
        let is_text = matches!(&node.kind, NodeKind::Text);
        let style = if is_text {
            Style::default()
        } else {
            style_from_node(node)?
        };

        specs.insert(
            id,
            NodeSpec {
                style,
                parent: node.parent,
                children: node.children.clone(),
                text: if is_text { node.text.clone() } else { None },
                is_text,
            },
        );

        pending.extend(node.children.iter().rev().copied());
    }

    Ok(specs)
}

fn style_from_node(node: &Node) -> Result<Style, LayoutError> {
    // The host defaults to CSS-like block flow; Taffy's default is flex.
    let mut style = Style {
        display: Display::Block,
        ..Style::default()
    };

    if let Some(value) = property(node, &["display"]) {
        style.display = match string_value(value) {
            Some("block") => Display::Block,
            Some("flow-root") => Display::FlowRoot,
            Some("flex") => Display::Flex,
            Some("grid") => Display::Grid,
            Some("none") => Display::None,
            _ => return Err(invalid_style(node, "display", value)),
        };
    }

    if let Some(value) = property(node, &["position"]) {
        let raw = string_value(value).unwrap_or_default();
        style.position = match raw {
            "static" => Position::Static,
            "relative" => Position::Relative,
            "absolute" => Position::Absolute,
            "fixed" => Position::Fixed,
            "sticky" => {
                return Err(LayoutError::UnsupportedPosition {
                    node: node.id,
                    value: raw.to_owned(),
                });
            }
            _ => return Err(invalid_style(node, "position", value)),
        };
    }

    if let Some(value) = property(node, &["boxSizing", "box-sizing"]) {
        style.box_sizing = match string_value(value) {
            Some("border-box") => taffy::prelude::BoxSizing::BorderBox,
            Some("content-box") => taffy::prelude::BoxSizing::ContentBox,
            _ => return Err(invalid_style(node, "box-sizing", value)),
        };
    }

    if let Some(value) = property(node, &["direction"]) {
        style.direction = match string_value(value) {
            Some("ltr") => Direction::Ltr,
            Some("rtl") => Direction::Rtl,
            _ => return Err(invalid_style(node, "direction", value)),
        };
    }

    if let Some(value) = property(node, &["width"]) {
        style.size.width = parse_dimension(node, "width", value)?;
    }
    if let Some(value) = property(node, &["height"]) {
        style.size.height = parse_dimension(node, "height", value)?;
    }
    if let Some(value) = property(node, &["minWidth", "min-width"]) {
        style.min_size.width = parse_length_auto(node, "min-width", value)?;
    }
    if let Some(value) = property(node, &["minHeight", "min-height"]) {
        style.min_size.height = parse_length_auto(node, "min-height", value)?;
    }
    if let Some(value) = property(node, &["maxWidth", "max-width"]) {
        style.max_size.width = parse_length_auto(node, "max-width", value)?;
    }
    if let Some(value) = property(node, &["maxHeight", "max-height"]) {
        style.max_size.height = parse_length_auto(node, "max-height", value)?;
    }

    style.margin = rect_length_auto(
        node,
        "margin",
        [
            ("marginLeft", "margin-left"),
            ("marginRight", "margin-right"),
            ("marginTop", "margin-top"),
            ("marginBottom", "margin-bottom"),
        ],
        LengthPercentageAuto::length(0.0),
    )?;
    style.padding = rect_length(
        node,
        "padding",
        [
            ("paddingLeft", "padding-left"),
            ("paddingRight", "padding-right"),
            ("paddingTop", "padding-top"),
            ("paddingBottom", "padding-bottom"),
        ],
        LengthPercentage::length(0.0),
    )?;
    style.border = rect_length(
        node,
        "borderWidth",
        [
            ("borderLeftWidth", "border-left-width"),
            ("borderRightWidth", "border-right-width"),
            ("borderTopWidth", "border-top-width"),
            ("borderBottomWidth", "border-bottom-width"),
        ],
        LengthPercentage::length(0.0),
    )?;
    style.inset = rect_length_auto(
        node,
        "inset",
        [
            ("left", "inset-left"),
            ("right", "inset-right"),
            ("top", "inset-top"),
            ("bottom", "inset-bottom"),
        ],
        LengthPercentageAuto::auto(),
    )?;

    if let Some(value) = property(node, &["aspectRatio", "aspect-ratio"]) {
        let ratio = parse_number(node, "aspect-ratio", value)?;
        if ratio <= 0.0 {
            return Err(invalid_style(node, "aspect-ratio", value));
        }
        style.aspect_ratio = Some(ratio);
    }

    if let Some(value) = property(node, &["gap"]) {
        let gap = parse_length(node, "gap", value)?;
        style.gap = Size {
            width: gap,
            height: gap,
        };
    }
    if let Some(value) = property(node, &["columnGap", "column-gap"]) {
        style.gap.width = parse_length(node, "column-gap", value)?;
    }
    if let Some(value) = property(node, &["rowGap", "row-gap"]) {
        style.gap.height = parse_length(node, "row-gap", value)?;
    }

    if let Some(value) = property(node, &["flexDirection", "flex-direction"]) {
        style.flex_direction = match string_value(value) {
            Some("row") => FlexDirection::Row,
            Some("row-reverse") => FlexDirection::RowReverse,
            Some("column") => FlexDirection::Column,
            Some("column-reverse") => FlexDirection::ColumnReverse,
            _ => return Err(invalid_style(node, "flex-direction", value)),
        };
    }
    if let Some(value) = property(node, &["flexWrap", "flex-wrap"]) {
        style.flex_wrap = match string_value(value) {
            Some("nowrap") => FlexWrap::NoWrap,
            Some("wrap") => FlexWrap::Wrap,
            Some("wrap-reverse") => FlexWrap::WrapReverse,
            _ => return Err(invalid_style(node, "flex-wrap", value)),
        };
    }
    if let Some(value) = property(node, &["flexGrow", "flex-grow"]) {
        style.flex_grow = parse_non_negative(node, "flex-grow", value)?;
    }
    if let Some(value) = property(node, &["flexShrink", "flex-shrink"]) {
        style.flex_shrink = parse_non_negative(node, "flex-shrink", value)?;
    }
    if let Some(value) = property(node, &["flexBasis", "flex-basis"]) {
        style.flex_basis = parse_dimension(node, "flex-basis", value)?;
    }

    if let Some(value) = property(node, &["alignItems", "align-items"]) {
        style.align_items = parse_align_items(node, "align-items", value)?;
    }
    if let Some(value) = property(node, &["alignSelf", "align-self"]) {
        style.align_self = Some(parse_align_items(node, "align-self", value)?);
    }
    if let Some(value) = property(node, &["justifyContent", "justify-content"]) {
        style.justify_content = parse_justify_content(node, "justify-content", value)?;
    }
    if let Some(value) = property(node, &["alignContent", "align-content"]) {
        style.align_content = parse_align_content(node, "align-content", value)?;
    }

    Ok(style)
}

fn property<'a>(node: &'a Node, keys: &[&str]) -> Option<&'a PropertyValue> {
    keys.iter().find_map(|key| node.properties.get(*key))
}

fn string_value(value: &PropertyValue) -> Option<&str> {
    match value {
        PropertyValue::String(value) => Some(value.trim()),
        _ => None,
    }
}

fn value_debug(value: &PropertyValue) -> String {
    match value {
        PropertyValue::String(value) => value.clone(),
        PropertyValue::Number(value) => value.to_string(),
        PropertyValue::Boolean(value) => value.to_string(),
        PropertyValue::Null => "null".to_owned(),
    }
}

fn invalid_style(node: &Node, property: &str, value: &PropertyValue) -> LayoutError {
    LayoutError::InvalidStyle {
        node: node.id,
        property: property.to_owned(),
        value: value_debug(value),
    }
}

enum ParsedLength {
    Length(f32),
    Percent(f32),
    Auto,
}

fn parse_scalar(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<ParsedLength, LayoutError> {
    match value {
        PropertyValue::Number(number) => {
            let number = *number as f32;
            if !number.is_finite() {
                return Err(invalid_style(node, property_name, value));
            }
            Ok(ParsedLength::Length(number))
        }
        PropertyValue::String(raw) => {
            let raw = raw.trim();
            if raw.eq_ignore_ascii_case("auto") {
                return Ok(ParsedLength::Auto);
            }

            if let Some(percent) = raw.strip_suffix('%') {
                let percent = percent
                    .trim()
                    .parse::<f32>()
                    .map_err(|_| invalid_style(node, property_name, value))?;
                if !percent.is_finite() {
                    return Err(invalid_style(node, property_name, value));
                }
                return Ok(ParsedLength::Percent(percent / 100.0));
            }

            let numeric = raw
                .strip_suffix("px")
                .or_else(|| raw.strip_suffix("dp"))
                .or_else(|| raw.strip_suffix("pt"))
                .unwrap_or(raw)
                .trim()
                .parse::<f32>()
                .map_err(|_| invalid_style(node, property_name, value))?;
            if !numeric.is_finite() {
                return Err(invalid_style(node, property_name, value));
            }
            Ok(ParsedLength::Length(numeric))
        }
        _ => Err(invalid_style(node, property_name, value)),
    }
}

fn parse_dimension(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<Dimension, LayoutError> {
    if let Some(raw) = string_value(value) {
        match raw {
            "min-content" => return Ok(Dimension::min_content()),
            "max-content" => return Ok(Dimension::max_content()),
            "fit-content" => return Ok(Dimension::fit_content()),
            "stretch" => return Ok(Dimension::stretch()),
            _ => {}
        }
    }

    match parse_scalar(node, property_name, value)? {
        ParsedLength::Length(value) => Ok(Dimension::length(value)),
        ParsedLength::Percent(value) => Ok(Dimension::percent(value)),
        ParsedLength::Auto => Ok(Dimension::auto()),
    }
}

fn parse_length_auto(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<LengthPercentageAuto, LayoutError> {
    match parse_scalar(node, property_name, value)? {
        ParsedLength::Length(value) => Ok(LengthPercentageAuto::length(value)),
        ParsedLength::Percent(value) => Ok(LengthPercentageAuto::percent(value)),
        ParsedLength::Auto => Ok(LengthPercentageAuto::auto()),
    }
}

fn parse_length(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<LengthPercentage, LayoutError> {
    match parse_scalar(node, property_name, value)? {
        ParsedLength::Length(value) => Ok(LengthPercentage::length(value)),
        ParsedLength::Percent(value) => Ok(LengthPercentage::percent(value)),
        ParsedLength::Auto => Err(invalid_style(node, property_name, value)),
    }
}

fn parse_number(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<f32, LayoutError> {
    let number = match value {
        PropertyValue::Number(number) => *number as f32,
        PropertyValue::String(value) => value.trim().parse::<f32>().map_err(|_| {
            invalid_style(node, property_name, &PropertyValue::String(value.clone()))
        })?,
        _ => return Err(invalid_style(node, property_name, value)),
    };
    if !number.is_finite() {
        return Err(invalid_style(node, property_name, value));
    }
    Ok(number)
}

fn parse_non_negative(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<f32, LayoutError> {
    let number = parse_number(node, property_name, value)?;
    if number < 0.0 {
        return Err(invalid_style(node, property_name, value));
    }
    Ok(number)
}

fn rect_length_auto(
    node: &Node,
    shorthand: &str,
    sides: [(&str, &str); 4],
    default: LengthPercentageAuto,
) -> Result<Rect<LengthPercentageAuto>, LayoutError> {
    let base = property(node, &[shorthand])
        .map(|value| parse_length_auto(node, shorthand, value))
        .transpose()?
        .unwrap_or(default);
    let mut values = [base; 4];
    for (index, (camel, kebab)) in sides.iter().enumerate() {
        if let Some(value) = property(node, &[*camel, *kebab]) {
            values[index] = parse_length_auto(node, kebab, value)?;
        }
    }
    Ok(Rect {
        left: values[0],
        right: values[1],
        top: values[2],
        bottom: values[3],
    })
}

fn rect_length(
    node: &Node,
    shorthand: &str,
    sides: [(&str, &str); 4],
    default: LengthPercentage,
) -> Result<Rect<LengthPercentage>, LayoutError> {
    let base = property(node, &[shorthand])
        .map(|value| parse_length(node, shorthand, value))
        .transpose()?
        .unwrap_or(default);
    let mut values = [base; 4];
    for (index, (camel, kebab)) in sides.iter().enumerate() {
        if let Some(value) = property(node, &[*camel, *kebab]) {
            values[index] = parse_length(node, kebab, value)?;
        }
    }
    Ok(Rect {
        left: values[0],
        right: values[1],
        top: values[2],
        bottom: values[3],
    })
}

fn parse_align_items(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<AlignItems, LayoutError> {
    match string_value(value) {
        Some("normal") => Ok(AlignItems::NORMAL),
        Some("start") => Ok(AlignItems::START),
        Some("end") => Ok(AlignItems::END),
        Some("flex-start") => Ok(AlignItems::FLEX_START),
        Some("flex-end") => Ok(AlignItems::FLEX_END),
        Some("center") => Ok(AlignItems::CENTER),
        Some("stretch") => Ok(AlignItems::STRETCH),
        Some("baseline") => Ok(AlignItems::BASELINE),
        _ => Err(invalid_style(node, property_name, value)),
    }
}

fn parse_justify_content(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<JustifyContent, LayoutError> {
    match string_value(value) {
        Some("normal") => Ok(JustifyContent::NORMAL),
        Some("start") => Ok(JustifyContent::START),
        Some("end") => Ok(JustifyContent::END),
        Some("flex-start") => Ok(JustifyContent::FLEX_START),
        Some("flex-end") => Ok(JustifyContent::FLEX_END),
        Some("center") => Ok(JustifyContent::CENTER),
        Some("space-between") => Ok(JustifyContent::SPACE_BETWEEN),
        Some("space-around") => Ok(JustifyContent::SPACE_AROUND),
        Some("space-evenly") => Ok(JustifyContent::SPACE_EVENLY),
        Some("stretch") => Ok(JustifyContent::STRETCH),
        _ => Err(invalid_style(node, property_name, value)),
    }
}

fn parse_align_content(
    node: &Node,
    property_name: &str,
    value: &PropertyValue,
) -> Result<AlignContent, LayoutError> {
    match string_value(value) {
        Some("normal") => Ok(AlignContent::NORMAL),
        Some("start") => Ok(AlignContent::START),
        Some("end") => Ok(AlignContent::END),
        Some("flex-start") => Ok(AlignContent::FLEX_START),
        Some("flex-end") => Ok(AlignContent::FLEX_END),
        Some("center") => Ok(AlignContent::CENTER),
        Some("space-between") => Ok(AlignContent::SPACE_BETWEEN),
        Some("space-around") => Ok(AlignContent::SPACE_AROUND),
        Some("space-evenly") => Ok(AlignContent::SPACE_EVENLY),
        Some("stretch") => Ok(AlignContent::STRETCH),
        _ => Err(invalid_style(node, property_name, value)),
    }
}

struct ChildIds<'a>(std::iter::Copied<std::slice::Iter<'a, NodeId>>);

impl Iterator for ChildIds<'_> {
    type Item = TaffyNodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|id| TaffyNodeId::new(id.get()))
    }
}

struct LayoutWorkspace<'a> {
    nodes: BTreeMap<NodeId, NodeState>,
    text_measurer: &'a dyn TextMeasurer,
    measurement_errors: BTreeSet<NodeId>,
}

impl LayoutWorkspace<'_> {
    fn core_id(id: TaffyNodeId) -> NodeId {
        NodeId::new(u64::from(id))
    }

    fn state(&self, id: TaffyNodeId) -> &NodeState {
        self.nodes
            .get(&Self::core_id(id))
            .expect("Taffy traversal only visits committed UiTree nodes")
    }

    fn state_mut(&mut self, id: TaffyNodeId) -> &mut NodeState {
        self.nodes
            .get_mut(&Self::core_id(id))
            .expect("Taffy traversal only visits committed UiTree nodes")
    }
}

impl TraversePartialTree for LayoutWorkspace<'_> {
    type ChildIter<'a>
        = ChildIds<'a>
    where
        Self: 'a;

    fn child_ids(&self, parent_node_id: TaffyNodeId) -> Self::ChildIter<'_> {
        ChildIds(self.state(parent_node_id).children.iter().copied())
    }

    fn child_count(&self, parent_node_id: TaffyNodeId) -> usize {
        self.state(parent_node_id).children.len()
    }

    fn get_child_id(&self, parent_node_id: TaffyNodeId, child_index: usize) -> TaffyNodeId {
        TaffyNodeId::new(self.state(parent_node_id).children[child_index].get())
    }
}

impl LayoutPartialTree for LayoutWorkspace<'_> {
    type CustomIdent = String;
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_core_container_style(&self, node_id: TaffyNodeId) -> Self::CoreContainerStyle<'_> {
        &self.state(node_id).style
    }

    fn set_unrounded_layout(&mut self, node_id: TaffyNodeId, layout: &Layout) {
        self.state_mut(node_id).layout = *layout;
    }

    fn compute_child_layout(
        &mut self,
        node_id: TaffyNodeId,
        inputs: taffy::tree::LayoutInput,
    ) -> LayoutOutput {
        let perform_layout = inputs.run_mode == taffy::RunMode::PerformLayout;

        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let core_id = Self::core_id(node_id);
            let (is_text, style, text) = {
                let state = tree.state(node_id);
                (
                    state.is_text,
                    state.style.clone(),
                    state.text.clone().unwrap_or_default(),
                )
            };

            let mut output = if is_text {
                let measurer = tree.text_measurer;
                let mut invalid_measurement = false;
                let output = compute_leaf_layout(
                    inputs,
                    &style,
                    |_value, _basis| 0.0,
                    |known_dimensions, available_space| {
                        let measured = measurer.measure(
                            core_id,
                            &text,
                            IntrinsicConstraints {
                                known_width: known_dimensions.width,
                                known_height: known_dimensions.height,
                                available_width: map_available(available_space.width),
                                available_height: map_available(available_space.height),
                            },
                        );
                        if !measured.width.is_finite()
                            || !measured.height.is_finite()
                            || measured.width < 0.0
                            || measured.height < 0.0
                        {
                            invalid_measurement = true;
                            Size {
                                width: 0.0,
                                height: 0.0,
                            }
                        } else {
                            Size {
                                width: measured.width,
                                height: measured.height,
                            }
                        }
                    },
                );
                if invalid_measurement {
                    tree.measurement_errors.insert(core_id);
                }
                output
            } else {
                match style.display {
                    Display::Block | Display::FlowRoot => {
                        compute_block_layout(tree, node_id, inputs, None)
                    }
                    Display::Flex => compute_flexbox_layout(tree, node_id, inputs),
                    Display::Grid => compute_grid_layout(tree, node_id, inputs),
                    Display::None => compute_hidden_layout(tree, node_id),
                }
            };

            if perform_layout && !is_text && style.display != Display::None {
                compute_oof_layout(tree, node_id, &mut output);
            }

            output
        })
    }
}

fn map_available(value: AvailableSpace) -> IntrinsicAvailableSpace {
    match value {
        AvailableSpace::Definite(value) => IntrinsicAvailableSpace::Definite(value),
        AvailableSpace::MinContent => IntrinsicAvailableSpace::MinContent,
        AvailableSpace::MaxContent => IntrinsicAvailableSpace::MaxContent,
    }
}

impl LayoutContainingBlock for LayoutWorkspace<'_> {
    type OofItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_oof_item_style(&self, node_id: TaffyNodeId) -> Self::OofItemStyle<'_> {
        &self.state(node_id).style
    }

    fn clear_hoisted_children(&mut self, node_id: TaffyNodeId) {
        self.state_mut(node_id).hoisted_children.clear();
    }

    fn add_hoisted_children(&mut self, node_id: TaffyNodeId, hoisted: &[TaffyNodeId]) {
        self.state_mut(node_id)
            .hoisted_children
            .extend_from_slice(hoisted);
    }
}

impl CacheTree for LayoutWorkspace<'_> {
    fn cache_get(
        &mut self,
        node_id: TaffyNodeId,
        input: &taffy::tree::LayoutInput,
    ) -> Option<LayoutOutput> {
        self.state_mut(node_id).cache.get(input)
    }

    fn cache_store(
        &mut self,
        node_id: TaffyNodeId,
        input: &taffy::tree::LayoutInput,
        layout_output: LayoutOutput,
    ) {
        self.state_mut(node_id).cache.store(input, layout_output);
    }

    fn cache_clear(&mut self, node_id: TaffyNodeId) {
        self.state_mut(node_id).cache.clear();
    }
}

impl LayoutFlexboxContainer for LayoutWorkspace<'_> {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: TaffyNodeId) -> Self::FlexboxContainerStyle<'_> {
        &self.state(node_id).style
    }

    fn get_flexbox_child_style(&self, child_node_id: TaffyNodeId) -> Self::FlexboxItemStyle<'_> {
        &self.state(child_node_id).style
    }
}

impl LayoutGridContainer for LayoutWorkspace<'_> {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: TaffyNodeId) -> Self::GridContainerStyle<'_> {
        &self.state(node_id).style
    }

    fn get_grid_child_style(&self, child_node_id: TaffyNodeId) -> Self::GridItemStyle<'_> {
        &self.state(child_node_id).style
    }
}

impl LayoutBlockContainer for LayoutWorkspace<'_> {
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: TaffyNodeId) -> Self::BlockContainerStyle<'_> {
        &self.state(node_id).style
    }

    fn get_block_child_style(&self, child_node_id: TaffyNodeId) -> Self::BlockItemStyle<'_> {
        &self.state(child_node_id).style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sn_core::{
        BatchHeader, HOST_PROTOCOL_MAJOR, HOST_PROTOCOL_MINOR, HostBatch, HostOperation,
    };

    struct FixedMeasurer {
        revision: u64,
    }

    impl TextMeasurer for FixedMeasurer {
        fn measure(
            &self,
            _node_id: NodeId,
            text: &str,
            _constraints: IntrinsicConstraints,
        ) -> IntrinsicSize {
            IntrinsicSize {
                width: text.chars().count() as f32 * 5.0,
                height: 12.0,
            }
        }

        fn revision(&self) -> u64 {
            self.revision
        }
    }

    fn set(id: u64, name: &str, value: PropertyValue) -> HostOperation {
        HostOperation::SetProperty {
            id: NodeId::new(id),
            name: name.to_owned(),
            value,
        }
    }

    fn string(value: &str) -> PropertyValue {
        PropertyValue::String(value.to_owned())
    }

    fn number(value: f64) -> PropertyValue {
        PropertyValue::Number(value)
    }

    fn commit(tree: &mut UiTree, operations: Vec<HostOperation>) {
        let batch = HostBatch {
            header: BatchHeader {
                major: HOST_PROTOCOL_MAJOR,
                minor: HOST_PROTOCOL_MINOR,
                surface_id: tree.surface_id().get(),
                session_generation: tree.session_generation().get(),
                transaction_id: tree.last_transaction_id() + 1,
                expected_revision: tree.revision(),
            },
            operations,
        };
        tree.apply_batch(&batch).unwrap();
    }

    fn root_with_children(child_ids: &[u64]) -> (UiTree, NodeId) {
        let mut tree = UiTree::new(SurfaceId::new(1), SessionGeneration::new(1));
        let root = NodeId::new(1);
        let mut operations = vec![HostOperation::CreateElement {
            id: root,
            tag: "root".to_owned(),
        }];
        for child in child_ids {
            operations.push(HostOperation::CreateElement {
                id: NodeId::new(*child),
                tag: "view".to_owned(),
            });
            operations.push(HostOperation::InsertNode {
                parent: root,
                child: NodeId::new(*child),
                anchor: None,
            });
        }
        commit(&mut tree, operations);
        (tree, root)
    }

    fn close(left: f32, right: f32) {
        assert!(
            (left - right).abs() < 0.01,
            "expected {left} to be approximately {right}"
        );
    }

    #[test]
    fn flex_row_golden_positions_children_in_logical_units() {
        let (mut tree, root) = root_with_children(&[2, 3]);
        commit(
            &mut tree,
            vec![
                set(1, "display", string("flex")),
                set(1, "flexDirection", string("row")),
                set(1, "width", number(100.0)),
                set(1, "height", number(40.0)),
                set(2, "width", number(30.0)),
                set(2, "height", number(20.0)),
                set(3, "width", number(40.0)),
                set(3, "height", number(20.0)),
            ],
        );

        let mut engine = LayoutEngine::new();
        let layout = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();

        close(layout.node(root).unwrap().width, 100.0);
        close(layout.node(NodeId::new(2)).unwrap().x, 0.0);
        close(layout.node(NodeId::new(3)).unwrap().x, 30.0);
    }

    #[test]
    fn grid_auto_placement_golden_places_items_in_rows_by_default() {
        let (mut tree, root) = root_with_children(&[2, 3]);
        commit(
            &mut tree,
            vec![
                set(1, "display", string("grid")),
                set(1, "width", number(120.0)),
                set(1, "height", number(40.0)),
                set(2, "width", number(40.0)),
                set(2, "height", number(20.0)),
                set(3, "width", number(40.0)),
                set(3, "height", number(20.0)),
            ],
        );

        let mut engine = LayoutEngine::new();
        let layout = engine
            .compute(
                &tree,
                root,
                Viewport::new(120.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();

        close(layout.node(NodeId::new(2)).unwrap().y, 0.0);
        close(layout.node(NodeId::new(3)).unwrap().x, 0.0);
        close(layout.node(NodeId::new(3)).unwrap().y, 20.0);
    }

    #[test]
    fn absolute_and_fixed_items_use_their_respective_containing_blocks() {
        let (mut tree, root) = root_with_children(&[2]);
        commit(
            &mut tree,
            vec![
                set(1, "display", string("flex")),
                set(1, "width", number(200.0)),
                set(1, "height", number(120.0)),
                set(2, "position", string("relative")),
                set(2, "width", number(100.0)),
                set(2, "height", number(70.0)),
                set(2, "marginLeft", number(20.0)),
                HostOperation::CreateElement {
                    id: NodeId::new(3),
                    tag: "absolute".to_owned(),
                },
                HostOperation::InsertNode {
                    parent: NodeId::new(2),
                    child: NodeId::new(3),
                    anchor: None,
                },
                set(3, "position", string("absolute")),
                set(3, "left", number(4.0)),
                set(3, "top", number(6.0)),
                set(3, "width", number(20.0)),
                set(3, "height", number(10.0)),
                HostOperation::CreateElement {
                    id: NodeId::new(4),
                    tag: "fixed".to_owned(),
                },
                HostOperation::InsertNode {
                    parent: NodeId::new(2),
                    child: NodeId::new(4),
                    anchor: None,
                },
                set(4, "position", string("fixed")),
                set(4, "left", number(35.0)),
                set(4, "top", number(15.0)),
                set(4, "width", number(10.0)),
                set(4, "height", number(10.0)),
            ],
        );

        let mut engine = LayoutEngine::new();
        let layout = engine
            .compute(
                &tree,
                root,
                Viewport::new(200.0, 120.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();

        let absolute = layout.node(NodeId::new(3)).unwrap();
        close(absolute.x, 4.0);
        close(absolute.y, 6.0);
        let fixed = layout.node(NodeId::new(4)).unwrap();
        close(fixed.x, 35.0);
        close(fixed.y, 15.0);
    }

    #[test]
    fn color_only_commit_keeps_layout_cache_valid_but_size_change_invalidates_ancestors() {
        let (mut tree, root) = root_with_children(&[2]);
        commit(
            &mut tree,
            vec![
                set(1, "display", string("flex")),
                set(1, "width", number(100.0)),
                set(1, "height", number(40.0)),
                set(2, "width", number(20.0)),
                set(2, "height", number(10.0)),
            ],
        );

        let mut engine = LayoutEngine::new();
        engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();

        commit(&mut tree, vec![set(2, "color", string("red"))]);
        let color_update = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();
        assert!(color_update.stats.invalidated_nodes.is_empty());

        commit(&mut tree, vec![set(2, "width", number(25.0))]);
        let size_update = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();

        assert!(size_update.stats.invalidated_nodes.contains(&root));
        assert!(
            size_update
                .stats
                .invalidated_nodes
                .contains(&NodeId::new(2))
        );
        close(size_update.node(NodeId::new(2)).unwrap().width, 25.0);
    }

    #[test]
    fn text_content_and_measurer_revision_invalidate_intrinsic_sizes() {
        let mut tree = UiTree::new(SurfaceId::new(1), SessionGeneration::new(1));
        let root = NodeId::new(1);
        let text = NodeId::new(2);
        commit(
            &mut tree,
            vec![
                HostOperation::CreateElement {
                    id: root,
                    tag: "label".to_owned(),
                },
                HostOperation::CreateText {
                    id: text,
                    text: "abcd".to_owned(),
                },
                HostOperation::InsertNode {
                    parent: root,
                    child: text,
                    anchor: None,
                },
                set(1, "display", string("flex")),
                set(1, "width", number(100.0)),
                set(1, "height", number(30.0)),
            ],
        );

        let mut engine = LayoutEngine::new();
        let first = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 30.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();
        close(first.node(text).unwrap().width, 20.0);

        commit(
            &mut tree,
            vec![HostOperation::ReplaceText {
                id: text,
                text: "abcdefgh".to_owned(),
            }],
        );
        let second = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 30.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();
        close(second.node(text).unwrap().width, 40.0);
        assert!(second.stats.invalidated_nodes.contains(&text));
        assert!(second.stats.invalidated_nodes.contains(&root));

        let third = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 30.0),
                &FixedMeasurer { revision: 2 },
            )
            .unwrap();
        assert!(third.stats.invalidated_nodes.contains(&text));
        assert!(third.stats.invalidated_nodes.contains(&root));
    }

    #[test]
    fn structure_commit_invalidates_parent_and_new_child() {
        let (mut tree, root) = root_with_children(&[2]);
        commit(
            &mut tree,
            vec![
                set(1, "display", string("flex")),
                set(1, "flexDirection", string("row")),
                set(1, "width", number(100.0)),
                set(1, "height", number(40.0)),
                set(2, "width", number(20.0)),
                set(2, "height", number(10.0)),
            ],
        );

        let mut engine = LayoutEngine::new();
        engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();

        commit(
            &mut tree,
            vec![
                HostOperation::CreateElement {
                    id: NodeId::new(3),
                    tag: "new-child".to_owned(),
                },
                HostOperation::InsertNode {
                    parent: root,
                    child: NodeId::new(3),
                    anchor: None,
                },
                set(3, "width", number(15.0)),
                set(3, "height", number(10.0)),
            ],
        );

        let updated = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap();

        assert!(updated.stats.invalidated_nodes.contains(&root));
        assert!(updated.stats.invalidated_nodes.contains(&NodeId::new(3)));
        close(updated.node(NodeId::new(3)).unwrap().x, 20.0);
    }

    struct InvalidMeasurer;

    impl TextMeasurer for InvalidMeasurer {
        fn measure(
            &self,
            _node_id: NodeId,
            _text: &str,
            _constraints: IntrinsicConstraints,
        ) -> IntrinsicSize {
            IntrinsicSize {
                width: f32::NAN,
                height: 12.0,
            }
        }
    }

    #[test]
    fn invalid_intrinsic_measurement_is_not_cached_as_a_successful_layout() {
        let mut tree = UiTree::new(SurfaceId::new(1), SessionGeneration::new(1));
        let root = NodeId::new(1);
        let text = NodeId::new(2);
        commit(
            &mut tree,
            vec![
                HostOperation::CreateElement {
                    id: root,
                    tag: "label".to_owned(),
                },
                HostOperation::CreateText {
                    id: text,
                    text: "invalid".to_owned(),
                },
                HostOperation::InsertNode {
                    parent: root,
                    child: text,
                    anchor: None,
                },
            ],
        );

        let mut engine = LayoutEngine::new();
        for _ in 0..2 {
            let error = engine
                .compute(&tree, root, Viewport::new(100.0, 30.0), &InvalidMeasurer)
                .unwrap_err();
            assert!(matches!(
                error,
                LayoutError::InvalidIntrinsicMeasurement(invalid_id) if invalid_id == text
            ));
        }
    }

    #[test]
    fn unsupported_sticky_position_fails_instead_of_silently_mapping_to_static() {
        let (mut tree, root) = root_with_children(&[2]);
        commit(&mut tree, vec![set(2, "position", string("sticky"))]);

        let mut engine = LayoutEngine::new();
        let error = engine
            .compute(
                &tree,
                root,
                Viewport::new(100.0, 40.0),
                &FixedMeasurer { revision: 1 },
            )
            .unwrap_err();

        assert!(matches!(error, LayoutError::UnsupportedPosition { .. }));
    }
}
