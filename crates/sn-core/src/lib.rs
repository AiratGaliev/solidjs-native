use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u64);

impl NodeId {
    pub const fn get(self) -> u64 {
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

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub properties: BTreeMap<String, PropertyValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Element(String),
    Text,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TreeError {
    #[error("node {0:?} does not exist")]
    MissingNode(NodeId),
    #[error("cannot insert a node into itself or one of its descendants")]
    Cycle,
    #[error("anchor is not a child of the specified parent")]
    InvalidAnchor,
}

#[derive(Debug, Default)]
pub struct UiTree {
    next_id: u64,
    nodes: BTreeMap<NodeId, Node>,
    roots: Vec<NodeId>,
}

impl UiTree {
    pub fn create_element(&mut self, tag: impl Into<String>) -> NodeId {
        self.create_node(NodeKind::Element(tag.into()))
    }

    pub fn create_text(&mut self) -> NodeId {
        self.create_node(NodeKind::Text)
    }

    fn create_node(&mut self, kind: NodeKind) -> NodeId {
        self.next_id += 1;
        let id = NodeId(self.next_id);
        self.nodes.insert(
            id,
            Node {
                id,
                kind,
                parent: None,
                children: Vec::new(),
                properties: BTreeMap::new(),
            },
        );
        self.roots.push(id);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&Node, TreeError> {
        self.nodes.get(&id).ok_or(TreeError::MissingNode(id))
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn set_property(
        &mut self,
        id: NodeId,
        name: impl Into<String>,
        value: PropertyValue,
    ) -> Result<(), TreeError> {
        self.nodes
            .get_mut(&id)
            .ok_or(TreeError::MissingNode(id))?
            .properties
            .insert(name.into(), value);
        Ok(())
    }

    pub fn insert_before(
        &mut self,
        parent: NodeId,
        child: NodeId,
        anchor: Option<NodeId>,
    ) -> Result<(), TreeError> {
        if !self.nodes.contains_key(&parent) {
            return Err(TreeError::MissingNode(parent));
        }
        if !self.nodes.contains_key(&child) {
            return Err(TreeError::MissingNode(child));
        }
        if parent == child || self.is_descendant(parent, child) {
            return Err(TreeError::Cycle);
        }

        if anchor == Some(child) {
            return Ok(());
        }
        if let Some(anchor_id) = anchor {
            if !self
                .nodes
                .get(&parent)
                .expect("parent checked")
                .children
                .contains(&anchor_id)
            {
                return Err(TreeError::InvalidAnchor);
            }
        }
        self.detach(child);
        let parent_node = self.nodes.get_mut(&parent).expect("parent checked");
        let index = anchor
            .and_then(|anchor_id| parent_node.children.iter().position(|id| *id == anchor_id))
            .unwrap_or(parent_node.children.len());
        parent_node.children.insert(index, child);
        self.nodes.get_mut(&child).expect("child checked").parent = Some(parent);
        Ok(())
    }

    pub fn remove(&mut self, id: NodeId) -> Result<(), TreeError> {
        if !self.nodes.contains_key(&id) {
            return Err(TreeError::MissingNode(id));
        }
        self.detach(id);
        let mut pending = vec![id];
        while let Some(current) = pending.pop() {
            if let Some(node) = self.nodes.remove(&current) {
                pending.extend(node.children);
            }
        }
        Ok(())
    }

    fn detach(&mut self, child: NodeId) {
        let parent = self.nodes.get(&child).and_then(|n| n.parent);
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
        let mut current = self.nodes.get(&candidate).and_then(|n| n.parent);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.nodes.get(&id).and_then(|n| n.parent);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_and_moves_nodes_without_duplicate_roots() {
        let mut tree = UiTree::default();
        let root = tree.create_element("window");
        let first = tree.create_element("view");
        let second = tree.create_text();
        tree.insert_before(root, first, None).unwrap();
        tree.insert_before(root, second, Some(first)).unwrap();
        assert_eq!(tree.node(root).unwrap().children, vec![second, first]);
        assert_eq!(tree.roots(), &[root]);
    }

    #[test]
    fn moving_a_sibling_before_another_uses_post_detach_index() {
        let mut tree = UiTree::default();
        let root = tree.create_element("root");
        let a = tree.create_element("a");
        let b = tree.create_element("b");
        let c = tree.create_element("c");
        tree.insert_before(root, a, None).unwrap();
        tree.insert_before(root, b, None).unwrap();
        tree.insert_before(root, c, None).unwrap();
        tree.insert_before(root, a, Some(c)).unwrap();
        assert_eq!(tree.node(root).unwrap().children, vec![b, a, c]);
    }

    #[test]
    fn rejects_cycles() {
        let mut tree = UiTree::default();
        let root = tree.create_element("root");
        let child = tree.create_element("child");
        tree.insert_before(root, child, None).unwrap();
        assert_eq!(tree.insert_before(child, root, None), Err(TreeError::Cycle));
    }

    #[test]
    fn removes_subtrees() {
        let mut tree = UiTree::default();
        let root = tree.create_element("root");
        let child = tree.create_element("child");
        tree.insert_before(root, child, None).unwrap();
        tree.remove(root).unwrap();
        assert!(tree.node(child).is_err());
        assert!(tree.roots().is_empty());
    }
}
