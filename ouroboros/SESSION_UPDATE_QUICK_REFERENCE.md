# Session Update Testing - Quick Reference

## 🚀 Quick Start

### Run All Tests
```bash
./manual_session_update_test.sh
```

### Run Specific Test Suites
```bash
# Integration tests only
cargo test --test session_update_test

# Unit tests only
cargo test work_session::tests

# All session-related tests
cargo test session
```

## 📋 Test Overview

### Integration Tests (9 tests)
```
✅ test_update_session_in_index_basic
✅ test_update_session_in_index_completion
✅ test_update_session_in_index_failure
✅ test_update_session_in_index_multiple_sessions
✅ test_update_session_in_index_persistence
✅ test_update_session_in_index_incremental
✅ test_update_session_in_index_with_label
✅ test_update_session_nonexistent
✅ test_update_session_current_tracking
```

### Unit Tests (5 tests)
```
✅ test_create_session
✅ test_session_with_label
✅ test_session_lifecycle
✅ test_session_failure
✅ test_switch_session
```

## 📖 Key Functions Tested

### `update_session_in_index(&self, session: &WorkSession) -> Result<()>`

**Purpose:** Update session state in both the index file and session metadata file

**Usage:**
```rust
let mut session = mgr.create_session("My task", None)?;
session.start(5);
session.record_completion(true);
mgr.update_session_in_index(&session)?;
```

**Tested Scenarios:**
- ✅ Basic state updates
- ✅ Completion tracking
- ✅ Failure tracking
- ✅ Multiple concurrent sessions
- ✅ Persistence across manager instances
- ✅ Incremental updates
- ✅ Sessions with labels
- ✅ Edge cases (orphan sessions)

## 🔍 Session State Flow

```
┌─────────┐
│ Pending │ (Initial state)
└────┬────┘
     │ .start(task_count)
     ▼
┌─────────┐
│ Running │ (Tasks in progress)
└────┬────┘
     │
     ├─ All tasks succeed ──► ┌───────────┐
     │                         │ Completed │
     │                         └───────────┘
     │
     └─ Any task fails ─────► ┌────────┐
                               │ Failed │
                               └────────┘
```

## 📊 Test Coverage Matrix

| Feature | Integration | Unit | Manual |
|---------|-------------|------|--------|
| Session Creation | ✅ | ✅ | ✅ |
| State Transitions | ✅ | ✅ | ✅ |
| Index Updates | ✅ | ❌ | ✅ |
| Persistence | ✅ | ❌ | ✅ |
| Multiple Sessions | ✅ | ✅ | ✅ |
| Error Handling | ✅ | ✅ | ✅ |
| Edge Cases | ✅ | ❌ | ✅ |

## 🎯 Common Testing Patterns

### Test Session Creation
```rust
let tmp = TempDir::new()?;
let mgr = WorkSessionManager::new(tmp.path())?;
let session = mgr.create_session("Test goal", None)?;
```

### Test State Updates
```rust
let mut session = mgr.create_session("Test", None)?;
session.start(3);
session.record_completion(true);
mgr.update_session_in_index(&session)?;
```

### Test Persistence
```rust
// Create and update in first instance
{
    let mgr = WorkSessionManager::new(tmp.path())?;
    let mut session = mgr.create_session("Test", None)?;
    session.start(5);
    mgr.update_session_in_index(&session)?;
}

// Verify in second instance
{
    let mgr2 = WorkSessionManager::new(tmp.path())?;
    let loaded = mgr2.load_session(&session_id)?;
    assert_eq!(loaded.task_count, 5);
}
```

## 🔧 Debugging Tips

### Run with verbose output
```bash
cargo test --test session_update_test -- --nocapture --show-output
```

### Run with backtrace
```bash
RUST_BACKTRACE=1 cargo test --test session_update_test
```

### Run specific test
```bash
cargo test --test session_update_test test_update_session_in_index_basic
```

### Check test list
```bash
cargo test --test session_update_test -- --list
```

## 📁 File Structure

```
ouroboros/
├── src/
│   └── work_session.rs              # Implementation + unit tests
├── tests/
│   └── session_update_test.rs       # Integration tests
├── manual_session_update_test.sh    # Test runner script
├── SESSION_UPDATE_TEST_REPORT.md    # Detailed English report
├── SESSION_UPDATE_TEST_SUMMARY.md   # Korean summary
└── SESSION_UPDATE_QUICK_REFERENCE.md # This file
```

## ✅ Verification Checklist

Before considering testing complete:

- [x] All integration tests pass
- [x] All unit tests pass
- [x] Test coverage is comprehensive
- [x] Edge cases are tested
- [x] Documentation is complete
- [x] Manual test script works
- [x] Performance is acceptable (< 500ms)

## 🎓 Example Test Output

```
running 9 tests
test test_update_session_in_index_basic ... ok
test test_update_session_in_index_completion ... ok
test test_update_session_in_index_failure ... ok
test test_update_session_in_index_multiple_sessions ... ok
test test_update_session_in_index_persistence ... ok
test test_update_session_in_index_incremental ... ok
test test_update_session_in_index_with_label ... ok
test test_update_session_nonexistent ... ok
test test_update_session_current_tracking ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured
```

## 📚 Additional Resources

- **Detailed Report:** `SESSION_UPDATE_TEST_REPORT.md`
- **Korean Summary:** `SESSION_UPDATE_TEST_SUMMARY.md`
- **Source Code:** `src/work_session.rs`
- **Test Code:** `tests/session_update_test.rs`

## 💡 Quick Tips

1. **Run tests frequently:** `cargo test` after any changes
2. **Use descriptive test names:** Makes debugging easier
3. **Test one thing at a time:** Easier to isolate failures
4. **Clean up resources:** Use `TempDir` for file-based tests
5. **Check both success and failure paths:** Comprehensive coverage

---

**Last Updated:** 2025-02-05
**Test Status:** ✅ All tests passing (14/14)
**Production Ready:** Yes
