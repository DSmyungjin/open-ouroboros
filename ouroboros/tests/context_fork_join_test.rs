//! Comprehensive tests for Context Tree Fork/Join workflow
//!
//! Tests context node creation, fork operations, join operations,
//! context inheritance, isolation, and complex multi-level scenarios.

use ouroboros::dag::{ContextTree, ContextStatus};
use std::path::PathBuf;
use anyhow::Result;

/// Test basic context node creation and hierarchy
#[test]
fn test_context_node_creation() -> Result<()> {
    let mut tree = ContextTree::new();

    // Initialize root
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Verify root properties
    assert!(tree.root().is_some());
    assert_eq!(tree.root().unwrap().node_id, root_id);
    assert!(tree.root().unwrap().parent.is_none());
    assert_eq!(tree.root().unwrap().status, ContextStatus::Active);
    assert!(tree.root().unwrap().cached_prefix.is_none());
    assert_eq!(tree.root().unwrap().delta_docs.len(), 0);

    Ok(())
}

/// Test root initialization with documents
#[test]
fn test_root_with_initial_docs() -> Result<()> {
    let mut tree = ContextTree::new();

    let docs = vec![
        PathBuf::from("./docs/architecture.md"),
        PathBuf::from("./docs/api-spec.md"),
    ];

    let root = tree.init_root_with_docs(docs.clone());
    assert_eq!(root.delta_docs.len(), 2);
    assert_eq!(root.delta_docs, docs);

    Ok(())
}

/// Test single-level fork operation
#[test]
fn test_single_fork() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Fork into 3 branches
    let branch_point = tree.branch(&root_id, "ctx-fill-task", 3, None)?;

    // Verify branch point
    assert_eq!(branch_point.source_context, root_id);
    assert_eq!(branch_point.trigger_task, "ctx-fill-task");
    assert_eq!(branch_point.branches.len(), 3);

    // Verify all branches are active
    assert_eq!(tree.active_contexts().len(), 4); // root + 3 branches

    // Verify parent relationships
    for branch_id in &branch_point.branches {
        let branch = tree.get(branch_id).unwrap();
        assert_eq!(branch.parent.as_ref().unwrap(), &root_id);
        assert!(branch.is_active());
    }

    // Verify children count
    let children = tree.children(&root_id);
    assert_eq!(children.len(), 3);

    Ok(())
}

/// Test fork with cached prefix
#[test]
fn test_fork_with_cached_prefix() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Add document to root
    tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/spec.md"));

    // Fork with cached prefix
    let cached_path = PathBuf::from("./cache/shared-context.md");
    let branch_point = tree.branch(&root_id, "ctx-fill", 2, Some(cached_path.clone()))?;

    // Verify branches have cached prefix
    for branch_id in &branch_point.branches {
        let branch = tree.get(branch_id).unwrap();
        assert_eq!(branch.cached_prefix, Some(cached_path.clone()));
    }

    Ok(())
}

/// Test fork with explicit branch IDs
#[test]
fn test_fork_with_explicit_ids() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    let branch_ids = ["task-1", "task-2", "task-3"];
    let branch_point = tree.branch_with_ids(&root_id, "ctx-fill", &branch_ids, None)?;

    // Verify branch IDs
    assert_eq!(branch_point.branches.len(), 3);
    for (i, branch_id) in branch_point.branches.iter().enumerate() {
        assert_eq!(branch_id, &format!("ctx-{}", branch_ids[i]));
    }

    Ok(())
}

/// Test that forking from inactive context fails
#[test]
fn test_fork_from_inactive_fails() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Complete the root context
    tree.get_mut(&root_id).unwrap().complete();

    // Attempt to fork should fail
    let result = tree.branch(&root_id, "ctx-fill", 2, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("inactive"));

    Ok(())
}

/// Test context isolation - branches don't share delta_docs
#[test]
fn test_context_isolation() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Add document to root
    tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/root.md"));

    // Fork into 2 branches
    let branch_point = tree.branch(&root_id, "ctx-fill", 2, None)?;
    let branch1_id = branch_point.branches[0].clone();
    let branch2_id = branch_point.branches[1].clone();

    // Add different docs to each branch
    tree.get_mut(&branch1_id).unwrap().add_doc(PathBuf::from("./docs/branch1.md"));
    tree.get_mut(&branch2_id).unwrap().add_doc(PathBuf::from("./docs/branch2.md"));

    // Verify isolation
    let branch1 = tree.get(&branch1_id).unwrap();
    let branch2 = tree.get(&branch2_id).unwrap();

    assert_eq!(branch1.delta_docs.len(), 1);
    assert_eq!(branch2.delta_docs.len(), 1);
    assert_ne!(branch1.delta_docs, branch2.delta_docs);

    Ok(())
}

/// Test context inheritance through get_docs()
#[test]
fn test_context_inheritance() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Add document to root
    tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/root.md"));

    // Fork with cached prefix
    let cached = PathBuf::from("./cache/shared.md");
    let branch_point = tree.branch(&root_id, "ctx-fill", 1, Some(cached.clone()))?;
    let branch_id = branch_point.branches[0].clone();

    // Add branch-specific document
    tree.get_mut(&branch_id).unwrap().add_doc(PathBuf::from("./docs/branch.md"));

    // Get full document list
    let docs = tree.get_docs(&branch_id);

    // Should contain: root delta + cached prefix + branch delta
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0], PathBuf::from("./docs/root.md"));
    assert_eq!(docs[1], PathBuf::from("./cache/shared.md"));
    assert_eq!(docs[2], PathBuf::from("./docs/branch.md"));

    Ok(())
}

/// Test multi-level context hierarchy
#[test]
fn test_multi_level_hierarchy() -> Result<()> {
    let mut tree = ContextTree::new();

    // Level 0: Root
    let root = tree.init_root();
    let root_id = root.node_id.clone();
    tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/level0.md"));

    // Level 1: Fork into 2 branches
    let bp1 = tree.branch(&root_id, "task-1", 2, None)?;
    let level1_id = bp1.branches[0].clone();
    tree.get_mut(&level1_id).unwrap().add_doc(PathBuf::from("./docs/level1.md"));

    // Level 2: Fork from level 1
    let bp2 = tree.branch(&level1_id, "task-2", 2, None)?;
    let level2_id = bp2.branches[0].clone();
    tree.get_mut(&level2_id).unwrap().add_doc(PathBuf::from("./docs/level2.md"));

    // Verify ancestor chain
    let ancestors = tree.ancestors(&level2_id);
    assert_eq!(ancestors.len(), 3);
    assert_eq!(ancestors[0].node_id, level2_id);
    assert_eq!(ancestors[1].node_id, level1_id);
    assert_eq!(ancestors[2].node_id, root_id);

    // Verify document inheritance
    let docs = tree.get_docs(&level2_id);
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0], PathBuf::from("./docs/level0.md"));
    assert_eq!(docs[1], PathBuf::from("./docs/level1.md"));
    assert_eq!(docs[2], PathBuf::from("./docs/level2.md"));

    Ok(())
}

/// Test branch completion tracking
#[test]
fn test_branch_completion_tracking() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Fork into 3 branches
    let bp = tree.branch(&root_id, "ctx-fill", 3, None)?;
    let branch_id = bp.id.clone();

    // Initially not complete
    assert!(!tree.is_branch_complete(&branch_id));

    // Complete first branch
    tree.get_mut(&bp.branches[0]).unwrap().complete();
    assert!(!tree.is_branch_complete(&branch_id));

    // Complete second branch
    tree.get_mut(&bp.branches[1]).unwrap().complete();
    assert!(!tree.is_branch_complete(&branch_id));

    // Complete third branch
    tree.get_mut(&bp.branches[2]).unwrap().complete();
    assert!(tree.is_branch_complete(&branch_id));

    // Verify active contexts decreased
    assert_eq!(tree.active_contexts().len(), 1); // Only root

    Ok(())
}

/// Test branch merging
#[test]
fn test_branch_merge() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Fork into 2 branches
    let bp = tree.branch(&root_id, "ctx-fill", 2, None)?;
    let branch1_id = bp.branches[0].clone();
    let branch2_id = bp.branches[1].clone();

    // Create merge target
    let merge_id = "ctx-merge".to_string();

    // Merge branches
    tree.get_mut(&branch1_id).unwrap().merge_into(&merge_id);
    tree.get_mut(&branch2_id).unwrap().merge_into(&merge_id);

    // Verify merge status
    let branch1 = tree.get(&branch1_id).unwrap();
    assert_eq!(branch1.status, ContextStatus::Merged { into: merge_id.clone() });

    let branch2 = tree.get(&branch2_id).unwrap();
    assert_eq!(branch2.status, ContextStatus::Merged { into: merge_id.clone() });

    // Both merged branches should count as complete for branch tracking
    assert!(tree.is_branch_complete(&bp.id));

    Ok(())
}

/// Test complex fork/join workflow
#[test]
fn test_complex_fork_join_workflow() -> Result<()> {
    let mut tree = ContextTree::new();

    // Setup: root -> fork(3) -> join
    let root = tree.init_root();
    let root_id = root.node_id.clone();
    tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/spec.md"));

    // Fork into 3 parallel branches
    let bp1 = tree.branch_with_ids(
        &root_id,
        "ctx-fill",
        &["branch-a", "branch-b", "branch-c"],
        Some(PathBuf::from("./cache/shared.md"))
    )?;

    // Add work to each branch
    tree.get_mut("ctx-branch-a").unwrap().add_doc(PathBuf::from("./docs/work-a.md"));
    tree.get_mut("ctx-branch-b").unwrap().add_doc(PathBuf::from("./docs/work-b.md"));
    tree.get_mut("ctx-branch-c").unwrap().add_doc(PathBuf::from("./docs/work-c.md"));

    // Verify document lists
    let docs_a = tree.get_docs("ctx-branch-a");
    assert_eq!(docs_a.len(), 3); // spec.md + shared.md + work-a.md
    assert!(docs_a.contains(&PathBuf::from("./docs/spec.md")));
    assert!(docs_a.contains(&PathBuf::from("./cache/shared.md")));
    assert!(docs_a.contains(&PathBuf::from("./docs/work-a.md")));

    // Complete all branches
    tree.get_mut("ctx-branch-a").unwrap().complete();
    tree.get_mut("ctx-branch-b").unwrap().complete();
    tree.get_mut("ctx-branch-c").unwrap().complete();

    // Verify branch completion
    assert!(tree.is_branch_complete(&bp1.id));

    Ok(())
}

/// Test diamond-shaped workflow (fork -> parallel forks -> join)
#[test]
fn test_diamond_workflow() -> Result<()> {
    let mut tree = ContextTree::new();

    // Root
    let root = tree.init_root();
    let root_id = root.node_id.clone();
    tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/root.md"));

    // Level 1: Fork into 2
    let bp1 = tree.branch_with_ids(&root_id, "task-1", &["branch-1", "branch-2"], None)?;
    let branch1_id = "ctx-branch-1".to_string();
    let branch2_id = "ctx-branch-2".to_string();

    tree.get_mut(&branch1_id).unwrap().add_doc(PathBuf::from("./docs/b1.md"));
    tree.get_mut(&branch2_id).unwrap().add_doc(PathBuf::from("./docs/b2.md"));

    // Level 2: Fork each branch into 2 sub-branches
    let bp2a = tree.branch_with_ids(&branch1_id, "task-2a", &["sub-1a", "sub-1b"], None)?;
    let bp2b = tree.branch_with_ids(&branch2_id, "task-2b", &["sub-2a", "sub-2b"], None)?;

    // Add work to sub-branches
    tree.get_mut("ctx-sub-1a").unwrap().add_doc(PathBuf::from("./docs/s1a.md"));
    tree.get_mut("ctx-sub-1b").unwrap().add_doc(PathBuf::from("./docs/s1b.md"));
    tree.get_mut("ctx-sub-2a").unwrap().add_doc(PathBuf::from("./docs/s2a.md"));
    tree.get_mut("ctx-sub-2b").unwrap().add_doc(PathBuf::from("./docs/s2b.md"));

    // Verify document inheritance for deepest node
    let docs = tree.get_docs("ctx-sub-1a");
    assert_eq!(docs.len(), 3); // root.md + b1.md + s1a.md
    assert_eq!(docs[0], PathBuf::from("./docs/root.md"));
    assert_eq!(docs[1], PathBuf::from("./docs/b1.md"));
    assert_eq!(docs[2], PathBuf::from("./docs/s1a.md"));

    // Complete all sub-branches
    tree.get_mut("ctx-sub-1a").unwrap().complete();
    tree.get_mut("ctx-sub-1b").unwrap().complete();
    assert!(tree.is_branch_complete(&bp2a.id));

    tree.get_mut("ctx-sub-2a").unwrap().complete();
    tree.get_mut("ctx-sub-2b").unwrap().complete();
    assert!(tree.is_branch_complete(&bp2b.id));

    // Now complete parent branches
    tree.get_mut(&branch1_id).unwrap().complete();
    tree.get_mut(&branch2_id).unwrap().complete();
    assert!(tree.is_branch_complete(&bp1.id));

    Ok(())
}

/// Test abandoned contexts
#[test]
fn test_abandoned_contexts() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    let bp = tree.branch(&root_id, "ctx-fill", 2, None)?;
    let branch1_id = bp.branches[0].clone();
    let branch2_id = bp.branches[1].clone();

    // Abandon first branch
    tree.get_mut(&branch1_id).unwrap().abandon();

    // Not active anymore
    assert!(!tree.get(&branch1_id).unwrap().is_active());

    // Branch not complete (one abandoned, one still pending)
    assert!(!tree.is_branch_complete(&bp.id));

    // Abandon second branch too
    tree.get_mut(&branch2_id).unwrap().abandon();

    // Still not considered complete (abandoned != completed)
    assert!(!tree.is_branch_complete(&bp.id));

    Ok(())
}

/// Test serialization and deserialization
#[test]
fn test_context_tree_serialization() -> Result<()> {
    let mut tree = ContextTree::new();

    // Build a tree
    let root = tree.init_root();
    let root_id = root.node_id.clone();
    tree.get_mut(&root_id).unwrap().add_doc(PathBuf::from("./docs/spec.md"));

    let bp = tree.branch(&root_id, "ctx-fill", 2, Some(PathBuf::from("./cache/shared.md")))?;
    tree.get_mut(&bp.branches[0]).unwrap().add_doc(PathBuf::from("./docs/branch1.md"));

    // Serialize
    let state = tree.to_state();

    // Verify state
    assert_eq!(state.nodes.len(), 3); // root + 2 branches
    assert_eq!(state.branch_points.len(), 1);
    assert!(state.root.is_some());

    // Deserialize
    let restored = ContextTree::from_state(state);

    // Verify restored tree
    assert_eq!(restored.active_contexts().len(), 3);
    assert!(restored.root().is_some());

    let restored_bp = restored.get_branch_point(&bp.id).unwrap();
    assert_eq!(restored_bp.branches.len(), 2);

    Ok(())
}

/// Test getting children of a context
#[test]
fn test_get_children() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // No children initially
    assert_eq!(tree.children(&root_id).len(), 0);

    // Fork into 4 branches
    tree.branch(&root_id, "ctx-fill", 4, None)?;

    // Should have 4 children
    let children = tree.children(&root_id);
    assert_eq!(children.len(), 4);

    // All children should have root as parent
    for child in children {
        assert_eq!(child.parent.as_ref().unwrap(), &root_id);
    }

    Ok(())
}

/// Test concurrent branch tracking
#[test]
fn test_concurrent_branch_tracking() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Create multiple fork points
    let bp1 = tree.branch_with_ids(&root_id, "task-1", &["a1", "a2"], None)?;

    let a1_id = "ctx-a1".to_string();
    let bp2 = tree.branch_with_ids(&a1_id, "task-2", &["b1", "b2", "b3"], None)?;

    // Track each branch point independently
    assert!(!tree.is_branch_complete(&bp1.id));
    assert!(!tree.is_branch_complete(&bp2.id));

    // Complete bp2 branches
    tree.get_mut("ctx-b1").unwrap().complete();
    tree.get_mut("ctx-b2").unwrap().complete();
    tree.get_mut("ctx-b3").unwrap().complete();

    assert!(tree.is_branch_complete(&bp2.id));
    assert!(!tree.is_branch_complete(&bp1.id)); // Still waiting on a1, a2

    // Complete bp1 branches
    tree.get_mut("ctx-a1").unwrap().complete();
    tree.get_mut("ctx-a2").unwrap().complete();

    assert!(tree.is_branch_complete(&bp1.id));

    Ok(())
}

/// Test edge case: empty context tree
#[test]
fn test_empty_tree() {
    let tree = ContextTree::new();

    assert!(tree.root().is_none());
    assert_eq!(tree.active_contexts().len(), 0);
    assert_eq!(tree.children("nonexistent").len(), 0);
    assert_eq!(tree.ancestors("nonexistent").len(), 0);
    assert!(tree.get("nonexistent").is_none());
}

/// Test edge case: get docs for nonexistent node
#[test]
fn test_get_docs_nonexistent() {
    let tree = ContextTree::new();
    let docs = tree.get_docs("nonexistent");
    assert_eq!(docs.len(), 0);
}

/// Test large fan-out (many branches)
#[test]
fn test_large_fanout() -> Result<()> {
    let mut tree = ContextTree::new();
    let root = tree.init_root();
    let root_id = root.node_id.clone();

    // Fork into 100 branches
    let bp = tree.branch(&root_id, "ctx-fill", 100, None)?;

    assert_eq!(bp.branches.len(), 100);
    assert_eq!(tree.active_contexts().len(), 101); // root + 100 branches
    assert_eq!(tree.children(&root_id).len(), 100);

    // Complete all branches
    for branch_id in &bp.branches {
        tree.get_mut(branch_id).unwrap().complete();
    }

    assert!(tree.is_branch_complete(&bp.id));
    assert_eq!(tree.active_contexts().len(), 1); // Only root

    Ok(())
}
