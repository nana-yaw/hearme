//! Read the focused UI element's selected text via the macOS Accessibility
//! API — the same permission the paste path already requires, so this adds no
//! new grant. Command mode captures this AT SHORTCUT RELEASE (not after
//! transcription) so the text the user had highlighted is what gets
//! transformed, immune to focus changes during the multi-second pipeline.
//!
//! Not every app exposes `AXSelectedText` (some Electron apps, secure fields
//! never) — every failure path returns `None` and the caller falls back to
//! the clipboard, which was the only behavior before this module existed.
//!
//! Also exported: [`FocusedElementHandle`], an identity handle to the focused
//! element used by live typing to detect ANY focus move — including window,
//! tab, and field changes inside the same app, which a bundle-id comparison
//! cannot see. On non-macOS both APIs are inert stubs (`None`).

#[cfg(target_os = "macos")]
mod macos {
    use core_foundation::base::{CFEqual, CFGetTypeID, CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use log::debug;
    use std::ffi::c_void;

    #[repr(C)]
    struct AXUIElementOpaque(c_void);
    type AXUIElementRef = *const AXUIElementOpaque;
    type AXError = i32;

    const AX_ERROR_SUCCESS: AXError = 0;
    /// AX attribute reads are synchronous mach RPCs to the target app with a
    /// ~6 s system default timeout. A hung app must not stall the shortcut
    /// thread or the typing thread for that long.
    const AX_MESSAGING_TIMEOUT_SECS: f32 = 0.5;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f32) -> AXError;
    }

    /// Copy one AX attribute as a raw CFTypeRef (caller releases). `None` on
    /// any AX error — permission missing, attribute unsupported, no focus.
    unsafe fn copy_attribute(element: AXUIElementRef, attribute: &str) -> Option<CFTypeRef> {
        let attribute = CFString::new(attribute);
        let mut value: CFTypeRef = std::ptr::null();
        let err =
            AXUIElementCopyAttributeValue(element, attribute.as_concrete_TypeRef(), &mut value);
        if err != AX_ERROR_SUCCESS || value.is_null() {
            return None;
        }
        Some(value)
    }

    /// The currently focused UI element, retained (caller releases). Setting
    /// the messaging timeout on the system-wide element makes it the default
    /// for all AX messages from this process.
    unsafe fn copy_focused_element() -> Option<AXUIElementRef> {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }
        let _ = AXUIElementSetMessagingTimeout(system_wide, AX_MESSAGING_TIMEOUT_SECS);
        let focused = copy_attribute(system_wide, "AXFocusedUIElement");
        CFRelease(system_wide as CFTypeRef);
        focused.map(|value| value as AXUIElementRef)
    }

    /// The text currently selected in the frontmost app's focused element, or
    /// `None` when there is no selection or the app doesn't expose one.
    pub fn frontmost_selected_text() -> Option<String> {
        unsafe {
            let focused = copy_focused_element()?;
            let selected = copy_attribute(focused, "AXSelectedText");
            CFRelease(focused as CFTypeRef);
            let selected = selected?;

            // The attribute is documented as a string, but a misbehaving app
            // could return anything — verify the type before wrapping.
            if CFGetTypeID(selected) != CFString::type_id() {
                debug!("AXSelectedText returned a non-string value; ignoring");
                CFRelease(selected);
                return None;
            }
            let text = CFString::wrap_under_create_rule(selected as CFStringRef).to_string();
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
    }

    /// Identity handle to the focused element at capture time. Created and
    /// used entirely on the typing thread — deliberately NOT `Send`, since
    /// Apple documents no cross-thread guarantees for `AXUIElementRef`.
    pub struct FocusedElementHandle(AXUIElementRef);

    impl FocusedElementHandle {
        /// Capture the element that currently has focus, or `None` when the
        /// frontmost app exposes no focused element (live typing then stays
        /// off rather than typing blind).
        pub fn capture() -> Option<Self> {
            unsafe { copy_focused_element().map(Self) }
        }

        /// Whether the SAME element (CFEqual identity) still has focus —
        /// false on any app, window, tab, or field change.
        pub fn is_still_focused(&self) -> bool {
            unsafe {
                let Some(current) = copy_focused_element() else {
                    return false;
                };
                let same = CFEqual(self.0 as CFTypeRef, current as CFTypeRef) != 0;
                CFRelease(current as CFTypeRef);
                same
            }
        }
    }

    impl Drop for FocusedElementHandle {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0 as CFTypeRef) }
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{frontmost_selected_text, FocusedElementHandle};

#[cfg(not(target_os = "macos"))]
mod stub {
    /// No selection source off macOS — command mode uses the clipboard only.
    pub fn frontmost_selected_text() -> Option<String> {
        None
    }

    /// No focus identity off macOS — `capture` returns `None`, which keeps
    /// live typing inert rather than typing without a focus guard.
    pub struct FocusedElementHandle;

    impl FocusedElementHandle {
        pub fn capture() -> Option<Self> {
            None
        }

        pub fn is_still_focused(&self) -> bool {
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub use stub::{frontmost_selected_text, FocusedElementHandle};
