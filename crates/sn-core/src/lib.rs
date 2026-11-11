mod host_protocol;

pub use host_protocol::{
    BatchHeader, HostBatch, HostOperation, MAX_BATCH_BYTES, MAX_OPERATIONS, ProtocolError,
    ValueKind, decode_batch,
};

use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceId(u64);

impl SurfaceId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionGeneration(u64);

impl SessionGeneration {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u64);

impl NodeId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HandlerId(u32);

impl HandlerId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Element(String),
    Text,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub text: Option<String>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub properties: BTreeMap<String, PropertyValue>,
    pub events: BTreeMap<String, HandlerId>,
    pub resources: BTreeMap<String, u64>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TreeError {
    #[error("node {0:?} does not exist")]
    MissingNode(NodeId),
    #[error("cannot insert a node into itself or one of its descendants")]
    Cycle,
    #[error("anchor is not a child of the specified parent")]
    InvalidAnchor,
    #[error("text nodes cannot contain child nodes or element properties")]
    InvalidNodeKind,
    #[error("invalid or empty {0}")]
    InvalidName(&'static str),
    #[error("node identifiers must be monotonic and never reused")]
    InvalidNodeId,
    #[error("node identifier space exhausted")]
    NodeIdExhausted,
    #[error("handler identifier must be non-zero")]
    InvalidHandlerId,
    #[error("resource identifier must be non-zero")]
    InvalidResourceId,
    #[error("batch targets a different Surface")]
    ForeignSurface,
    #[error("batch targets a stale or different Surface session")]
    StaleSession,
    #[error("batch revision conflict: expected {expected}, received {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error("batch transaction sequence conflict: expected {expected}, received {actual}")]
    TransactionSequence { expected: u64, actual: u64 },
    #[error("tree revision space exhausted")]
    RevisionExhausted,
    #[error("transaction identifier space exhausted")]
    TransactionIdExhausted,
    #[error("event handler {0} is not registered")]
    MissingHandler(u32),
    #[error("event {0} is not bound on node")]
    MissingEvent(String),
    #[error("resource {0} is not attached to node")]
    MissingResource(String),
    #[error("numeric property values must be finite")]
    NonFiniteNumber,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitAck {
    pub surface_id: SurfaceId,
    pub session_generation: SessionGeneration,
    pub transaction_id: u64,
    pub revision: u64,
    pub next_node_id: u64,
    pub delta: Vec<HostOperation>,
}

#[derive(Debug, Clone)]
pub struct UiTree {
    surface_id: SurfaceId,
    session_generation: SessionGeneration,
    revision: u64,
    last_transaction_id: u64,
    next_node_id: u64,
    nodes: BTreeMap<NodeId, Node>,
    roots: Vec<NodeId>,
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new(SurfaceId::new(1), SessionGeneration::new(1))
    }
}

impl UiTree {
    pub fn new(surface_id: SurfaceId, session_generation: SessionGeneration) -> Self {
        Self {
            surface_id,
            session_generation,
            revision: 0,
            last_transaction_id: 0,
            next_node_id: 1,
            nodes: BTreeMap::new(),
            roots: Vec::new(),
        }
    }

    pub const fn surface_id(&self) -> SurfaceId {
        self.surface_id
    }
    pub const fn session_generation(&self) -> SessionGeneration {
        self.session_generation
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    pub const fn next_node_id(&self) -> u64 {
        self.next_node_id
    }
    pub const fn last_transaction_id(&self) -> u64 {
        self.last_transaction_id
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn create_element(&mut self, tag: impl Into<String>) -> Result<NodeId, TreeError> {
        let tag = tag.into();
        validate_name(&tag, "element tag", 256)?;
        self.allocate_node(NodeKind::Element(tag), None)
    }

    pub fn create_text(&mut self, value: impl Into<String>) -> Result<NodeId, TreeError> {
        self.allocate_node(NodeKind::Text, Some(value.into()))
    }

    fn allocate_node(&mut self, kind: NodeKind, text: Option<String>) -> Result<NodeId, TreeError> {
        let id = NodeId(self.next_node_id);
        self.next_node_id = self
            .next_node_id
            .checked_add(1)
            .ok_or(TreeError::NodeIdExhausted)?;
        self.nodes.insert(
            id,
            Node {
                id,
                kind,
                text,
                parent: None,
                children: Vec::new(),
                properties: BTreeMap::new(),
                events: BTreeMap::new(),
                resources: BTreeMap::new(),
            },
        );
        self.roots.push(id);
        Ok(id)
    }

    pub fn node(&self, id: NodeId) -> Result<&Node, TreeError> {
        self.nodes.get(&id).ok_or(TreeError::MissingNode(id))
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn find_element(&self, tag: &str) -> Option<NodeId> {
        self.nodes.values().find_map(|node| {
            if matches!(&node.kind, NodeKind::Element(value) if value == tag) {
                Some(node.id)
            } else {
                None
            }
        })
    }

    pub fn text_content(&self, id: NodeId) -> Result<&str, TreeError> {
        let node = self.node(id)?;
        if node.kind != NodeKind::Text {
            return Err(TreeError::InvalidNodeKind);
        }
        Ok(node.text.as_deref().unwrap_or_default())
    }

    pub fn find_text_containing(&self, needle: &str) -> Option<NodeId> {
        self.nodes.values().find_map(|node| {
            if node.kind == NodeKind::Text
                && node
                    .text
                    .as_deref()
                    .is_some_and(|value| value.contains(needle))
            {
                Some(node.id)
            } else {
                None
            }
        })
    }

    pub fn handler_ids(&self) -> BTreeSet<HandlerId> {
        self.nodes
            .values()
            .flat_map(|node| node.events.values().copied())
            .collect()
    }

    pub fn set_property(
        &mut self,
        id: NodeId,
        name: impl Into<String>,
        value: PropertyValue,
    ) -> Result<(), TreeError> {
        let name = name.into();
        validate_name(&name, "property name", 256)?;
        if let PropertyValue::Number(number) = &value {
            if !number.is_finite() {
                return Err(TreeError::NonFiniteNumber);
            }
        }
        self.element_mut(id)?.properties.insert(name, value);
        Ok(())
    }

    pub fn insert_before(
        &mut self,
        parent: NodeId,
        child: NodeId,
        anchor: Option<NodeId>,
    ) -> Result<(), TreeError> {
        let parent_node = self
            .nodes
            .get(&parent)
            .ok_or(TreeError::MissingNode(parent))?;
        if matches!(&parent_node.kind, NodeKind::Text) {
            return Err(TreeError::InvalidNodeKind);
        }
        if !self.nodes.contains_key(&child) {
            return Err(TreeError::MissingNode(child));
        }
        if parent == child || self.is_descendant(parent, child) {
            return Err(TreeError::Cycle);
        }

        // Validate anchor ownership before the self-anchor no-op. Otherwise a child
        // from another parent can be accepted as its own foreign anchor.
        if let Some(anchor_id) = anchor {
            if !parent_node.children.contains(&anchor_id) {
                return Err(TreeError::InvalidAnchor);
            }
        }
        if anchor == Some(child) {
            return Ok(());
        }

        self.detach(child);
        let index = anchor
            .and_then(|anchor_id| {
                self.nodes
                    .get(&parent)
                    .and_then(|n| n.children.iter().position(|id| *id == anchor_id))
            })
            .unwrap_or_else(|| self.nodes.get(&parent).map_or(0, |n| n.children.len()));
        self.nodes
            .get_mut(&parent)
            .ok_or(TreeError::MissingNode(parent))?
            .children
            .insert(index, child);
        self.nodes
            .get_mut(&child)
            .ok_or(TreeError::MissingNode(child))?
            .parent = Some(parent);
        Ok(())
    }

    pub fn remove(&mut self, id: NodeId) -> Result<Vec<NodeId>, TreeError> {
        if !self.nodes.contains_key(&id) {
            return Err(TreeError::MissingNode(id));
        }
        self.detach(id);
        let mut removed = Vec::new();
        let mut pending = vec![id];
        while let Some(current) = pending.pop() {
            if let Some(node) = self.nodes.remove(&current) {
                pending.extend(node.children.iter().copied());
                removed.push(current);
            }
        }
        Ok(removed)
    }

    pub fn apply_batch(&mut self, batch: &HostBatch) -> Result<CommitAck, TreeError> {
        if batch.header.surface_id != self.surface_id.get() {
            return Err(TreeError::ForeignSurface);
        }
        if batch.header.session_generation != self.session_generation.get() {
            return Err(TreeError::StaleSession);
        }
        if batch.header.expected_revision != self.revision {
            return Err(TreeError::RevisionConflict {
                expected: self.revision,
                actual: batch.header.expected_revision,
            });
        }
        let expected_transaction = self
            .last_transaction_id
            .checked_add(1)
            .ok_or(TreeError::TransactionIdExhausted)?;
        if batch.header.transaction_id != expected_transaction {
            return Err(TreeError::TransactionSequence {
                expected: expected_transaction,
                actual: batch.header.transaction_id,
            });
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(TreeError::RevisionExhausted)?;

        let mut staging = self.clone();
        for operation in &batch.operations {
            staging.apply_operation(operation)?;
        }
        staging.revision = next_revision;
        staging.last_transaction_id = batch.header.transaction_id;

        let ack = CommitAck {
            surface_id: staging.surface_id,
            session_generation: staging.session_generation,
            transaction_id: staging.last_transaction_id,
            revision: staging.revision,
            next_node_id: staging.next_node_id,
            delta: batch.operations.clone(),
        };
        *self = staging;
        Ok(ack)
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.roots.clear();
    }

    fn apply_operation(&mut self, operation: &HostOperation) -> Result<(), TreeError> {
        match operation {
            HostOperation::CreateElement { id, tag } => {
                validate_name(tag, "element tag", 256)?;
                self.insert_reserved_node(*id, NodeKind::Element(tag.clone()), None)
            }
            HostOperation::CreateText { id, text } => {
                self.insert_reserved_node(*id, NodeKind::Text, Some(text.clone()))
            }
            HostOperation::ReplaceText { id, text } => {
                let node = self.nodes.get_mut(id).ok_or(TreeError::MissingNode(*id))?;
                if node.kind != NodeKind::Text {
                    return Err(TreeError::InvalidNodeKind);
                }
                node.text = Some(text.clone());
                Ok(())
            }
            HostOperation::SetProperty { id, name, value } => {
                validate_name(name, "property name", 256)?;
                if let PropertyValue::Number(value) = value {
                    if !value.is_finite() {
                        return Err(TreeError::NonFiniteNumber);
                    }
                }
                self.element_mut(*id)?
                    .properties
                    .insert(name.clone(), value.clone());
                Ok(())
            }
            HostOperation::RemoveProperty { id, name } => {
                validate_name(name, "property name", 256)?;
                self.element_mut(*id)?.properties.remove(name);
                Ok(())
            }
            HostOperation::InsertNode {
                parent,
                child,
                anchor,
            } => self.insert_before(*parent, *child, *anchor),
            HostOperation::RemoveNode { id } => self.remove(*id).map(|_| ()),
            HostOperation::BindEvent {
                id,
                event,
                handler_id,
            } => {
                validate_name(event, "event name", 64)?;
                if handler_id.get() == 0 {
                    return Err(TreeError::InvalidHandlerId);
                }
                self.element_mut(*id)?
                    .events
                    .insert(event.clone(), *handler_id);
                Ok(())
            }
            HostOperation::UnbindEvent { id, event } => {
                validate_name(event, "event name", 64)?;
                let node = self.element_mut(*id)?;
                if node.events.remove(event).is_none() {
                    return Err(TreeError::MissingEvent(event.clone()));
                }
                Ok(())
            }
            HostOperation::AttachResource {
                id,
                name,
                resource_id,
            } => {
                validate_name(name, "resource name", 256)?;
                if *resource_id == 0 {
                    return Err(TreeError::InvalidResourceId);
                }
                self.element_mut(*id)?
                    .resources
                    .insert(name.clone(), *resource_id);
                Ok(())
            }
            HostOperation::DetachResource { id, name } => {
                validate_name(name, "resource name", 256)?;
                let node = self.element_mut(*id)?;
                if node.resources.remove(name).is_none() {
                    return Err(TreeError::MissingResource(name.clone()));
                }
                Ok(())
            }
        }
    }

    fn insert_reserved_node(
        &mut self,
        id: NodeId,
        kind: NodeKind,
        text: Option<String>,
    ) -> Result<(), TreeError> {
        if id.get() == 0 || id.get() != self.next_node_id {
            return Err(TreeError::InvalidNodeId);
        }
        let next = self
            .next_node_id
            .checked_add(1)
            .ok_or(TreeError::NodeIdExhausted)?;
        if self.nodes.contains_key(&id) {
            return Err(TreeError::InvalidNodeId);
        }
        self.nodes.insert(
            id,
            Node {
                id,
                kind,
                text,
                parent: None,
                children: Vec::new(),
                properties: BTreeMap::new(),
                events: BTreeMap::new(),
                resources: BTreeMap::new(),
            },
        );
        self.roots.push(id);
        self.next_node_id = next;
        Ok(())
    }

    fn element_mut(&mut self, id: NodeId) -> Result<&mut Node, TreeError> {
        let node = self.nodes.get_mut(&id).ok_or(TreeError::MissingNode(id))?;
        if matches!(&node.kind, NodeKind::Text) {
            return Err(TreeError::InvalidNodeKind);
        }
        Ok(node)
    }

    fn detach(&mut self, child: NodeId) {
        let parent = self.nodes.get(&child).and_then(|node| node.parent);
        if let Some(parent) = parent {
            if let Some(node) = self.nodes.get_mut(&parent) {
                node.children.retain(|id| *id != child);
            }
        } else {
            self.roots.retain(|id| *id != child);
        }
        if let Some(node) = self.nodes.get_mut(&child) {
            node.parent = None;
        }
    }

    fn is_descendant(&self, candidate: NodeId, ancestor: NodeId) -> bool {
        let mut current = self.nodes.get(&candidate).and_then(|node| node.parent);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.nodes.get(&id).and_then(|node| node.parent);
        }
        false
    }
}

fn validate_name(
    value: &str,
    description: &'static str,
    max_bytes: usize,
) -> Result<(), TreeError> {
    if value.is_empty() || value.len() > max_bytes || value.contains('\0') {
        return Err(TreeError::InvalidName(description));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(value: u64) -> NodeId {
        NodeId::new(value)
    }

    #[test]
    fn inserts_and_moves_nodes_without_duplicate_roots() {
        let mut tree = UiTree::default();
        let root = tree.create_element("window").unwrap();
        let first = tree.create_element("view").unwrap();
        let second = tree.create_text("hello").unwrap();
        tree.insert_before(root, first, None).unwrap();
        tree.insert_before(root, second, Some(first)).unwrap();
        assert_eq!(tree.node(root).unwrap().children, vec![second, first]);
        assert_eq!(tree.roots(), &[root]);
    }

    #[test]
    fn moving_a_sibling_before_another_uses_post_detach_index() {
        let mut tree = UiTree::default();
        let root = tree.create_element("root").unwrap();
        let a = tree.create_element("a").unwrap();
        let b = tree.create_element("b").unwrap();
        let c = tree.create_element("c").unwrap();
        tree.insert_before(root, a, None).unwrap();
        tree.insert_before(root, b, None).unwrap();
        tree.insert_before(root, c, None).unwrap();
        tree.insert_before(root, a, Some(c)).unwrap();
        assert_eq!(tree.node(root).unwrap().children, vec![b, a, c]);
    }

    #[test]
    fn rejects_foreign_anchor_even_when_anchor_is_the_child() {
        let mut tree = UiTree::default();
        let left = tree.create_element("left").unwrap();
        let right = tree.create_element("right").unwrap();
        let child = tree.create_element("child").unwrap();
        tree.insert_before(left, child, None).unwrap();
        let before_left = tree.node(left).unwrap().children.clone();
        let before_right = tree.node(right).unwrap().children.clone();
        assert_eq!(
            tree.insert_before(right, child, Some(child)),
            Err(TreeError::InvalidAnchor)
        );
        assert_eq!(tree.node(left).unwrap().children, before_left);
        assert_eq!(tree.node(right).unwrap().children, before_right);
        assert_eq!(tree.node(child).unwrap().parent, Some(left));
    }

    #[test]
    fn same_parent_self_anchor_is_a_noop() {
        let mut tree = UiTree::default();
        let root = tree.create_element("root").unwrap();
        let child = tree.create_element("child").unwrap();
        tree.insert_before(root, child, None).unwrap();
        assert_eq!(tree.insert_before(root, child, Some(child)), Ok(()));
        assert_eq!(tree.node(root).unwrap().children, vec![child]);
    }

    #[test]
    fn moves_nodes_between_parents_without_duplicate_roots() {
        let mut tree = UiTree::default();
        let left = tree.create_element("left").unwrap();
        let right = tree.create_element("right").unwrap();
        let child = tree.create_element("child").unwrap();
        tree.insert_before(left, child, None).unwrap();
        tree.insert_before(right, child, None).unwrap();
        assert!(tree.node(left).unwrap().children.is_empty());
        assert_eq!(tree.node(right).unwrap().children, vec![child]);
        assert_eq!(tree.node(child).unwrap().parent, Some(right));
        assert_eq!(tree.roots(), &[left, right]);
    }

    #[test]
    fn rejects_cycles() {
        let mut tree = UiTree::default();
        let root = tree.create_element("root").unwrap();
        let child = tree.create_element("child").unwrap();
        tree.insert_before(root, child, None).unwrap();
        assert_eq!(tree.insert_before(child, root, None), Err(TreeError::Cycle));
    }

    #[test]
    fn removes_subtrees_and_events() {
        let mut tree = UiTree::default();
        let root = tree.create_element("root").unwrap();
        let child = tree.create_element("child").unwrap();
        tree.insert_before(root, child, None).unwrap();
        let bind = HostBatch {
            header: BatchHeader {
                major: 1,
                minor: 0,
                surface_id: 1,
                session_generation: 1,
                transaction_id: 1,
                expected_revision: 0,
            },
            operations: vec![HostOperation::BindEvent {
                id: child,
                event: "click".into(),
                handler_id: HandlerId::new(7),
            }],
        };
        tree.apply_batch(&bind).unwrap();
        let removed = tree.remove(root).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(tree.node(child).is_err());
        assert!(tree.roots().is_empty());
        assert!(tree.handler_ids().is_empty());
    }

    #[test]
    fn text_and_typed_properties_are_retained() {
        let mut tree = UiTree::default();
        let element = tree.create_element("view").unwrap();
        let text = tree.create_text("before").unwrap();
        tree.set_property(element, "disabled", PropertyValue::Boolean(true))
            .unwrap();
        tree.set_property(element, "opacity", PropertyValue::Number(0.5))
            .unwrap();
        tree.insert_before(element, text, None).unwrap();
        assert_eq!(tree.text_content(text).unwrap(), "before");
        assert_eq!(
            tree.node(element).unwrap().properties.get("disabled"),
            Some(&PropertyValue::Boolean(true))
        );
    }

    #[test]
    fn invalid_batch_rolls_back_tree_and_revision() {
        let mut tree = UiTree::default();
        let header = BatchHeader {
            major: 1,
            minor: 0,
            surface_id: 1,
            session_generation: 1,
            transaction_id: 1,
            expected_revision: 0,
        };
        let batch = HostBatch {
            header,
            operations: vec![
                HostOperation::CreateElement {
                    id: node_id(1),
                    tag: "root".into(),
                },
                HostOperation::InsertNode {
                    parent: node_id(999),
                    child: node_id(1),
                    anchor: None,
                },
            ],
        };
        assert!(tree.apply_batch(&batch).is_err());
        assert_eq!(tree.revision(), 0);
        assert_eq!(tree.node_count(), 0);
        assert_eq!(tree.next_node_id(), 1);
    }

    #[test]
    fn valid_batch_commits_once_and_preserves_text() {
        let mut tree = UiTree::default();
        let batch = HostBatch {
            header: BatchHeader {
                major: 1,
                minor: 0,
                surface_id: 1,
                session_generation: 1,
                transaction_id: 1,
                expected_revision: 0,
            },
            operations: vec![
                HostOperation::CreateElement {
                    id: node_id(1),
                    tag: "root".into(),
                },
                HostOperation::CreateText {
                    id: node_id(2),
                    text: "hello".into(),
                },
                HostOperation::InsertNode {
                    parent: node_id(1),
                    child: node_id(2),
                    anchor: None,
                },
            ],
        };
        let ack = tree.apply_batch(&batch).unwrap();
        assert_eq!(ack.revision, 1);
        assert_eq!(ack.next_node_id, 3);
        assert_eq!(tree.text_content(node_id(2)).unwrap(), "hello");
    }

    #[test]
    fn stale_revision_is_rejected_without_mutation() {
        let mut tree = UiTree::default();
        let batch = HostBatch {
            header: BatchHeader {
                major: 1,
                minor: 0,
                surface_id: 1,
                session_generation: 1,
                transaction_id: 1,
                expected_revision: 1,
            },
            operations: vec![HostOperation::CreateElement {
                id: node_id(1),
                tag: "root".into(),
            }],
        };
        assert!(matches!(
            tree.apply_batch(&batch),
            Err(TreeError::RevisionConflict { .. })
        ));
        assert_eq!(tree.revision(), 0);
        assert_eq!(tree.node_count(), 0);
    }

    #[test]
    fn rejects_wrong_surface_and_session() {
        let mut tree = UiTree::new(SurfaceId::new(7), SessionGeneration::new(9));
        let mut batch = HostBatch {
            header: BatchHeader {
                major: 1,
                minor: 0,
                surface_id: 8,
                session_generation: 9,
                transaction_id: 1,
                expected_revision: 0,
            },
            operations: vec![HostOperation::CreateElement {
                id: node_id(1),
                tag: "root".into(),
            }],
        };
        assert_eq!(tree.apply_batch(&batch), Err(TreeError::ForeignSurface));
        batch.header.surface_id = 7;
        batch.header.session_generation = 8;
        assert_eq!(tree.apply_batch(&batch), Err(TreeError::StaleSession));
        assert_eq!(tree.revision(), 0);
        assert_eq!(tree.node_count(), 0);
    }
}
