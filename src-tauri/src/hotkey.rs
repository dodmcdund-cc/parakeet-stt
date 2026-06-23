use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyMode {
    PushToTalk,
    Toggle,
}

struct Binding {
    keycode: u32,
    modifiers: u32,
    on_press: Box<dyn Fn() + Send>,
    on_release: Box<dyn Fn() + Send>,
}

pub struct HotkeyManager {
    mode: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    bindings: Vec<Binding>,
    started: bool,
}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            mode: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
            bindings: Vec::new(),
            started: false,
        }
    }

    pub fn set_mode(&mut self, mode: HotkeyMode) {
        self.mode
            .store(mode == HotkeyMode::Toggle, Ordering::SeqCst);
    }

    pub fn mode(&self) -> HotkeyMode {
        if self.mode.load(Ordering::SeqCst) {
            HotkeyMode::Toggle
        } else {
            HotkeyMode::PushToTalk
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn register(
        &mut self,
        keycode: u32,
        modifiers: u32,
        on_press: Box<dyn Fn() + Send>,
        on_release: Box<dyn Fn() + Send>,
    ) {
        self.bindings.push(Binding {
            keycode,
            modifiers,
            on_press,
            on_release,
        });
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.started {
            return Err("hotkey manager already started".into());
        }
        self.started = true;

        let bindings = std::mem::take(&mut self.bindings);
        let (tx, rx) = mpsc::channel();
        let mode = self.mode.clone();
        let active = self.active.clone();

        platform::start_listener(&bindings, tx)?;

        std::thread::Builder::new()
            .name("hotkey-dispatch".into())
            .spawn(move || {
                match mode.load(Ordering::SeqCst) {
                    false => Self::run_push_to_talk(rx, &bindings, &active),
                    true => Self::run_toggle(rx, &bindings, &active),
                }
            })
            .map_err(|e| format!("failed to spawn dispatch thread: {}", e))?;

        Ok(())
    }

    fn run_push_to_talk(
        rx: mpsc::Receiver<PlatformEvent>,
        bindings: &[Binding],
        active: &AtomicBool,
    ) {
        while let Ok(event) = rx.recv() {
            match event {
                PlatformEvent::Press(kc, _mods) => {
                    active.store(true, Ordering::SeqCst);
                    if let Some(b) = bindings.iter().find(|b| b.keycode == kc) {
                        (b.on_press)();
                    }
                }
                PlatformEvent::Release(kc, _mods) => {
                    active.store(false, Ordering::SeqCst);
                    if let Some(b) = bindings.iter().find(|b| b.keycode == kc) {
                        (b.on_release)();
                    }
                }
            }
        }
    }

    fn run_toggle(
        rx: mpsc::Receiver<PlatformEvent>,
        bindings: &[Binding],
        active: &AtomicBool,
    ) {
        while let Ok(event) = rx.recv() {
            if let PlatformEvent::Press(kc, _mods) = event {
                let was = active.fetch_xor(true, Ordering::SeqCst);
                if let Some(b) = bindings.iter().find(|b| b.keycode == kc) {
                    if was {
                        (b.on_release)();
                    } else {
                        (b.on_press)();
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Platform abstraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum PlatformEvent {
    Press(u32, u32),
    Release(u32, u32),
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::collections::HashMap;
    use std::os::raw::{c_int, c_long, c_uint, c_ulong};
    use std::sync::mpsc;
    use x11_dl::xlib;

    struct HotkeyState {
        display: *mut xlib::Display,
        xlib: xlib::Xlib,
        registry: HashMap<i32, bool>,
    }

    unsafe impl Send for HotkeyState {}

    pub fn start_listener(
        bindings: &[Binding],
        tx: mpsc::Sender<PlatformEvent>,
    ) -> Result<(), String> {
        let xlib =
            xlib::Xlib::open().map_err(|e| format!("X11 open failed: {}", e))?;

        let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };
        if display.is_null() {
            return Err("XOpenDisplay returned null".into());
        }

        let root = unsafe { (xlib.XDefaultRootWindow)(display) };

        let mut registry = HashMap::new();

        for b in bindings {
            let keycode = unsafe {
                (xlib.XKeysymToKeycode)(display, b.keycode as c_uint as c_ulong) as i32
            };

            let ret = unsafe {
                (xlib.XGrabKey)(
                    display,
                    keycode as c_int,
                    b.modifiers as c_uint,
                    root,
                    0,
                    xlib::GrabModeAsync as c_int,
                    xlib::GrabModeAsync as c_int,
                )
            };
            if ret == 0 {
                return Err(format!("XGrabKey failed for keycode {}", b.keycode));
            }
            registry.insert(keycode, false);
        }

        unsafe {
            (xlib.XSelectInput)(
                display,
                root,
                (xlib::KeyPressMask | xlib::KeyReleaseMask) as c_long,
            );
            let mut supported: c_int = 0;
            (xlib.XkbSetDetectableAutoRepeat)(display, 1, &mut supported);
        }

        let state = HotkeyState {
            display,
            xlib,
            registry,
        };

        std::thread::Builder::new()
            .name("hotkey-x11".into())
            .spawn(move || x11_listen(state, tx))
            .map_err(|e| format!("failed to spawn X11 thread: {}", e))?;

        Ok(())
    }

    fn x11_listen(mut state: HotkeyState, tx: mpsc::Sender<PlatformEvent>) {
        unsafe {
            loop {
                let mut event_buf: std::mem::MaybeUninit<xlib::XEvent> =
                    std::mem::MaybeUninit::uninit();
                (state.xlib.XNextEvent)(state.display, event_buf.as_mut_ptr());
                let ev: &xlib::XEvent = event_buf.assume_init_ref();

                let type_ = ev.get_type();
                if type_ != xlib::KeyPress && type_ != xlib::KeyRelease {
                    continue;
                }

                let keycode = ev.key.keycode as i32;
                let is_press = type_ == xlib::KeyPress;

                let was_pressed = state.registry.get(&keycode).copied().unwrap_or(false);

                if is_press && !was_pressed {
                    state.registry.insert(keycode, true);
                    let _ = tx.send(PlatformEvent::Press(keycode as u32, 0));
                } else if !is_press && was_pressed {
                    state.registry.insert(keycode, false);
                    let _ = tx.send(PlatformEvent::Release(keycode as u32, 0));
                }
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::sync::mpsc;

    pub fn start_listener(
        _bindings: &[Binding],
        _tx: mpsc::Sender<PlatformEvent>,
    ) -> Result<(), String> {
        Err("Windows hotkey support not yet implemented".into())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType,
    };
    use core_graphics::event::CGEventField;
    use std::sync::mpsc;
    use std::sync::Mutex;

    static TX: std::sync::LazyLock<Mutex<Option<mpsc::Sender<PlatformEvent>>>> =
        std::sync::LazyLock::new(|| Mutex::new(None));

    pub fn start_listener(
        _bindings: &[Binding],
        tx: mpsc::Sender<PlatformEvent>,
    ) -> Result<(), String> {
        *TX.lock().unwrap() = Some(tx);

        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown, CGEventType::KeyUp],
            |_proxy, event_type, event| {
                let keycode = event
                    .get_integer_value_field(CGEventField::KeyboardEventKeycode)
                    as u32;

                if let Some(ref tx) = *TX.lock().unwrap() {
                    match event_type {
                        CGEventType::KeyDown => {
                            let _ = tx.send(PlatformEvent::Press(keycode, 0));
                        }
                        CGEventType::KeyUp => {
                            let _ = tx.send(PlatformEvent::Release(keycode, 0));
                        }
                        _ => {}
                    }
                }

                Some(event)
            },
        )
        .map_err(|e| format!("failed to create event tap: {:?}", e))?;

        let source = tap
            .mach_port
            .create_runloop_source(0)
            .map_err(|e| format!("failed to create runloop source: {:?}", e))?;

        unsafe {
            tap.enable();
            CFRunLoop::get_current().add_source(&source, kCFRunLoopCommonModes);
            CFRunLoop::run_current();
        }

        Ok(())
    }
}
