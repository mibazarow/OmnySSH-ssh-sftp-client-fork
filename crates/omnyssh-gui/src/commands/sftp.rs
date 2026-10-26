//! SFTP session commands (tech-gui.md §4.2). `sftp_open` awaits the core connect and
//! spawns a per-session forwarder that stamps every `sftp-*` event with the tab's
//! session id (§3.4); the remote ops are thin `SftpCommand` enqueues whose results
//! arrive as events. Local filesystem listing/preview return directly — the GUI never
//! emits `LocalDirListed` (§4.3).

use tauri::{AppHandle, State};
use tokio::sync::mpsc;

use omnyssh_core::event::CoreEvent;
use omnyssh_core::ssh::identity;
use omnyssh_core::ssh::password::Prompter;
use omnyssh_core::ssh::sftp::{
    list_local_dir as core_list_local_dir, local_roots,
    preview_local_file as core_preview_local_file, SftpCommand, SftpManager,
};

use crate::bridge;
use crate::dto::{FileEntryDto, FileMetaDto, FileTextDto};
use crate::error::CommandError;
use crate::state::GuiState;

/// A tab's dedicated core-event channel buffer. Comfortably absorbs the connect ack
/// plus a burst of transfer-progress ticks before the forwarder drains them (§3.4).
const SFTP_EVENT_BUFFER: usize = 256;

/// Open an SFTP session for `host_name` (tech-gui.md §4.2). Awaits the core connect,
/// registers the manager under a fresh public id, and spawns the per-session
/// forwarder; the `sftp-connected` ack then arrives stamped with that id (§3.4).
#[tauri::command]
#[specta::specta]
pub async fn sftp_open(
    app: AppHandle,
    state: State<'_, GuiState>,
    host_name: String,
) -> Result<u64, CommandError> {
    let host = state.host_by_name(&host_name).ok_or_else(|| CommandError {
        message: format!("unknown host '{host_name}'"),
    })?;
    // A dedicated channel per tab: its owner is the session id, so the forwarder can
    // attribute the core's session-less `sftp-*` events to this tab (§3.4).
    let (tx, rx) = mpsc::channel::<CoreEvent>(SFTP_EVENT_BUFFER);
    // The prompt goes out on the engine channel: this tab's own channel only
    // carries `sftp-*` events.
    let prompter = Prompter::new(state.engine_sender(), &host_name);
    let manager = match SftpManager::connect(&host, tx, prompter).await {
        Ok(manager) => manager,
        Err(e) => {
            if let Some(path) = omnyssh_core::ssh::session::passphrase_required(&e) {
                identity::ask_passphrase(&state.engine_sender(), &host_name, path).await;
            }
            // The whole chain: the core wraps the cause in "SFTP SSH connect".
            return Err(CommandError {
                message: format!("{e:#}"),
            });
        }
    };
    let session_id = state.register_sftp(manager);
    tauri::async_runtime::spawn(bridge::forward_sftp_events(app, session_id, rx));
    Ok(session_id)
}

/// List a remote directory (tech-gui.md §4.2); the result arrives as `sftp-dir-listed`.
#[tauri::command]
#[specta::specta]
pub fn sftp_list(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
) -> Result<(), CommandError> {
    state.send_sftp(session_id, SftpCommand::ListDir(path));
    Ok(())
}

/// Upload a local file or folder to a remote path (tech-gui.md §4.2). Allocates a
/// transfer id owned by this session so `transfer-progress` routes back to the tab
/// (§3.4).
#[tauri::command]
#[specta::specta]
pub fn sftp_upload(
    state: State<'_, GuiState>,
    session_id: u64,
    local: String,
    remote: String,
) -> Result<(), CommandError> {
    let transfer_id = state.next_transfer(session_id);
    state.send_sftp(
        session_id,
        SftpCommand::Upload {
            local,
            remote,
            transfer_id,
        },
    );
    Ok(())
}

/// Download a remote file or folder to a local path (tech-gui.md §4.2). See
/// `sftp_upload` for the transfer-id routing. `local` ends in the name the server
/// listed, which the frontend checks is one plain name; the core keeps every name in
/// a folder inside the destination (§3.2).
#[tauri::command]
#[specta::specta]
pub fn sftp_download(
    state: State<'_, GuiState>,
    session_id: u64,
    local: String,
    remote: String,
) -> Result<(), CommandError> {
    let transfer_id = state.next_transfer(session_id);
    state.send_sftp(
        session_id,
        SftpCommand::Download {
            remote,
            local,
            transfer_id,
        },
    );
    Ok(())
}

/// Create a remote directory (tech-gui.md §4.2); completion arrives as `sftp-op-done`.
#[tauri::command]
#[specta::specta]
pub fn sftp_mkdir(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
) -> Result<(), CommandError> {
    state.send_sftp(session_id, SftpCommand::MkDir(path));
    Ok(())
}

/// Rename / move a remote path (tech-gui.md §4.2).
#[tauri::command]
#[specta::specta]
pub fn sftp_rename(
    state: State<'_, GuiState>,
    session_id: u64,
    from: String,
    to: String,
) -> Result<(), CommandError> {
    state.send_sftp(session_id, SftpCommand::Rename { from, to });
    Ok(())
}

/// Delete a remote file (falls back to an empty directory in the core) (tech-gui.md §4.2).
#[tauri::command]
#[specta::specta]
pub fn sftp_delete(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
) -> Result<(), CommandError> {
    state.send_sftp(session_id, SftpCommand::Delete(path));
    Ok(())
}

/// Read a remote file's preview bytes (tech-gui.md §4.2); arrives as `file-preview`.
#[tauri::command]
#[specta::specta]
pub fn sftp_preview(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
) -> Result<(), CommandError> {
    state.send_sftp(session_id, SftpCommand::ReadPreview(path));
    Ok(())
}

/// The error of a command aimed at an SFTP session that is gone.
fn unknown_session(session_id: u64) -> CommandError {
    CommandError {
        message: format!("SFTP session {session_id} is not open"),
    }
}

/// Read a remote file whole for the editor (tech-gui.md §4.2). The whole chain of
/// reasons — binary, not UTF-8, over the editor's limit — is the useful part.
#[tauri::command]
#[specta::specta]
pub async fn sftp_read_file(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
) -> Result<FileTextDto, CommandError> {
    let manager = state
        .sftp_manager(session_id)
        .ok_or_else(|| unknown_session(session_id))?;
    let file = manager
        .read_text_file(path)
        .await
        .map_err(|e| CommandError {
            message: format!("{e:#}"),
        })?;
    Ok(FileTextDto {
        content: file.content,
        mode: file.mode,
    })
}

/// Replace a remote file's contents, keeping its permission bits (tech-gui.md §4.2).
#[tauri::command]
#[specta::specta]
pub async fn sftp_write_file(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
    content: String,
) -> Result<(), CommandError> {
    let manager = state
        .sftp_manager(session_id)
        .ok_or_else(|| unknown_session(session_id))?;
    manager
        .write_text_file(path, content)
        .await
        .map_err(|e| CommandError {
            message: format!("{e:#}"),
        })
}

/// Read a remote path's metadata for the properties dialog (tech-gui.md §4.2).
#[tauri::command]
#[specta::specta]
pub async fn sftp_stat(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
) -> Result<FileMetaDto, CommandError> {
    let manager = state
        .sftp_manager(session_id)
        .ok_or_else(|| unknown_session(session_id))?;
    let meta = manager.stat(path).await.map_err(|e| CommandError {
        message: format!("{e:#}"),
    })?;
    Ok(meta.into())
}

/// Set a remote path's permission bits (`0o7777`; the properties dialog sends the
/// setuid/setgid/sticky ones back unchanged).
#[tauri::command]
#[specta::specta]
pub async fn sftp_chmod(
    state: State<'_, GuiState>,
    session_id: u64,
    path: String,
    mode: u32,
) -> Result<(), CommandError> {
    let manager = state
        .sftp_manager(session_id)
        .ok_or_else(|| unknown_session(session_id))?;
    manager
        .set_mode(path, mode)
        .await
        .map_err(|e| CommandError {
            message: format!("{e:#}"),
        })
}

/// Rename a local path (tech-gui.md §4.2). The frontend builds `to` from one plain
/// name inside the entry's own folder.
#[tauri::command]
#[specta::specta]
pub async fn rename_local_path(from: String, to: String) -> Result<(), CommandError> {
    tokio::fs::rename(&from, &to)
        .await
        .map_err(|e| CommandError {
            message: format!("rename '{from}': {e}"),
        })
}

/// Cancel the transfer this session is running (tech-gui.md §4.2). It stops at its
/// next step, and its `sftp-op-done` says it was cancelled.
#[tauri::command]
#[specta::specta]
pub fn sftp_cancel(state: State<'_, GuiState>, session_id: u64) -> Result<(), CommandError> {
    state.cancel_sftp(session_id);
    Ok(())
}

/// Close an SFTP session and its connection, stopping its transfer (tech-gui.md §4.2).
#[tauri::command]
#[specta::specta]
pub fn sftp_close(state: State<'_, GuiState>, session_id: u64) -> Result<(), CommandError> {
    state.close_sftp(session_id);
    Ok(())
}

/// List a local directory (tech-gui.md §4.2). Returns directly off the async worker;
/// the GUI never emits `LocalDirListed` (§4.3). The core prepends a `..` entry and
/// sorts dirs-first.
#[tauri::command]
#[specta::specta]
pub async fn list_local_dir(path: String) -> Result<Vec<FileEntryDto>, CommandError> {
    // The whole chain: the OS reason ("Access is denied") is the useful part.
    let entries = core_list_local_dir(&path).await.map_err(|e| CommandError {
        message: format!("{e:#}"),
    })?;
    Ok(entries.iter().map(FileEntryDto::from).collect())
}

/// The roots the local pane can switch to: every drive letter on Windows, `/`
/// elsewhere (tech-gui.md §4.2).
#[tauri::command]
#[specta::specta]
pub fn list_local_roots() -> Vec<String> {
    local_roots()
}

/// Read up to 4 KiB of a local file as UTF-8 for preview (tech-gui.md §4.2).
#[tauri::command]
#[specta::specta]
pub async fn preview_local_file(path: String) -> Result<String, CommandError> {
    core_preview_local_file(&path)
        .await
        .map_err(|e| CommandError {
            message: e.to_string(),
        })
}
