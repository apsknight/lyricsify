use crate::app_core::AppEvent;
use crate::config::AppConfig;
use crate::error::{LyricsifyError, Result};
use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSFont, NSMenu, NSMenuItem, NSScreen, NSStatusBar, NSStatusItem,
    NSTextView, NSVisualEffectView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    NSWindowTitleVisibility,
};
use objc2_foundation::{ns_string, CGPoint, CGRect, CGSize, MainThreadMarker, NSObject, NSString};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

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
        window.setContentView(Some(&effect_view));

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

// Declare a custom delegate class for handling menu actions
struct MenuBarDelegateIvars {
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

declare_class!(
    struct MenuBarDelegate;

    unsafe impl ClassType for MenuBarDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MenuBarDelegate";
    }

    impl DeclaredClass for MenuBarDelegate {
        type Ivars = MenuBarDelegateIvars;
    }

    unsafe impl MenuBarDelegate {
        #[method(toggleOverlay:)]
        fn toggle_overlay(&self, _sender: *const NSMenuItem) {
            let _ = self.ivars().event_tx.send(AppEvent::ToggleOverlay);
        }

        #[method(authenticate:)]
        fn authenticate(&self, _sender: *const NSMenuItem) {
            let _ = self.ivars().event_tx.send(AppEvent::Authenticate);
        }

        #[method(quit:)]
        fn quit(&self, _sender: *const NSMenuItem) {
            let _ = self.ivars().event_tx.send(AppEvent::Quit);
        }
    }
);

impl MenuBarDelegate {
    fn new(event_tx: mpsc::UnboundedSender<AppEvent>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>();
        let this = this.set_ivars(MenuBarDelegateIvars { event_tx });
        unsafe { msg_send_id![super(this), init] }
    }
}

/// Manages the menu bar status item and dropdown menu
pub struct MenuBar {
    status_item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    toggle_item: Retained<NSMenuItem>,
    auth_item: Retained<NSMenuItem>,
    delegate: Retained<MenuBarDelegate>,
    overlay_visible: Arc<Mutex<bool>>,
    authenticated: Arc<Mutex<bool>>,
}

impl MenuBar {
    /// Create a new menu bar with status item
    pub fn new(event_tx: mpsc::UnboundedSender<AppEvent>) -> Result<Self> {
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        // Create the delegate
        let delegate = MenuBarDelegate::new(event_tx, mtm);

        // Get the system status bar and create status item
        let status_item = unsafe {
            let status_bar = NSStatusBar::systemStatusBar();
            status_bar.statusItemWithLength(-1.0) // NSVariableStatusItemLength = -1.0
        };

        // Create the menu
        let menu = NSMenu::new(mtm);

        // Set the icon to a musical note symbol
        if let Some(button) = unsafe { status_item.button(mtm) } {
            // Use a simple text-based icon for now
            // SF Symbols require newer objc2-app-kit APIs
            unsafe {
                button.setTitle(ns_string!("♪"));
            }
        }

        // Create menu items with actions
        // 1. Toggle Lyrics menu item
        let toggle_item = unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                ns_string!("Show Lyrics"),
                Some(objc2::sel!(toggleOverlay:)),
                ns_string!(""),
            );
            item.setTarget(Some(&delegate));
            item
        };

        // 2. Authenticate Spotify menu item
        let auth_item = unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                ns_string!("Authenticate Spotify"),
                Some(objc2::sel!(authenticate:)),
                ns_string!(""),
            );
            item.setTarget(Some(&delegate));
            item
        };

        // 3. Quit menu item
        let quit_item = unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                ns_string!("Quit"),
                Some(objc2::sel!(quit:)),
                ns_string!("q"),
            );
            item.setTarget(Some(&delegate));
            item
        };

        // Add items to menu
        menu.addItem(&toggle_item);
        menu.addItem(&auth_item);
        menu.addItem(
            &NSMenuItem::separatorItem(mtm), // Add separator before quit
        );
        menu.addItem(&quit_item);

        // Attach the menu to the status item
        unsafe {
            status_item.setMenu(Some(&menu));
        }

        Ok(Self {
            status_item,
            menu,
            toggle_item,
            auth_item,
            delegate,
            overlay_visible: Arc::new(Mutex::new(false)),
            authenticated: Arc::new(Mutex::new(false)),
        })
    }

    /// Update the visibility state of the overlay
    pub fn update_visibility_state(&self, visible: bool) -> Result<()> {
        if let Ok(mut vis) = self.overlay_visible.lock() {
            *vis = visible;
        }

        // Update menu item text
        let title = if visible {
            ns_string!("Hide Lyrics")
        } else {
            ns_string!("Show Lyrics")
        };
        unsafe {
            self.toggle_item.setTitle(title);
        }

        // Update icon appearance based on visibility
        // For now, we'll keep the same icon but could change color in future
        // when SF Symbols support is available in objc2-app-kit

        Ok(())
    }

    /// Update the authentication state
    pub fn update_auth_state(&self, authenticated: bool) -> Result<()> {
        if let Ok(mut auth) = self.authenticated.lock() {
            *auth = authenticated;
        }

        // Show/hide the authenticate menu item based on auth state
        unsafe {
            self.auth_item.setHidden(authenticated);
        }

        Ok(())
    }

    /// Get the current visibility state
    pub fn is_overlay_visible(&self) -> bool {
        self.overlay_visible.lock().map(|v| *v).unwrap_or(false)
    }

    /// Get the current authentication state
    pub fn is_authenticated(&self) -> bool {
        self.authenticated.lock().map(|a| *a).unwrap_or(false)
    }
}
