use crate::config::AppConfig;
use crate::error::{LyricsifyError, Result};
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSFont, NSScreen, NSTextView, NSVisualEffectView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_foundation::{ns_string, CGPoint, CGRect, CGSize, MainThreadMarker, NSString};
use std::sync::{Arc, Mutex};

/// Manages the overlay window for displaying lyrics
pub struct OverlayWindow {
    window: Retained<NSWindow>,
    text_view: Retained<NSTextView>,
    current_position: Arc<Mutex<CGPoint>>,
    config: Arc<Mutex<AppConfig>>,
}

impl OverlayWindow {
    /// Create a new overlay window with the given configuration
    pub fn new(config: AppConfig) -> Result<Self> {
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        // Get screen dimensions for positioning
        let screen = NSScreen::mainScreen(mtm)
            .ok_or_else(|| LyricsifyError::UIError("Failed to get main screen".to_string()))?;
        let screen_frame = screen.frame();

        // Calculate default position (top-right corner)
        let window_width = 400.0;
        let window_height = 600.0;
        let default_x = screen_frame.size.width - window_width - 20.0;
        let default_y = screen_frame.size.height - window_height - 60.0;

        // Use saved position or default
        let (x, y) = if config.window_position == (100.0, 100.0) {
            (default_x, default_y)
        } else {
            config.window_position
        };

        // Create window frame
        let window_rect = CGRect::new(
            CGPoint::new(x, y),
            CGSize::new(window_width, window_height),
        );

        // Create window with appropriate style mask
        let style_mask = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable
            | NSWindowStyleMask::Resizable
            | NSWindowStyleMask::FullSizeContentView;

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                window_rect,
                style_mask,
                NSBackingStoreType::NSBackingStoreBuffered,
                false,
            )
        };

        // Configure window properties
        unsafe {
            // Set window level to floating (always on top)
            window.setLevel(3); // NSFloatingWindowLevel = 3

            // Make window non-activating (doesn't steal focus)
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::Stationary,
            );

            // Set window opacity
            window.setOpaque(false);
            window.setAlphaValue(0.8);

            // Set background color to clear
            window.setBackgroundColor(Some(&NSColor::clearColor()));

            // Enable rounded corners
            window.setTitlebarAppearsTransparent(true);
            window.setTitleVisibility(NSWindowTitleVisibility::NSWindowTitleHidden);

            // Make window movable by background
            window.setMovableByWindowBackground(true);
        }

        // Create visual effect view for blur background
        let content_view = window
            .contentView()
            .ok_or_else(|| LyricsifyError::UIError("Failed to get content view".to_string()))?;
        let content_frame = content_view.frame();

        let effect_view = unsafe {
            let view = NSVisualEffectView::initWithFrame(mtm.alloc(), content_frame);
            view.setMaterial(NSVisualEffectMaterial::HUDWindow);
            view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            view.setState(NSVisualEffectState::Active);
            view
        };

        // Create text view for lyrics display
        let text_frame = CGRect::new(
            CGPoint::new(20.0, 20.0),
            CGSize::new(content_frame.size.width - 40.0, content_frame.size.height - 40.0),
        );

        let text_view = unsafe {
            let tv = NSTextView::initWithFrame(mtm.alloc(), text_frame);

            // Configure text view properties
            tv.setEditable(false);
            tv.setSelectable(true);
            tv.setBackgroundColor(&NSColor::clearColor());

            // Set text color to white
            tv.setTextColor(Some(&NSColor::whiteColor()));

            // Set font to SF Pro Text, 14pt
            let font = NSFont::systemFontOfSize(14.0);
            tv.setFont(Some(&font));

            // Configure text container for padding and line spacing
            if let Some(text_container) = tv.textContainer() {
                text_container.setLineFragmentPadding(0.0);
            }

            // Set initial text
            tv.setString(ns_string!("Waiting for lyrics..."));

            tv
        };

        // Add text view to effect view
        unsafe {
            effect_view.addSubview(&text_view);
        }

        // Set effect view as content view
        unsafe {
            window.setContentView(Some(&effect_view));
        }

        // Set window visibility based on config
        if config.overlay_visible {
            window.makeKeyAndOrderFront(None);
        }

        let current_position = Arc::new(Mutex::new(CGPoint::new(x, y)));
        let config_arc = Arc::new(Mutex::new(config));

        Ok(Self {
            window,
            text_view,
            current_position,
            config: config_arc,
        })
    }

    /// Show the overlay window
    pub fn show(&self) -> Result<()> {
        self.window.makeKeyAndOrderFront(None);
        unsafe {
            self.window.orderFrontRegardless();
        }

        // Update config
        if let Ok(mut config) = self.config.lock() {
            config.overlay_visible = true;
            let _ = config.save();
        }

        Ok(())
    }

    /// Hide the overlay window
    pub fn hide(&self) -> Result<()> {
        self.window.orderOut(None);

        // Update config
        if let Ok(mut config) = self.config.lock() {
            config.overlay_visible = false;
            let _ = config.save();
        }

        Ok(())
    }

    /// Update the lyrics displayed in the overlay
    pub fn update_lyrics(&self, lyrics: &str) -> Result<()> {
        let text = NSString::from_str(lyrics);
        unsafe {
            self.text_view.setString(&text);
        }
        Ok(())
    }

    /// Get the current window position
    pub fn get_position(&self) -> CGPoint {
        let frame = self.window.frame();
        frame.origin
    }

    /// Set the window position
    pub fn set_position(&self, point: CGPoint) -> Result<()> {
        let mut frame = self.window.frame();
        frame.origin = point;
        self.window.setFrame_display(frame, true);

        // Update stored position
        if let Ok(mut pos) = self.current_position.lock() {
            *pos = point;
        }

        // Save to config
        if let Ok(mut config) = self.config.lock() {
            config.window_position = (point.x, point.y);
            let _ = config.save();
        }

        Ok(())
    }

    /// Check if the overlay is currently visible
    pub fn is_visible(&self) -> bool {
        self.window.isVisible()
    }
}

pub struct UIManager {
    overlay_window: Option<OverlayWindow>,
}

impl UIManager {
    pub fn new(config: AppConfig) -> Result<Self> {
        let overlay_window = Some(OverlayWindow::new(config)?);
        Ok(Self { overlay_window })
    }

    pub fn overlay_window(&self) -> Option<&OverlayWindow> {
        self.overlay_window.as_ref()
    }

    pub fn overlay_window_mut(&mut self) -> Option<&mut OverlayWindow> {
        self.overlay_window.as_mut()
    }
}
