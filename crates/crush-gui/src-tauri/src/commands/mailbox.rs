//! Mailbox commands — expose captured dev emails to the GUI.
//!
//! The SMTP sink itself is started once at app launch (see `lib.rs`); these
//! commands just read/clear the in-memory store it fills.

use crate::state::AppState;
use crush_build::mailbox::CapturedMail;
use tauri::State;

/// All captured messages, newest first.
#[tauri::command]
pub async fn list_mail(state: State<'_, AppState>) -> Result<Vec<CapturedMail>, String> {
    Ok(state.mailbox.list())
}

/// Discard all captured messages.
#[tauri::command]
pub async fn clear_mail(state: State<'_, AppState>) -> Result<(), String> {
    state.mailbox.clear();
    Ok(())
}
