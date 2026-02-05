//! Integration tests for session update functionality
//!
//! Tests the complete workflow of updating session state through the index.

use ouroboros::work_session::{WorkSession, WorkSessionManager, WorkSessionStatus};
use tempfile::TempDir;
use anyhow::Result;

#[test]
fn test_update_session_in_index_basic() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create a session
    let mut session = mgr.create_session("Test goal", None)?;
    let original_id = session.id.clone();

    // Verify initial state
    assert_eq!(session.status, WorkSessionStatus::Pending);
    assert_eq!(session.task_count, 0);
    assert_eq!(session.completed_count, 0);

    // Update session state
    session.start(5);
    session.record_completion(true);
    session.record_completion(true);

    // Update in index
    mgr.update_session_in_index(&session)?;

    // Reload and verify
    let loaded = mgr.load_session(&original_id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Running);
    assert_eq!(loaded.task_count, 5);
    assert_eq!(loaded.completed_count, 2);

    Ok(())
}

#[test]
fn test_update_session_in_index_completion() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create and start a session
    let mut session = mgr.create_session("Complete all tasks", None)?;
    session.start(3);
    mgr.update_session_in_index(&session)?;

    // Complete all tasks
    session.record_completion(true);
    session.record_completion(true);
    session.record_completion(true);

    // Update in index
    mgr.update_session_in_index(&session)?;

    // Verify session is completed
    let loaded = mgr.load_session(&session.id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Completed);
    assert_eq!(loaded.completed_count, 3);
    assert!(loaded.completed_at.is_some());

    Ok(())
}

#[test]
fn test_update_session_in_index_failure() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create and start a session
    let mut session = mgr.create_session("Task with failures", None)?;
    session.start(3);
    mgr.update_session_in_index(&session)?;

    // Complete with some failures
    session.record_completion(true);
    session.record_completion(false);
    session.record_completion(true);

    // Update in index
    mgr.update_session_in_index(&session)?;

    // Verify session failed
    let loaded = mgr.load_session(&session.id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Failed);
    assert_eq!(loaded.completed_count, 2);
    assert_eq!(loaded.failed_count, 1);

    Ok(())
}

#[test]
fn test_update_session_in_index_multiple_sessions() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create multiple sessions
    let mut s1 = mgr.create_session("First session", Some("first".to_string()))?;
    let mut s2 = mgr.create_session("Second session", Some("second".to_string()))?;
    let mut s3 = mgr.create_session("Third session", Some("third".to_string()))?;

    // Update all sessions with different states
    s1.start(2);
    s1.record_completion(true);
    s1.record_completion(true);
    mgr.update_session_in_index(&s1)?;

    s2.start(3);
    s2.record_completion(true);
    mgr.update_session_in_index(&s2)?;

    s3.start(1);
    s3.record_completion(false);
    mgr.update_session_in_index(&s3)?;

    // Verify all sessions have correct states
    let loaded_s1 = mgr.load_session(&s1.id)?;
    assert_eq!(loaded_s1.status, WorkSessionStatus::Completed);

    let loaded_s2 = mgr.load_session(&s2.id)?;
    assert_eq!(loaded_s2.status, WorkSessionStatus::Running);

    let loaded_s3 = mgr.load_session(&s3.id)?;
    assert_eq!(loaded_s3.status, WorkSessionStatus::Failed);

    // Verify index contains all sessions
    let index = mgr.load_index()?;
    assert_eq!(index.sessions.len(), 3);

    Ok(())
}

#[test]
fn test_update_session_in_index_persistence() -> Result<()> {
    let tmp = TempDir::new()?;

    // Create session in first manager instance
    let session_id = {
        let mgr = WorkSessionManager::new(tmp.path())?;
        let mut session = mgr.create_session("Persistent test", None)?;
        session.start(10);
        session.record_completion(true);
        session.record_completion(true);
        session.record_completion(true);
        mgr.update_session_in_index(&session)?;
        session.id.clone()
    };

    // Create new manager instance and verify persistence
    let mgr2 = WorkSessionManager::new(tmp.path())?;
    let loaded = mgr2.load_session(&session_id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Running);
    assert_eq!(loaded.task_count, 10);
    assert_eq!(loaded.completed_count, 3);

    Ok(())
}

#[test]
fn test_update_session_in_index_incremental() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create a session
    let mut session = mgr.create_session("Incremental updates", None)?;
    session.start(5);
    mgr.update_session_in_index(&session)?;

    // Update incrementally
    for i in 0..5 {
        session.record_completion(true);
        mgr.update_session_in_index(&session)?;

        // Verify after each update
        let loaded = mgr.load_session(&session.id)?;
        assert_eq!(loaded.completed_count, i + 1);

        if i < 4 {
            assert_eq!(loaded.status, WorkSessionStatus::Running);
        } else {
            assert_eq!(loaded.status, WorkSessionStatus::Completed);
        }
    }

    Ok(())
}

#[test]
fn test_update_session_in_index_with_label() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create a session with label
    let mut session = mgr.create_session(
        "Session with label",
        Some("my-session".to_string())
    )?;

    assert!(session.id.contains("my-session"));
    assert_eq!(session.label, Some("my-session".to_string()));

    // Update session
    session.start(3);
    session.record_completion(true);
    mgr.update_session_in_index(&session)?;

    // Verify label is preserved
    let loaded = mgr.load_session(&session.id)?;
    assert_eq!(loaded.label, Some("my-session".to_string()));
    assert_eq!(loaded.completed_count, 1);

    Ok(())
}

#[test]
fn test_update_session_nonexistent() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create a session but don't add it to index properly
    let orphan_session = WorkSession::new("Orphan", None, 999);

    // Create the session directory structure (but not in index)
    let session_dir = mgr.session_dir(&orphan_session.id);
    std::fs::create_dir_all(&session_dir)?;

    // Attempting to update a session not in index should succeed
    // (it saves session files but doesn't update index entry)
    let result = mgr.update_session_in_index(&orphan_session);

    // This should succeed (saves to file but doesn't modify index)
    assert!(result.is_ok());

    // Verify index doesn't contain the orphan (since it wasn't added to index)
    let index = mgr.load_index()?;
    assert!(!index.sessions.iter().any(|s| s.id == orphan_session.id));

    // But the session file should exist
    assert!(session_dir.join("session.json").exists());

    Ok(())
}

#[test]
fn test_update_session_current_tracking() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create first session
    let mut s1 = mgr.create_session("First", None)?;
    let s1_id = s1.id.clone();

    // Verify it's current
    assert_eq!(mgr.current_session_id()?, Some(s1_id.clone()));

    // Create second session
    let mut s2 = mgr.create_session("Second", None)?;
    let s2_id = s2.id.clone();

    // Current should change to s2
    assert_eq!(mgr.current_session_id()?, Some(s2_id.clone()));

    // Update s1 (not current)
    s1.start(2);
    mgr.update_session_in_index(&s1)?;

    // Current should still be s2
    assert_eq!(mgr.current_session_id()?, Some(s2_id.clone()));

    // Update s2 (current)
    s2.start(3);
    mgr.update_session_in_index(&s2)?;

    // Current should still be s2
    assert_eq!(mgr.current_session_id()?, Some(s2_id));

    Ok(())
}
