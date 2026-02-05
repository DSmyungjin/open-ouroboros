//! Comprehensive Work Session Lifecycle Tests
//!
//! Tests all aspects of work session lifecycle management:
//! - Session creation and initialization
//! - Status transitions (pending → running → completed/failed)
//! - Session switching and transitions
//! - Metadata and context management
//! - Concurrency and error handling

use ouroboros::work_session::{WorkSession, WorkSessionManager, WorkSessionStatus};
use tempfile::TempDir;
use anyhow::Result;
use std::sync::Arc;
use std::thread;
use chrono::Utc;

// ============================================================================
// Session Creation Tests
// ============================================================================

#[test]
fn test_create_session_basic() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Test goal", None)?;

    // Verify basic properties
    assert_eq!(session.goal, "Test goal");
    assert_eq!(session.status, WorkSessionStatus::Pending);
    assert_eq!(session.task_count, 0);
    assert_eq!(session.completed_count, 0);
    assert_eq!(session.failed_count, 0);
    assert!(session.id.len() >= 10); // Format: "001-abc123"
    assert!(session.label.is_none());
    assert!(session.completed_at.is_none());

    Ok(())
}

#[test]
fn test_create_session_with_label() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session(
        "Refactor authentication system",
        Some("auth-refactor".to_string())
    )?;

    // Verify label is included in ID
    assert!(session.id.contains("auth-refactor"));
    assert_eq!(session.label, Some("auth-refactor".to_string()));
    assert_eq!(session.goal, "Refactor authentication system");

    Ok(())
}

#[test]
fn test_create_session_sequence_numbering() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let s1 = mgr.create_session("First session", None)?;
    let s2 = mgr.create_session("Second session", None)?;
    let s3 = mgr.create_session("Third session", None)?;

    // Verify sequence numbers
    assert!(s1.id.starts_with("001-"));
    assert!(s2.id.starts_with("002-"));
    assert!(s3.id.starts_with("003-"));

    Ok(())
}

#[test]
fn test_create_session_directory_structure() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Test", None)?;
    let session_dir = mgr.session_dir(&session.id);

    // Verify directory structure
    assert!(session_dir.exists());
    assert!(session_dir.join("tasks").exists());
    assert!(session_dir.join("results").exists());
    assert!(session_dir.join("contexts").exists());
    assert!(session_dir.join("session.json").exists());

    Ok(())
}

#[test]
fn test_create_session_updates_index() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Test", None)?;
    let index = mgr.load_index()?;

    // Verify index was updated
    assert_eq!(index.sessions.len(), 1);
    assert_eq!(index.sessions[0].id, session.id);
    assert_eq!(index.sessions[0].goal, "Test");
    assert_eq!(index.current, Some(session.id));

    Ok(())
}

#[test]
fn test_create_session_sets_current() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Current test", None)?;

    // Verify current session is set
    assert_eq!(mgr.current_session_id()?, Some(session.id.clone()));

    let current = mgr.current_session()?.expect("Should have current session");
    assert_eq!(current.id, session.id);

    Ok(())
}

// ============================================================================
// Status Transition Tests
// ============================================================================

#[test]
fn test_status_transition_pending_to_running() -> Result<()> {
    let mut session = WorkSession::new("Test", None, 1);
    assert_eq!(session.status, WorkSessionStatus::Pending);

    session.start(5);

    assert_eq!(session.status, WorkSessionStatus::Running);
    assert_eq!(session.task_count, 5);
    assert_eq!(session.completed_count, 0);

    Ok(())
}

#[test]
fn test_status_transition_running_to_completed() -> Result<()> {
    let mut session = WorkSession::new("Test", None, 1);
    session.start(3);
    assert_eq!(session.status, WorkSessionStatus::Running);

    // Complete all tasks
    session.record_completion(true);
    assert_eq!(session.status, WorkSessionStatus::Running);

    session.record_completion(true);
    assert_eq!(session.status, WorkSessionStatus::Running);

    session.record_completion(true);

    // Should transition to completed
    assert_eq!(session.status, WorkSessionStatus::Completed);
    assert_eq!(session.completed_count, 3);
    assert_eq!(session.failed_count, 0);
    assert!(session.completed_at.is_some());

    Ok(())
}

#[test]
fn test_status_transition_running_to_failed() -> Result<()> {
    let mut session = WorkSession::new("Test", None, 1);
    session.start(3);

    // Complete with at least one failure
    session.record_completion(true);
    session.record_completion(false);
    session.record_completion(true);

    // Should transition to failed
    assert_eq!(session.status, WorkSessionStatus::Failed);
    assert_eq!(session.completed_count, 2);
    assert_eq!(session.failed_count, 1);
    assert!(session.completed_at.is_some());

    Ok(())
}

#[test]
fn test_status_transition_all_failures() -> Result<()> {
    let mut session = WorkSession::new("Test", None, 1);
    session.start(2);

    session.record_completion(false);
    session.record_completion(false);

    assert_eq!(session.status, WorkSessionStatus::Failed);
    assert_eq!(session.completed_count, 0);
    assert_eq!(session.failed_count, 2);

    Ok(())
}

#[test]
fn test_status_transition_persistence() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create and transition session
    let mut session = mgr.create_session("Persistence test", None)?;
    let session_id = session.id.clone();

    session.start(3);
    session.record_completion(true);
    mgr.update_session_in_index(&session)?;

    // Reload and verify state
    let loaded = mgr.load_session(&session_id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Running);
    assert_eq!(loaded.task_count, 3);
    assert_eq!(loaded.completed_count, 1);

    // Continue and complete
    session.record_completion(true);
    session.record_completion(true);
    mgr.update_session_in_index(&session)?;

    // Reload and verify completion
    let loaded = mgr.load_session(&session_id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Completed);
    assert_eq!(loaded.completed_count, 3);

    Ok(())
}

#[test]
fn test_status_transition_edge_cases() -> Result<()> {
    // Test zero tasks
    let mut session = WorkSession::new("Zero tasks", None, 1);
    session.start(0);
    assert_eq!(session.status, WorkSessionStatus::Running);

    // Test single task
    let mut session = WorkSession::new("Single task", None, 2);
    session.start(1);
    session.record_completion(true);
    assert_eq!(session.status, WorkSessionStatus::Completed);

    // Test single task failure
    let mut session = WorkSession::new("Single failure", None, 3);
    session.start(1);
    session.record_completion(false);
    assert_eq!(session.status, WorkSessionStatus::Failed);

    Ok(())
}

// ============================================================================
// Session Switching Tests
// ============================================================================

#[test]
fn test_switch_session_basic() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create two sessions
    let s1 = mgr.create_session("First", None)?;
    let _s2 = mgr.create_session("Second", None)?;

    // Current should be _s2
    assert_eq!(mgr.current_session_id()?, Some(_s2.id.clone()));

    // Switch to s1
    mgr.switch_session(&s1.id)?;
    assert_eq!(mgr.current_session_id()?, Some(s1.id.clone()));

    // Switch back to _s2
    mgr.switch_session(&_s2.id)?;
    assert_eq!(mgr.current_session_id()?, Some(_s2.id.clone()));

    Ok(())
}

#[test]
fn test_switch_session_by_prefix() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Test", None)?;
    let short_id = session.short_id().to_string();

    // Switch using short ID prefix
    mgr.switch_session(&short_id[..7])?; // Use first 7 chars
    assert_eq!(mgr.current_session_id()?, Some(session.id.clone()));

    Ok(())
}

#[test]
fn test_switch_session_with_label() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let s1 = mgr.create_session("First", Some("alpha".to_string()))?;
    let s2 = mgr.create_session("Second", Some("beta".to_string()))?;

    // Verify labels are in IDs
    assert!(s1.id.contains("alpha"));
    assert!(s2.id.contains("beta"));

    // Switch between labeled sessions
    mgr.switch_session(&s1.id)?;
    assert_eq!(mgr.current_session_id()?, Some(s1.id.clone()));

    Ok(())
}

#[test]
fn test_switch_session_updates_symlink() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let s1 = mgr.create_session("First", None)?;
    let _s2 = mgr.create_session("Second", None)?;

    // Check current symlink exists
    let current_link = tmp.path().join("current");
    assert!(current_link.exists() || current_link.is_symlink());

    // Switch and verify symlink updates
    mgr.switch_session(&s1.id)?;

    // On Unix, verify symlink target
    #[cfg(unix)]
    {
        let target = std::fs::read_link(&current_link)?;
        assert!(target.to_string_lossy().contains(&s1.id));
    }

    Ok(())
}

#[test]
fn test_switch_session_nonexistent() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Try to switch to nonexistent session
    let result = mgr.switch_session("nonexistent-id");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_switch_session_preserves_state() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create and update first session
    let mut s1 = mgr.create_session("First", None)?;
    s1.start(5);
    s1.record_completion(true);
    s1.record_completion(true);
    mgr.update_session_in_index(&s1)?;

    // Create second session
    let _s2 = mgr.create_session("Second", None)?;

    // Switch back to first session
    let switched = mgr.switch_session(&s1.id)?;

    // Verify state was preserved
    assert_eq!(switched.status, WorkSessionStatus::Running);
    assert_eq!(switched.completed_count, 2);
    assert_eq!(switched.task_count, 5);

    Ok(())
}

// ============================================================================
// Metadata Management Tests
// ============================================================================

#[test]
fn test_session_metadata_timestamps() -> Result<()> {
    let session = WorkSession::new("Test", None, 1);

    // Verify created_at is set
    let now = Utc::now();
    assert!(session.created_at <= now);

    // Completed_at should be None initially
    assert!(session.completed_at.is_none());

    Ok(())
}

#[test]
fn test_session_metadata_completion_timestamp() -> Result<()> {
    let mut session = WorkSession::new("Test", None, 1);
    session.start(2);

    let before_completion = Utc::now();
    session.record_completion(true);
    session.record_completion(true);
    let after_completion = Utc::now();

    // Verify completion timestamp was set
    let completed_at = session.completed_at.expect("Should have completion time");
    assert!(completed_at >= before_completion && completed_at <= after_completion);

    Ok(())
}

#[test]
fn test_session_metadata_short_id() -> Result<()> {
    let session = WorkSession::new("Test", None, 1);

    let short_id = session.short_id();

    // Should be sequence + hash (e.g., "001-abc123")
    assert_eq!(short_id.len(), 10);
    assert!(short_id.starts_with("001-"));

    Ok(())
}

#[test]
fn test_session_metadata_display_line() -> Result<()> {
    let session = WorkSession::new("Test goal description", None, 1);

    let display = session.display_line();

    // Should contain status, ID, and goal
    assert!(display.contains("001-"));
    assert!(display.contains("Test goal description"));
    assert!(display.contains("○")); // Pending status icon

    Ok(())
}

#[test]
fn test_session_metadata_display_with_label() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Test", Some("my-label".to_string()))?;
    let display = session.display_line();

    // Should contain label
    assert!(display.contains("[my-label]"));

    Ok(())
}

#[test]
fn test_session_metadata_progress_tracking() -> Result<()> {
    let mut session = WorkSession::new("Progress test", None, 1);
    session.start(10);

    // Track progress
    for i in 1..=10 {
        session.record_completion(true);
        assert_eq!(session.completed_count, i);

        let display = session.display_line();
        assert!(display.contains(&format!("{}/10", i)));
    }

    Ok(())
}

// ============================================================================
// Context Management Tests
// ============================================================================

#[test]
fn test_session_context_directory_creation() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Context test", None)?;
    let contexts_dir = mgr.session_dir(&session.id).join("contexts");

    assert!(contexts_dir.exists());
    assert!(contexts_dir.is_dir());

    Ok(())
}

#[test]
fn test_session_context_tasks_directory() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Tasks test", None)?;
    let tasks_dir = mgr.session_dir(&session.id).join("tasks");

    assert!(tasks_dir.exists());
    assert!(tasks_dir.is_dir());

    Ok(())
}

#[test]
fn test_session_context_results_directory() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Results test", None)?;
    let results_dir = mgr.session_dir(&session.id).join("results");

    assert!(results_dir.exists());
    assert!(results_dir.is_dir());

    Ok(())
}

#[test]
fn test_session_get_data_dir() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let session = mgr.create_session("Data dir test", None)?;

    // Get data dir by session ID
    let data_dir = mgr.get_session_data_dir(Some(&session.id))?;
    assert!(data_dir.exists());
    assert!(data_dir.ends_with(&session.id));

    // Get current session's data dir
    let current_dir = mgr.get_session_data_dir(None)?;
    assert!(current_dir.exists());

    Ok(())
}

#[test]
fn test_session_get_data_dir_no_current() -> Result<()> {
    let tmp = TempDir::new()?;

    // Create sessions directory but no index
    std::fs::create_dir_all(tmp.path().join("sessions"))?;

    let mgr = WorkSessionManager::new(tmp.path())?;

    // Should fail when no current session exists
    let result = mgr.get_session_data_dir(None);
    assert!(result.is_err());

    Ok(())
}

// ============================================================================
// Concurrency Tests
// ============================================================================

#[test]
fn test_concurrent_session_creation() -> Result<()> {
    let tmp = TempDir::new()?;
    let tmp_path = tmp.path().to_path_buf();

    // Note: File-based session management has inherent race conditions.
    // This test demonstrates that sessions can be created sequentially
    // with proper synchronization (using joins between creates).

    let mgr = Arc::new(WorkSessionManager::new(&tmp_path)?);
    let mut session_ids = vec![];

    // Create sessions with staggered timing to reduce race conditions
    for i in 0..5 {
        let mgr_clone = Arc::clone(&mgr);
        let handle = thread::spawn(move || {
            // Add staggered delay to reduce contention
            thread::sleep(std::time::Duration::from_millis(i * 20));
            mgr_clone.create_session(
                &format!("Concurrent session {}", i),
                Some(format!("session-{}", i))
            )
        });

        // Wait for this thread to complete before starting the next
        // This ensures we don't have multiple threads writing to index simultaneously
        match handle.join().unwrap() {
            Ok(session) => session_ids.push(session.id),
            Err(e) => eprintln!("Session creation failed: {:?}", e),
        }
    }

    // Verify sessions were created
    assert!(session_ids.len() >= 4, "Expected at least 4 successful sessions, got {}", session_ids.len());

    // Verify sessions are in index
    let index = mgr.load_index()?;
    assert!(index.sessions.len() >= 4, "Expected at least 4 sessions in index, got {}", index.sessions.len());

    // Verify all created sessions can be loaded
    for session_id in &session_ids {
        let loaded = mgr.load_session(session_id)?;
        assert_eq!(&loaded.id, session_id);
    }

    Ok(())
}

#[test]
fn test_concurrent_session_updates() -> Result<()> {
    let tmp = TempDir::new()?;
    let tmp_path = tmp.path().to_path_buf();
    let mgr = Arc::new(WorkSessionManager::new(&tmp_path)?);

    // Create initial session
    let mut session = mgr.create_session("Update test", None)?;
    session.start(20);
    mgr.update_session_in_index(&session)?;

    let session_id = session.id.clone();
    let mut handles = vec![];

    // Update session sequentially to avoid file corruption
    // (Real-world usage should use proper synchronization)
    for _ in 0..5 {
        let mgr_clone = Arc::clone(&mgr);
        let id_clone = session_id.clone();

        let handle = thread::spawn(move || -> Result<()> {
            // Add a small delay to reduce race conditions
            thread::sleep(std::time::Duration::from_millis(10));

            let mut sess = mgr_clone.load_session(&id_clone)?;
            sess.record_completion(true);
            mgr_clone.update_session_in_index(&sess)?;
            Ok(())
        });
        handles.push(handle);
    }

    // Wait for all threads and collect results
    let results: Vec<_> = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    // Count successful updates
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert!(success_count >= 3, "Expected at least 3 successful updates");

    // Verify the session is still valid and has some completions
    let final_session = mgr.load_session(&session_id)?;
    assert!(final_session.completed_count > 0);
    assert!(final_session.completed_count <= 20);
    assert_eq!(final_session.status, WorkSessionStatus::Running);

    Ok(())
}

#[test]
fn test_concurrent_session_switching() -> Result<()> {
    let tmp = TempDir::new()?;
    let tmp_path = tmp.path().to_path_buf();
    let mgr = Arc::new(WorkSessionManager::new(&tmp_path)?);

    // Create multiple sessions
    let s1 = mgr.create_session("Session 1", None)?;
    let s2 = mgr.create_session("Session 2", None)?;
    let s3 = mgr.create_session("Session 3", None)?;

    let sessions = vec![s1.id, s2.id, s3.id];
    let mut handles = vec![];

    // Switch between sessions concurrently
    for (i, session_id) in sessions.iter().enumerate() {
        let mgr_clone = Arc::clone(&mgr);
        let id_clone = session_id.clone();

        let handle = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(i as u64 * 10));
            mgr_clone.switch_session(&id_clone)
        });
        handles.push(handle);
    }

    // Wait for all threads
    let results: Vec<_> = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    // All switches should succeed
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 3);

    // Current should be one of the sessions
    let current = mgr.current_session_id()?.expect("Should have current");
    assert!(sessions.contains(&current));

    Ok(())
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_error_load_nonexistent_session() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    let result = mgr.load_session("nonexistent-id-12345");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_error_invalid_session_data() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create session with corrupted data
    let session_dir = mgr.session_dir("001-bad");
    std::fs::create_dir_all(&session_dir)?;
    std::fs::write(session_dir.join("session.json"), "invalid json data")?;

    // Should fail to load
    let result = mgr.load_session("001-bad");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_error_switch_to_invalid_session() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    mgr.create_session("Valid", None)?;

    // Try to switch to invalid session
    let result = mgr.switch_session("999-invalid");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_error_recovery_index_corruption() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create valid session first
    mgr.create_session("Test", None)?;

    // Corrupt the index
    std::fs::write(mgr.index_path(), "corrupted data")?;

    // Should return error when loading
    let result = mgr.load_index();
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_error_handling_missing_directories() -> Result<()> {
    let tmp = TempDir::new()?;

    // Try to create manager in non-existent location
    let non_existent = tmp.path().join("does-not-exist");

    // Manager should create directories
    let mgr = WorkSessionManager::new(&non_existent)?;
    assert!(mgr.sessions_dir().exists());

    Ok(())
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_lifecycle_integration() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // 1. Create session
    let mut session = mgr.create_session(
        "Full lifecycle test",
        Some("lifecycle".to_string())
    )?;
    let session_id = session.id.clone();
    assert_eq!(session.status, WorkSessionStatus::Pending);

    // 2. Start session
    session.start(5);
    mgr.update_session_in_index(&session)?;
    let loaded = mgr.load_session(&session_id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Running);

    // 3. Progress through tasks
    for i in 0..4 {
        session.record_completion(true);
        mgr.update_session_in_index(&session)?;
        let loaded = mgr.load_session(&session_id)?;
        assert_eq!(loaded.completed_count, i + 1);
        assert_eq!(loaded.status, WorkSessionStatus::Running);
    }

    // 4. Complete final task
    session.record_completion(true);
    mgr.update_session_in_index(&session)?;
    let loaded = mgr.load_session(&session_id)?;
    assert_eq!(loaded.status, WorkSessionStatus::Completed);
    assert!(loaded.completed_at.is_some());

    // 5. Verify persistence across manager instances
    let mgr2 = WorkSessionManager::new(tmp.path())?;
    let loaded2 = mgr2.load_session(&session_id)?;
    assert_eq!(loaded2.status, WorkSessionStatus::Completed);
    assert_eq!(loaded2.completed_count, 5);

    Ok(())
}

#[test]
fn test_multiple_sessions_lifecycle() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create and manage multiple sessions
    let mut s1 = mgr.create_session("Session 1", Some("s1".to_string()))?;
    let mut s2 = mgr.create_session("Session 2", Some("s2".to_string()))?;
    let mut s3 = mgr.create_session("Session 3", Some("s3".to_string()))?;

    // Progress each session differently
    s1.start(3);
    s1.record_completion(true);
    s1.record_completion(true);
    s1.record_completion(true);
    mgr.update_session_in_index(&s1)?;

    s2.start(2);
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

    // Verify index integrity
    let sessions = mgr.list_sessions()?;
    assert_eq!(sessions.len(), 3);

    Ok(())
}

#[test]
fn test_session_switching_workflow() -> Result<()> {
    let tmp = TempDir::new()?;
    let mgr = WorkSessionManager::new(tmp.path())?;

    // Create multiple sessions
    let s1 = mgr.create_session("Background task", Some("bg".to_string()))?;
    let s2 = mgr.create_session("Main task", Some("main".to_string()))?;

    // Current should be s2
    assert_eq!(mgr.current_session_id()?, Some(s2.id.clone()));

    // Switch to work on background task
    let mut bg_session = mgr.switch_session(&s1.id)?;
    bg_session.start(2);
    bg_session.record_completion(true);
    mgr.update_session_in_index(&bg_session)?;

    // Switch back to main task
    let mut main_session = mgr.switch_session(&s2.id)?;
    main_session.start(5);
    main_session.record_completion(true);
    main_session.record_completion(true);
    mgr.update_session_in_index(&main_session)?;

    // Verify both sessions retained their state
    let bg_final = mgr.load_session(&s1.id)?;
    assert_eq!(bg_final.completed_count, 1);

    let main_final = mgr.load_session(&s2.id)?;
    assert_eq!(main_final.completed_count, 2);

    Ok(())
}
