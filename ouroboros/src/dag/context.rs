//! Context Tree - Document Reference Specification
//!
//! The Context Tree defines what documents each task/branch should reference.
//! It is NOT about Claude session management - it's about curating the right
//! context for each worker task.
//!
//! Structure:
//! - Root node: Common documents all tasks need
//! - Branch nodes: Branch-specific documents
//! - Each node has: cached_prefix (shared docs) + delta_docs (branch additions)
//!
//! The context for a task = ancestor chain docs + dependency task results

use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::{Result, anyhow};

/// A node in the context tree representing document references for a branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextNode {
    /// Unique node identifier
    pub node_id: String,
    /// Parent node (None for root)
    pub parent: Option<String>,
    /// Path to cached/shared context document (inherited from this point)
    pub cached_prefix: Option<PathBuf>,
    /// Documents specific to this branch (delta from parent)
    pub delta_docs: Vec<PathBuf>,
    /// When this context was created
    pub created_at: DateTime<Utc>,
    /// Current status
    pub status: ContextStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextStatus {
    /// Active and can accept new work
    Active,
    /// Completed successfully
    Completed,
    /// Failed or abandoned
    Abandoned,
    /// Merged into another context
    Merged { into: String },
}

impl ContextNode {
    pub fn new_root() -> Self {
        Self {
            node_id: format!("ctx-{}", Uuid::new_v4().to_string().split('-').next().unwrap()),
            parent: None,
            cached_prefix: None,
            delta_docs: vec![],
            created_at: Utc::now(),
            status: ContextStatus::Active,
        }
    }

    /// Create a root with initial shared documents
    pub fn new_root_with_docs(docs: Vec<PathBuf>) -> Self {
        Self {
            node_id: format!("ctx-{}", Uuid::new_v4().to_string().split('-').next().unwrap()),
            parent: None,
            cached_prefix: None,
            delta_docs: docs,
            created_at: Utc::now(),
            status: ContextStatus::Active,
        }
    }

    pub fn fork_from(parent: &ContextNode, cached_prefix: Option<PathBuf>) -> Self {
        Self {
            node_id: format!("ctx-{}", Uuid::new_v4().to_string().split('-').next().unwrap()),
            parent: Some(parent.node_id.clone()),
            cached_prefix,
            delta_docs: vec![],
            created_at: Utc::now(),
            status: ContextStatus::Active,
        }
    }

    /// Add a document to this branch's context
    pub fn add_doc(&mut self, doc: PathBuf) {
        self.delta_docs.push(doc);
    }

    pub fn complete(&mut self) {
        self.status = ContextStatus::Completed;
    }

    pub fn abandon(&mut self) {
        self.status = ContextStatus::Abandoned;
    }

    pub fn merge_into(&mut self, target: &str) {
        self.status = ContextStatus::Merged { into: target.to_string() };
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, ContextStatus::Active)
    }
}

/// Branch point connecting workflow DAG to context tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchPoint {
    /// ID of this branch point
    pub id: String,
    /// Source context (where branch originates)
    pub source_context: String,
    /// Task that triggers this branch (e.g., ctx-fill task)
    pub trigger_task: String,
    /// Branch contexts created
    pub branches: Vec<String>,
    /// When this branch was created
    pub created_at: DateTime<Utc>,
}

impl BranchPoint {
    pub fn new(source_context: &str, trigger_task: &str) -> Self {
        Self {
            id: format!("branch-{}", Uuid::new_v4().to_string().split('-').next().unwrap()),
            source_context: source_context.to_string(),
            trigger_task: trigger_task.to_string(),
            branches: vec![],
            created_at: Utc::now(),
        }
    }

    pub fn add_branch(&mut self, node_id: String) {
        self.branches.push(node_id);
    }
}

/// Manages the context tree structure for document references
pub struct ContextTree {
    /// All context nodes indexed by node_id
    nodes: HashMap<String, ContextNode>,
    /// Branch points indexed by id
    branch_points: HashMap<String, BranchPoint>,
    /// Root node id
    root: Option<String>,
}

impl ContextTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            branch_points: HashMap::new(),
            root: None,
        }
    }

    /// Initialize with a root context
    pub fn init_root(&mut self) -> &ContextNode {
        let root = ContextNode::new_root();
        let node_id = root.node_id.clone();
        self.nodes.insert(node_id.clone(), root);
        self.root = Some(node_id.clone());
        self.nodes.get(&node_id).unwrap()
    }

    /// Initialize with root documents
    pub fn init_root_with_docs(&mut self, docs: Vec<PathBuf>) -> &ContextNode {
        let root = ContextNode::new_root_with_docs(docs);
        let node_id = root.node_id.clone();
        self.nodes.insert(node_id.clone(), root);
        self.root = Some(node_id.clone());
        self.nodes.get(&node_id).unwrap()
    }

    /// Get root context
    pub fn root(&self) -> Option<&ContextNode> {
        self.root.as_ref().and_then(|id| self.nodes.get(id))
    }

    /// Get context by node id
    pub fn get(&self, node_id: &str) -> Option<&ContextNode> {
        self.nodes.get(node_id)
    }

    /// Get mutable context by node id
    pub fn get_mut(&mut self, node_id: &str) -> Option<&mut ContextNode> {
        self.nodes.get_mut(node_id)
    }

    /// Create a branch from a source context
    pub fn branch(
        &mut self,
        source_id: &str,
        trigger_task: &str,
        branch_count: usize,
        cached_prefix: Option<PathBuf>,
    ) -> Result<BranchPoint> {
        let source = self.nodes.get(source_id)
            .ok_or_else(|| anyhow!("Source context {} not found", source_id))?
            .clone();

        if !source.is_active() {
            return Err(anyhow!("Cannot branch from inactive context {}", source_id));
        }

        let mut branch_point = BranchPoint::new(source_id, trigger_task);

        for _ in 0..branch_count {
            let branch = ContextNode::fork_from(&source, cached_prefix.clone());
            branch_point.add_branch(branch.node_id.clone());
            self.nodes.insert(branch.node_id.clone(), branch);
        }

        let branch_id = branch_point.id.clone();
        self.branch_points.insert(branch_id, branch_point.clone());

        Ok(branch_point)
    }

    /// Create branches with explicit IDs (for task-based naming)
    pub fn branch_with_ids(
        &mut self,
        source_id: &str,
        trigger_task: &str,
        branch_ids: &[&str],
        cached_prefix: Option<PathBuf>,
    ) -> Result<BranchPoint> {
        let source = self.nodes.get(source_id)
            .ok_or_else(|| anyhow!("Source context {} not found", source_id))?
            .clone();

        if !source.is_active() {
            return Err(anyhow!("Cannot branch from inactive context {}", source_id));
        }

        let mut branch_point = BranchPoint::new(source_id, trigger_task);

        for id in branch_ids {
            let mut branch = ContextNode::fork_from(&source, cached_prefix.clone());
            branch.node_id = format!("ctx-{}", id);
            branch_point.add_branch(branch.node_id.clone());
            self.nodes.insert(branch.node_id.clone(), branch);
        }

        let branch_id = branch_point.id.clone();
        self.branch_points.insert(branch_id, branch_point.clone());

        Ok(branch_point)
    }

    /// Get all active contexts
    pub fn active_contexts(&self) -> Vec<&ContextNode> {
        self.nodes.values()
            .filter(|n| n.is_active())
            .collect()
    }

    /// Get children of a context
    pub fn children(&self, node_id: &str) -> Vec<&ContextNode> {
        self.nodes.values()
            .filter(|n| n.parent.as_deref() == Some(node_id))
            .collect()
    }

    /// Get ancestor chain (from node to root)
    pub fn ancestors(&self, node_id: &str) -> Vec<&ContextNode> {
        let mut result = vec![];
        let mut current = self.nodes.get(node_id);

        while let Some(node) = current {
            result.push(node);
            current = node.parent.as_ref()
                .and_then(|pid| self.nodes.get(pid));
        }

        result
    }

    /// Get full document list for a context (all docs from root to this node)
    /// This is the core function - assembles what docs a task should reference
    pub fn get_docs(&self, node_id: &str) -> Vec<PathBuf> {
        let ancestors = self.ancestors(node_id);
        let mut docs = vec![];

        // Traverse from root to leaf
        for node in ancestors.into_iter().rev() {
            if let Some(prefix) = &node.cached_prefix {
                docs.push(prefix.clone());
            }
            docs.extend(node.delta_docs.iter().cloned());
        }

        docs
    }

    /// Get branch point by id
    pub fn get_branch_point(&self, branch_id: &str) -> Option<&BranchPoint> {
        self.branch_points.get(branch_id)
    }

    /// Check if all branches are completed
    pub fn is_branch_complete(&self, branch_id: &str) -> bool {
        let Some(branch_point) = self.branch_points.get(branch_id) else {
            return false;
        };

        branch_point.branches.iter().all(|node_id| {
            self.nodes.get(node_id)
                .map(|n| matches!(n.status, ContextStatus::Completed | ContextStatus::Merged { .. }))
                .unwrap_or(false)
        })
    }
}

impl Default for ContextTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTreeState {
    pub nodes: Vec<ContextNode>,
    pub branch_points: Vec<BranchPoint>,
    pub root: Option<String>,
}

impl ContextTree {
    pub fn to_state(&self) -> ContextTreeState {
        ContextTreeState {
            nodes: self.nodes.values().cloned().collect(),
            branch_points: self.branch_points.values().cloned().collect(),
            root: self.root.clone(),
        }
    }

    pub fn from_state(state: ContextTreeState) -> Self {
        Self {
            nodes: state.nodes.into_iter().map(|n| (n.node_id.clone(), n)).collect(),
            branch_points: state.branch_points.into_iter().map(|bp| (bp.id.clone(), bp)).collect(),
            root: state.root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_tree_basic() {
        let mut tree = ContextTree::new();

        let root = tree.init_root();
        let root_id = root.node_id.clone();

        assert!(tree.root().is_some());
        assert_eq!(tree.active_contexts().len(), 1);

        // Branch into 3
        let branch = tree.branch(&root_id, "ctx-fill-task", 3, None).unwrap();
        assert_eq!(branch.branches.len(), 3);
        assert_eq!(tree.active_contexts().len(), 4);

        // Check children
        let children = tree.children(&root_id);
        assert_eq!(children.len(), 3);

        // Complete one branch
        let branch_id = branch.branches[0].clone();
        tree.get_mut(&branch_id).unwrap().complete();

        assert_eq!(tree.active_contexts().len(), 3);
        assert!(!tree.is_branch_complete(&branch.id));

        // Complete all branches
        for b in &branch.branches[1..] {
            tree.get_mut(b).unwrap().complete();
        }
        assert!(tree.is_branch_complete(&branch.id));
    }

    #[test]
    fn test_get_docs() {
        let mut tree = ContextTree::new();

        let root = tree.init_root();
        let root_id = root.node_id.clone();

        // Add doc to root
        tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/spec.md"));

        // Branch with cached prefix
        let branch = tree.branch(
            &root_id,
            "ctx-fill",
            1,
            Some(PathBuf::from("./cache/shared.md"))
        ).unwrap();
        let branch_id = branch.branches[0].clone();

        // Add branch-specific doc
        tree.get_mut(&branch_id).unwrap().add_doc(PathBuf::from("./docs/branch-a.md"));

        // Check docs
        let docs = tree.get_docs(&branch_id);
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0], PathBuf::from("./docs/spec.md"));
        assert_eq!(docs[1], PathBuf::from("./cache/shared.md"));
        assert_eq!(docs[2], PathBuf::from("./docs/branch-a.md"));
    }

    #[test]
    fn test_ancestors() {
        let mut tree = ContextTree::new();

        let root = tree.init_root();
        let root_id = root.node_id.clone();

        // Level 1 branch
        let branch1 = tree.branch(&root_id, "task-1", 1, None).unwrap();
        let level1_id = branch1.branches[0].clone();

        // Level 2 branch
        let branch2 = tree.branch(&level1_id, "task-2", 1, None).unwrap();
        let level2_id = branch2.branches[0].clone();

        // Check ancestors
        let ancestors = tree.ancestors(&level2_id);
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0].node_id, level2_id);
        assert_eq!(ancestors[1].node_id, level1_id);
        assert_eq!(ancestors[2].node_id, root_id);
    }
}
