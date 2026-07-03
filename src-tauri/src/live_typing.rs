//! Live typing (experimental, off by default): during a streaming dictation,
//! the committed transcript prefix is typed into the frontmost app as it
//! grows, so words appear while the user is still speaking.
//!
//! Correctness model: committed text is append-only (see `StreamTextEvent`),
//! so deltas are typed forward only. At finalize (or cancel) everything typed
//! is erased again and the unchanged paste path delivers the final text —
//! custom words, spoken punctuation, and AI edits therefore always win, at
//! the cost of a brief re-settle.
//!
//! Safety rails (each one is load-bearing):
//! - Only [`is_erase_safe`] deltas (printable ASCII) are ever typed: for
//!   those, one Unicode scalar == one macOS deletion unit, so the
//!   backspace-based erase count is exact BY CONSTRUCTION. Emoji, combining
//!   marks, and non-Latin scripts freeze live output instead — over-deleting
//!   the user's own text is never risked.
//! - Focus is tracked by AX element identity ([`FocusedElementHandle`]), not
//!   bundle id: any app, window, tab, or field change halts typing. If no
//!   focused element can be captured (unsupported app, non-macOS), live
//!   typing stays off entirely.
//! - The erase runs in chunks, re-checking focus between chunks, and carries
//!   a deadline: a wedged queue can never fire deferred backspaces into the
//!   final pasted text.
//! - Command mode never live-types (the instruction is not dictation), and
//!   paste methods that avoid synthetic input keep live typing off too.

use crate::input::{paste_text_direct, EnigoState};
use crate::selected_text::FocusedElementHandle;
use crate::settings::PasteMethod;
use enigo::{Direction, Key, Keyboard};
use log::{debug, warn};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Backspaces per erase chunk between focus re-checks.
const ERASE_CHUNK: usize = 25;

/// How the committed transcript moved relative to what was already typed.
#[derive(Debug, PartialEq, Eq)]
pub enum LiveDelta<'a> {
    /// Committed grew (or stayed identical — empty delta). Type the suffix.
    Append(&'a str),
    /// Committed no longer starts with the typed text — the model rewrote
    /// history. Stop feeding; reconciliation at finalize fixes the screen.
    Rewrite,
}

/// Compare the already-typed prefix with the new committed text.
pub fn live_typing_delta<'a>(already_typed: &str, committed: &'a str) -> LiveDelta<'a> {
    match committed.strip_prefix(already_typed) {
        Some(delta) => LiveDelta::Append(delta),
        None => LiveDelta::Rewrite,
    }
}

/// True when every scalar in the delta is one the erase can account for
/// exactly: printable ASCII, where one scalar is one macOS deletion unit.
/// Newline and tab are excluded on purpose — enigo turns a leading `\t` into
/// a real Tab KEYPRESS (focus navigation, not text). Anything else must go
/// through the paste path instead.
pub fn is_erase_safe(delta: &str) -> bool {
    delta.chars().all(|c| (' '..='~').contains(&c))
}

/// Live typing synthesizes keystrokes, so it must stay off when the user
/// configured delivery to avoid synthetic input entirely (copy-only) or to
/// delegate it to an external script.
pub fn allowed_for_paste_method(method: &PasteMethod) -> bool {
    !matches!(method, PasteMethod::None | PasteMethod::ExternalScript)
}

enum TyperCommand {
    Type(String),
    /// Erase everything this typer has typed, then ack. Ignored past the
    /// deadline so a late-draining queue cannot delete the pasted text.
    EraseAll {
        deadline: Instant,
        ack: mpsc::Sender<()>,
    },
}

/// Handle to the dedicated typing thread. Keystroke synthesis is slow
/// (milliseconds per event), so it must never run on the stream worker's
/// feed loop — commands queue over a channel instead.
pub struct LiveTyper {
    tx: mpsc::Sender<TyperCommand>,
}

impl LiveTyper {
    /// Spawn the typing thread. The element focused at spawn time is the only
    /// place this typer will ever touch: every command re-checks that the
    /// SAME element still has focus and goes dormant the moment it doesn't.
    pub fn spawn(app_handle: &tauri::AppHandle) -> Self {
        let (tx, rx) = mpsc::channel::<TyperCommand>();
        let app_handle = app_handle.clone();
        std::thread::spawn(move || run_typer_thread(app_handle, rx));
        Self { tx }
    }

    /// Queue a committed-text delta for typing. Never blocks.
    pub fn feed(&self, delta: String) {
        let _ = self.tx.send(TyperCommand::Type(delta));
    }

    /// Erase everything typed so far and wait for it to finish, so the caller
    /// can paste the final text afterwards without interleaving keystrokes.
    /// On timeout the deadline also voids the queued erase itself — the paste
    /// may leave the live prefix behind (duplicate text), but deferred
    /// backspaces can never destroy the pasted result.
    pub fn erase_all_and_wait(self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let (ack_tx, ack_rx) = mpsc::channel();
        if self
            .tx
            .send(TyperCommand::EraseAll {
                deadline,
                ack: ack_tx,
            })
            .is_err()
        {
            return;
        }
        if ack_rx.recv_timeout(timeout).is_err() {
            warn!(
                "Live typing: erase did not finish within {:?}; the queued erase voids itself past the deadline",
                timeout
            );
        }
    }
}

fn run_typer_thread(app_handle: tauri::AppHandle, rx: mpsc::Receiver<TyperCommand>) {
    // Identity of the focused element — not just the app — so window, tab,
    // and field switches all halt typing. No capturable element (unsupported
    // app, non-macOS) means live typing stays off for this session.
    let focus = FocusedElementHandle::capture();
    let mut halted = focus.is_none();
    if halted {
        debug!("Live typing off: no focused element to anchor to");
    }
    // Chars actually typed (not merely queued). Deltas are ASCII-safe by the
    // feed gate, so this count equals the on-screen deletion units exactly.
    let mut typed_chars: usize = 0;
    let focus_ok =
        |focus: &Option<FocusedElementHandle>| focus.as_ref().is_some_and(|f| f.is_still_focused());

    while let Ok(command) = rx.recv() {
        match command {
            TyperCommand::Type(delta) => {
                if halted || delta.is_empty() {
                    continue;
                }
                if !focus_ok(&focus) {
                    debug!("Live typing halted: focus moved mid-dictation");
                    halted = true;
                    continue;
                }
                match with_enigo(&app_handle, |enigo| paste_text_direct(enigo, &delta)) {
                    Some(Ok(())) => typed_chars += delta.chars().count(),
                    Some(Err(e)) => {
                        warn!("Live typing failed, halting for this session: {}", e);
                        halted = true;
                    }
                    None => {
                        debug!("Live typing unavailable (no input synthesizer); halting");
                        halted = true;
                    }
                }
            }
            TyperCommand::EraseAll { deadline, ack } => {
                // Deliberately NOT gated on `halted`: if focus bounced away
                // and came back to the anchored element, everything typed is
                // under the caret again and must be erased or the final paste
                // would duplicate it. The focus identity check is the gate.
                while typed_chars > 0 {
                    if Instant::now() >= deadline {
                        warn!("Live typing: erase deadline reached; leaving remaining live text");
                        break;
                    }
                    if !focus_ok(&focus) {
                        debug!("Live typing: focus moved; leaving live text where it is");
                        break;
                    }
                    let chunk = typed_chars.min(ERASE_CHUNK);
                    let erased = with_enigo(&app_handle, |enigo| {
                        for _ in 0..chunk {
                            enigo
                                .key(Key::Backspace, Direction::Click)
                                .map_err(|e| format!("backspace failed: {}", e))?;
                        }
                        Ok(())
                    });
                    match erased {
                        Some(Ok(())) => typed_chars -= chunk,
                        _ => {
                            warn!("Live typing: erase incomplete");
                            break;
                        }
                    }
                }
                let _ = ack.send(());
                return;
            }
        }
    }
}

/// Run a closure with the shared Enigo synthesizer, or `None` when it was
/// never initialized (frontend not yet loaded — nothing to type into anyway).
fn with_enigo<T>(
    app_handle: &tauri::AppHandle,
    action: impl FnOnce(&mut enigo::Enigo) -> Result<T, String>,
) -> Option<Result<T, String>> {
    use tauri::Manager;
    let enigo_state = app_handle.try_state::<EnigoState>()?;
    let mut enigo = match enigo_state.0.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    Some(action(&mut enigo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_from_empty_is_full_text() {
        assert_eq!(
            live_typing_delta("", "hello world"),
            LiveDelta::Append("hello world")
        );
    }

    #[test]
    fn delta_is_the_new_suffix_only() {
        assert_eq!(
            live_typing_delta("hello ", "hello world"),
            LiveDelta::Append("world")
        );
    }

    #[test]
    fn identical_text_is_an_empty_append() {
        assert_eq!(live_typing_delta("hello", "hello"), LiveDelta::Append(""));
    }

    #[test]
    fn rewritten_prefix_is_detected() {
        assert_eq!(
            live_typing_delta("hallo ", "hello world"),
            LiveDelta::Rewrite
        );
    }

    #[test]
    fn shrinking_committed_text_is_a_rewrite() {
        assert_eq!(
            live_typing_delta("hello world", "hello"),
            LiveDelta::Rewrite
        );
    }

    #[test]
    fn ascii_text_is_erase_safe() {
        assert!(is_erase_safe(
            "Send the invoice, then copy the manager. OK?"
        ));
        assert!(is_erase_safe(""));
    }

    #[test]
    fn newline_and_tab_are_not_erase_safe() {
        // enigo sends a leading tab as a real Tab keypress (focus navigation);
        // both go through the paste path instead.
        assert!(!is_erase_safe("line one\nline two"));
        assert!(!is_erase_safe("col a\tcol b"));
    }

    #[test]
    fn multi_scalar_sequences_are_not_erase_safe() {
        assert!(!is_erase_safe(
            "family \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F466}"
        ));
        // NFD e + combining acute — two scalars, one on-screen character.
        assert!(!is_erase_safe("cafe\u{0301}"));
        assert!(!is_erase_safe("क्या")); // Devanagari conjunct
    }

    #[test]
    fn precomposed_non_ascii_is_conservatively_excluded() {
        // NFC é is a single scalar and would erase fine, but ASCII-only is
        // the provable invariant — it must fall back to paste.
        assert!(!is_erase_safe("café"));
    }

    #[test]
    fn control_characters_are_not_erase_safe() {
        assert!(!is_erase_safe("bell\u{0007}"));
    }

    #[test]
    fn paste_methods_that_avoid_synthetic_input_disable_live_typing() {
        assert!(!allowed_for_paste_method(&PasteMethod::None));
        assert!(!allowed_for_paste_method(&PasteMethod::ExternalScript));
        assert!(allowed_for_paste_method(&PasteMethod::CtrlV));
        assert!(allowed_for_paste_method(&PasteMethod::Direct));
    }
}
