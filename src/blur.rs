//! Asking the compositor to blur what is behind the window.
//!
//! Two backends, picked from the window handle the toolkit hands us:
//!
//! * **X11** — set `_KDE_NET_WM_BLUR_BEHIND_REGION` on the window. An empty
//!   region means "all of it". KWin reads it; so do a few other compositors.
//! * **Wayland** — KWin only offers this through `org_kde_kwin_blur`. We attach
//!   to the toolkit's existing display, bind the manager on an event queue of
//!   our own so the toolkit's queue is never touched, and turn blur on for the
//!   toolkit's surface.
//!
//! Both are entirely optional. If the session is neither, or the compositor
//! does not carry the interface, the app is told so and paints its glass more
//! opaque instead of showing the desktop through it.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};

/// What came of asking.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Untried,
    /// Live, with the name of the backend that did it.
    On(&'static str),
    /// Not available, with something short enough to show in Settings.
    Off(String),
}

impl State {
    pub fn is_on(&self) -> bool {
        matches!(self, State::On(_))
    }

    pub fn describe(&self) -> String {
        match self {
            State::Untried => "not asked for yet".to_string(),
            State::On(how) => format!("on, via {how}"),
            State::Off(why) => format!("off — {why}"),
        }
    }
}

/// Whatever has to stay alive for the blur to stay on. On X11 nothing does; on
/// Wayland the protocol objects must outlive the window.
#[derive(Default)]
pub struct Blur {
    pub state: State,
    /// The X11 window, kept so the blurred region can be reshaped when the
    /// window is resized.
    #[cfg(target_os = "linux")]
    x11_window: Option<u32>,
    /// The size the region was last cut to, in physical pixels.
    #[cfg(target_os = "linux")]
    region_for: Option<(u32, u32, u32)>,
    #[cfg(target_os = "linux")]
    _wayland: Option<wl::Held>,
}

impl Blur {
    /// Cut the blurred area to the window's *rounded* outline.
    ///
    /// Left as the default — the whole window rect — KWin blurs behind the
    /// transparent corners too, and the blurred desktop showing through them
    /// is exactly what reads as a square corner on a rounded window.
    #[allow(unused_variables)]
    pub fn reshape(&mut self, width: u32, height: u32, radius: u32) {
        #[cfg(target_os = "linux")]
        {
            if !self.state.is_on() || width < 4 || height < 4 {
                return;
            }
            let Some(win) = self.x11_window else { return };
            let want = (width, height, radius);
            if self.region_for == Some(want) {
                return;
            }
            self.region_for = Some(want);
            let _ = x11::set_region(win, width, height, radius);
        }
    }
}

impl Blur {
    /// Ask for blur behind `window`. Safe to call when the session cannot do
    /// it — that is the ordinary case on GNOME.
    pub fn install<H>(handle: &H) -> Blur
    where
        H: HasWindowHandle + HasDisplayHandle,
    {
        #[cfg(target_os = "linux")]
        {
            let win = match handle.window_handle() {
                Ok(w) => w.as_raw(),
                Err(e) => {
                    return Blur {
                        state: State::Off(format!("no window handle ({e})")),
                        x11_window: None,
                        region_for: None,
                        _wayland: None,
                    }
                }
            };
            let disp = match handle.display_handle() {
                Ok(d) => d.as_raw(),
                Err(e) => {
                    return Blur {
                        state: State::Off(format!("no display handle ({e})")),
                        x11_window: None,
                        region_for: None,
                        _wayland: None,
                    }
                }
            };

            let x11_window = match win {
                RawWindowHandle::Xlib(w) => Some(w.window as u32),
                RawWindowHandle::Xcb(w) => Some(w.window.get()),
                _ => None,
            };
            match (win, disp) {
                (RawWindowHandle::Xlib(_), _) | (RawWindowHandle::Xcb(_), _) => Blur {
                    state: x11::set(x11_window.unwrap_or(0)),
                    x11_window,
                    region_for: None,
                    _wayland: None,
                },
                (RawWindowHandle::Wayland(w), RawDisplayHandle::Wayland(d)) => {
                    match wl::set(d.display.as_ptr(), w.surface.as_ptr()) {
                        Ok(held) => Blur {
                            state: State::On("org_kde_kwin_blur"),
                            x11_window: None,
                            region_for: None,
                            _wayland: Some(held),
                        },
                        Err(why) => Blur {
                            state: State::Off(why),
                            x11_window: None,
                            region_for: None,
                            _wayland: None,
                        },
                    }
                }
                _ => Blur {
                    state: State::Off("this window system has no blur hook".into()),
                    x11_window: None,
                    region_for: None,
                    _wayland: None,
                },
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = handle;
            Blur {
                state: State::Off("only wired up for Linux".into()),
            }
        }
    }
}

// ------------------------------------------------------------------- X11 --

#[cfg(target_os = "linux")]
mod x11 {
    use super::State;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
    use x11rb::wrapper::ConnectionExt as _;

    pub fn set(window: u32) -> State {
        match try_set(window) {
            Ok(()) => State::On("_KDE_NET_WM_BLUR_BEHIND_REGION"),
            Err(e) => State::Off(e),
        }
    }

    fn try_set(window: u32) -> Result<(), String> {
        let (conn, screen) = x11rb::connect(None).map_err(|e| format!("X11: {e}"))?;
        let ask = |name: &[u8]| -> Result<u32, String> {
            conn.intern_atom(false, name)
                .map_err(|e| format!("X11: {e}"))?
                .reply()
                .map_err(|e| format!("X11: {e}"))
                .map(|r| r.atom)
        };

        // An empty region asks for the whole window. `set_region` narrows it to
        // the rounded outline once the window's size is known.
        let blur = ask(b"_KDE_NET_WM_BLUR_BEHIND_REGION")?;
        conn.change_property32(PropMode::REPLACE, window, blur, AtomEnum::CARDINAL, &[])
            .map_err(|e| format!("X11: {e}"))?
            .check()
            .map_err(|e| format!("X11: {e}"))?;
        conn.flush().map_err(|e| format!("X11: {e}"))?;

        // Nothing reads that property unless a compositing manager owns the
        // screen's selection. Without one, going translucent would show the
        // desktop through the window rather than a blur of it.
        let cm = ask(format!("_NET_WM_CM_S{screen}").as_bytes())?;
        let owner = conn
            .get_selection_owner(cm)
            .map_err(|e| format!("X11: {e}"))?
            .reply()
            .map_err(|e| format!("X11: {e}"))?
            .owner;
        if owner == x11rb::NONE {
            return Err("nothing is compositing this X11 screen".into());
        }
        Ok(())
    }

    /// Cut the blur to a rounded rectangle, as a run of one-pixel-tall strips
    /// down each corner plus one big rectangle for everything between them.
    pub fn set_region(window: u32, w: u32, h: u32, radius: u32) -> Result<(), String> {
        let (conn, _) = x11rb::connect(None).map_err(|e| format!("X11: {e}"))?;
        let blur = conn
            .intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION")
            .map_err(|e| format!("X11: {e}"))?
            .reply()
            .map_err(|e| format!("X11: {e}"))?
            .atom;

        let r = radius.min(w / 2).min(h / 2);
        let mut rects: Vec<u32> = Vec::with_capacity((r as usize * 2 + 1) * 4);
        let strip = |x: u32, y: u32, sw: u32, sh: u32, out: &mut Vec<u32>| {
            if sw > 0 && sh > 0 {
                out.extend_from_slice(&[x, y, sw, sh]);
            }
        };
        for y in 0..r {
            // how far in the edge has come at this row of the corner arc
            let dy = (r - y) as f32;
            let inset = (r as f32 - ((r as f32).powi(2) - dy * dy).max(0.0).sqrt()).round() as u32;
            let inset = inset.min(w / 2);
            strip(inset, y, w.saturating_sub(inset * 2), 1, &mut rects);
            strip(inset, h - 1 - y, w.saturating_sub(inset * 2), 1, &mut rects);
        }
        strip(0, r, w, h.saturating_sub(r * 2), &mut rects);

        conn.change_property32(PropMode::REPLACE, window, blur, AtomEnum::CARDINAL, &rects)
            .map_err(|e| format!("X11: {e}"))?
            .check()
            .map_err(|e| format!("X11: {e}"))?;
        conn.flush().map_err(|e| format!("X11: {e}"))?;
        Ok(())
    }
}

// --------------------------------------------------------------- Wayland --

#[cfg(target_os = "linux")]
mod wl {
    use std::ffi::c_void;

    use wayland_backend::sys::client::{Backend, ObjectId};
    use wayland_client::protocol::wl_registry::{self, WlRegistry};
    use wayland_client::protocol::wl_surface::WlSurface;
    use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
    use wayland_protocols_plasma::blur::client::org_kde_kwin_blur::OrgKdeKwinBlur;
    use wayland_protocols_plasma::blur::client::org_kde_kwin_blur_manager::OrgKdeKwinBlurManager;

    /// The pieces that have to outlive the call, or the compositor drops the
    /// blur again.
    pub struct Held {
        _blur: OrgKdeKwinBlur,
        _manager: OrgKdeKwinBlurManager,
        _queue: EventQueue<Hunt>,
        _conn: Connection,
    }

    #[derive(Default)]
    pub struct Hunt {
        manager: Option<OrgKdeKwinBlurManager>,
    }

    impl Dispatch<WlRegistry, ()> for Hunt {
        fn event(
            state: &mut Self,
            registry: &WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
            {
                if interface == "org_kde_kwin_blur_manager" && state.manager.is_none() {
                    state.manager = Some(registry.bind::<OrgKdeKwinBlurManager, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ));
                }
            }
        }
    }

    // Neither interface sends anything back.
    impl Dispatch<OrgKdeKwinBlurManager, ()> for Hunt {
        fn event(
            _: &mut Self,
            _: &OrgKdeKwinBlurManager,
            _: <OrgKdeKwinBlurManager as Proxy>::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<OrgKdeKwinBlur, ()> for Hunt {
        fn event(
            _: &mut Self,
            _: &OrgKdeKwinBlur,
            _: <OrgKdeKwinBlur as Proxy>::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    pub fn set(display: *mut c_void, surface: *mut c_void) -> Result<Held, String> {
        if display.is_null() || surface.is_null() {
            return Err("Wayland: the toolkit gave us no display".into());
        }
        // Wrapping a display we do not own: dropping this connection leaves the
        // toolkit's display exactly as it found it, and every object we make
        // goes on an event queue of our own so the toolkit's is never dispatched
        // out from under it.
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let conn = Connection::from_backend(backend);

        let mut queue: EventQueue<Hunt> = conn.new_event_queue();
        let qh = queue.handle();
        let mut hunt = Hunt::default();

        let _registry = conn.display().get_registry(&qh, ());
        queue
            .roundtrip(&mut hunt)
            .map_err(|e| format!("Wayland: {e}"))?;

        let Some(manager) = hunt.manager.clone() else {
            return Err("this compositor has no org_kde_kwin_blur_manager".into());
        };

        // The surface belongs to the toolkit; we only ever name it in a request.
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.cast()) }
            .map_err(|_| "Wayland: the surface is not one we can name".to_string())?;
        let surface =
            WlSurface::from_id(&conn, id).map_err(|_| "Wayland: unusable surface".to_string())?;

        let blur = manager.create(&surface, &qh, ());
        // No region set means the whole surface.
        blur.set_region(None);
        blur.commit();
        conn.flush().map_err(|e| format!("Wayland: {e}"))?;
        queue
            .roundtrip(&mut hunt)
            .map_err(|e| format!("Wayland: {e}"))?;

        Ok(Held {
            _blur: blur,
            _manager: manager,
            _queue: queue,
            _conn: conn,
        })
    }
}

/// What the session says it is, for the Settings panel.
pub fn session_name() -> String {
    let kind = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_default();
    match (kind.is_empty(), desktop.is_empty()) {
        (true, true) => "unknown session".to_string(),
        (true, false) => desktop,
        (false, true) => kind,
        (false, false) => format!("{desktop} on {kind}"),
    }
}
