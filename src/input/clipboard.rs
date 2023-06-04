use raw_window_handle::HasRawDisplayHandle;

/// Handles interfacing with the OS clipboard.
///
/// If the "clipboard" feature is off, or we cannot connect to the OS clipboard,
/// then a fallback clipboard that just works works within the same app is used instead.
pub struct Clipboard {
    arboard: Option<arboard::Clipboard>,

    /// Fallback manual clipboard.
    clipboard: String,
}

impl Clipboard {
    /// Construct a new instance
    ///
    /// # Safety
    ///
    /// The returned `Clipboard` must not outlive the input `_display_target`.
    pub fn new(_display_target: &dyn HasRawDisplayHandle) -> Self {
        Self {
            arboard: init_arboard(),

            clipboard: Default::default(),
        }
    }

    pub fn get(&mut self) -> Option<String> {
        if let Some(clipboard) = &mut self.arboard {
            return match clipboard.get_text() {
                Ok(text) => Some(text),
                Err(err) => {
                    log::error!("arboard paste error: {err}");
                    None
                }
            };
        }

        Some(self.clipboard.clone())
    }

    pub fn set(&mut self, text: String) {
        if let Some(clipboard) = &mut self.arboard {
            if let Err(err) = clipboard.set_text(text) {
                log::error!("arboard copy/cut error: {err}");
            }
            return;
        }

        self.clipboard = text;
    }
}

fn init_arboard() -> Option<arboard::Clipboard> {
    log::debug!("Initializing arboard clipboard…");
    match arboard::Clipboard::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            log::warn!("Failed to initialize arboard clipboard: {err}");
            None
        }
    }
}
