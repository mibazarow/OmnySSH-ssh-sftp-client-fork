//! Application configuration modules.
//!
//! - [`app_config`]  — main `~/.config/omnyssh/config.toml`
//! - [`snippets`]    — `~/.config/omnyssh/snippets.toml`
//!
//! Top-level functions in this module handle loading and persisting the
//! host list (`hosts.toml`) — the app's own storage and the single source
//! of hosts. `~/.ssh/config` is never read.

pub mod app_config;
pub mod snippets;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::ssh::client::Host;
use crate::utils::platform;

// ---------------------------------------------------------------------------
// hosts.toml I/O
// ---------------------------------------------------------------------------

/// TOML container for the hosts list.
#[derive(Debug, Default, Serialize, Deserialize)]
struct HostsFile {
    #[serde(default)]
    hosts: Vec<Host>,
}

/// Loads the host list from `~/.config/omnyssh/hosts.toml`.
///
/// Returns an empty `Vec` if the file does not exist yet.
///
/// # Errors
/// Returns an error if the file exists but cannot be read or parsed.
pub fn load_hosts() -> anyhow::Result<Vec<Host>> {
    let path = platform::hosts_config_path().context("Cannot determine hosts config path")?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let file: HostsFile =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;

    Ok(file.hosts)
}

/// Persists the host list to `~/.config/omnyssh/hosts.toml`.
///
/// # Errors
/// Returns an error if the directory cannot be created or the file cannot
/// be written.
pub fn save_hosts(hosts: &[Host]) -> anyhow::Result<()> {
    let dir = platform::app_config_dir().context("Cannot determine app config directory")?;

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create directory {}", dir.display()))?;

    let path = dir.join("hosts.toml");

    let file = HostsFile {
        hosts: hosts.to_vec(),
    };
    let content = toml::to_string_pretty(&file).context("Failed to serialise hosts")?;

    // Write to a temp file and rename for atomic replacement (avoids a corrupt
    // hosts.toml if the process is interrupted mid-write).
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, content)
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                tmp_path.display(),
                path.display()
            )
        });
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(&path, perms);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(name: &str) -> Host {
        Host {
            name: name.to_string(),
            ..Host::default()
        }
    }

    fn names(hosts: &[Host]) -> Vec<&str> {
        hosts.iter().map(|h| h.name.as_str()).collect()
    }

    #[test]
    fn hostsfile_roundtrip_preserves_fields() {
        let mut h = host("web");
        h.hostname = "10.0.0.1".to_string();
        h.user = "deploy".to_string();
        h.port = 2222;
        h.tags = vec!["prod".to_string()];
        let toml_str = toml::to_string_pretty(&HostsFile { hosts: vec![h] }).unwrap();
        let parsed: HostsFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.hosts.len(), 1);
        let g = &parsed.hosts[0];
        assert_eq!(g.name, "web");
        assert_eq!(g.hostname, "10.0.0.1");
        assert_eq!(g.user, "deploy");
        assert_eq!(g.port, 2222);
        assert_eq!(g.tags, vec!["prod"]);
    }

    #[test]
    fn hostsfile_empty_input_parses_to_empty() {
        let parsed: HostsFile = toml::from_str("").unwrap();
        assert!(parsed.hosts.is_empty());
    }

    #[test]
    fn hostsfile_omits_none_optional_fields() {
        let toml_str = toml::to_string_pretty(&HostsFile {
            hosts: vec![host("a")],
        })
        .unwrap();
        assert!(!toml_str.contains("identity_file"));
        assert!(!toml_str.contains("password"));
    }

    #[test]
    fn hostsfile_password_survives_roundtrip() {
        // Confirms passwords are persisted in plaintext.
        let mut h = host("a");
        h.password = Some("secret".to_string());
        let toml_str = toml::to_string_pretty(&HostsFile { hosts: vec![h] }).unwrap();
        let parsed: HostsFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.hosts[0].password.as_deref(), Some("secret"));
    }

    #[test]
    fn hostsfile_parses_an_entry_written_by_an_older_release() {
        // Fields that no longer exist (source, original_ssh_host) must not
        // reject a file written before they were removed.
        let parsed: HostsFile =
            toml::from_str("[[hosts]]\nname = \"web\"\nhostname = \"10.0.0.1\"\nsource = \"manual\"\noriginal_ssh_host = \"web\"\n")
                .unwrap();
        assert_eq!(names(&parsed.hosts), ["web"]);
    }
}
