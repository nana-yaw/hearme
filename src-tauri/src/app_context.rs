//! Frontmost-application detection for per-app post-processing profiles.
//! Uses macOS's built-in `lsappinfo` (no accessibility permission, no extra
//! dependency, ~30 ms) — called once per post-processed transcription, at
//! stop time, which matches where the paste will land.

#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "macos")]
pub fn frontmost_bundle_id() -> Option<String> {
    let front = Command::new("lsappinfo").arg("front").output().ok()?;
    let asn = String::from_utf8_lossy(&front.stdout).trim().to_string();
    if asn.is_empty() {
        return None;
    }
    let info = Command::new("lsappinfo")
        .args(["info", "-only", "bundleid", &asn])
        .output()
        .ok()?;
    parse_bundle_id(&String::from_utf8_lossy(&info.stdout))
}

#[cfg(not(target_os = "macos"))]
pub fn frontmost_bundle_id() -> Option<String> {
    None
}

/// Parses `"CFBundleIdentifier"="com.apple.Notes"` (lsappinfo's output shape).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_bundle_id(output: &str) -> Option<String> {
    let value = output.split('=').nth(1)?.trim().trim_matches('"').trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lsappinfo_output() {
        assert_eq!(
            parse_bundle_id("\"CFBundleIdentifier\"=\"com.apple.Notes\"\n"),
            Some("com.apple.Notes".to_string())
        );
    }

    #[test]
    fn rejects_empty_or_malformed_output() {
        assert_eq!(parse_bundle_id(""), None);
        assert_eq!(parse_bundle_id("no separator here"), None);
        assert_eq!(parse_bundle_id("\"CFBundleIdentifier\"=\"\""), None);
    }
}
