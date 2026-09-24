//! Motion.
//!
//! Everything that moves in the app goes through here, for two reasons: the
//! curves stay consistent, and one switch in Settings turns the whole lot off
//! without any call site needing to know.

// The shared kit is kept whole and in step with Tesseract's copy: a
// primitive with no caller in Northstar today is still part of the language.
#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui::{self, Id};

static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

// ------------------------------------------------------------------ curves --

/// Symmetric and satisfying in both directions — the default for anything
/// that toggles, because a hover that eases in and snaps out feels broken.
pub fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Fast off the mark, long settle. The curve most of the app moves on.
pub fn ease_out_expo(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 {
        1.0
    } else {
        1.0 - (-9.0 * t).exp2()
    }
}

/// Overshoots and springs back, with the wobble of something with mass.
pub fn ease_out_elastic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return t;
    }
    let c = std::f32::consts::TAU / 3.0;
    (-9.0 * t).exp2() * ((t * 10.0 - 0.75) * c).sin() + 1.0
}

/// The house bounce: one clear overshoot and a short settle, rather than the
/// long ring of a true elastic. Everything that *arrives* uses this.
pub fn bounce(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 {
        return 1.0;
    }
    // a damped spring, normalised so bounce(0) == 0
    1.0 - (-6.2 * t).exp() * (8.4 * t).cos()
}

/// The house bounce, reshaped for something that *travels*.
///
/// `bounce` is a pop: it reaches its target a fifth of the way in and spends
/// the rest wobbling, which is right for a thing appearing and wrong for a
/// thing crossing a distance. This is the same damped spring with its first
/// arrival pushed out to two thirds of the window, an overshoot after it, and
/// an envelope that lands it exactly on 1.
pub fn bounce_travel(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 {
        return 1.0;
    }
    1.0 - (-2.2 * t).exp() * (2.55 * t).cos() * (1.0 - t).sqrt()
}

/// 0 → 1 on the house bounce, driven by a flag.
pub fn bounce_in(ctx: &egui::Context, id: impl std::hash::Hash, on: bool, secs: f32) -> f32 {
    if !enabled() {
        return if on { 1.0 } else { 0.0 };
    }
    let raw = ctx.animate_bool_with_time(Id::new(id), on, secs);
    if on {
        bounce(raw)
    } else {
        smootherstep(raw)
    }
}

/// Overshoots past 1 and settles back: the bounce the alerts arrive on.
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

/// A damped spring, normalised so `pop(0) == 0` and `pop(1) == 1`. Bouncier
/// than `ease_out_back`, used where something should feel physical.
pub fn pop(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 {
        return 1.0;
    }
    1.0 - (-7.0 * t).exp() * (10.0 * t).cos()
}

// ------------------------------------------------------------------ drivers --

/// 0 while `on` is false, 1 while it is true, on a smootherstep both ways.
/// This is the one every hover, highlight and reveal should use.
pub fn ease(ctx: &egui::Context, id: impl std::hash::Hash, on: bool, secs: f32) -> f32 {
    if !enabled() {
        return if on { 1.0 } else { 0.0 };
    }
    smootherstep(ctx.animate_bool_with_time(Id::new(id), on, secs))
}

/// A value that eases toward its target between frames. Snaps when motion is
/// switched off.
pub fn to(ctx: &egui::Context, id: impl std::hash::Hash, target: f32, secs: f32) -> f32 {
    if !enabled() {
        return target;
    }
    ctx.animate_value_with_time(Id::new(id), target, secs)
}

/// A value that eases toward its target along the app's curve rather than in
/// a straight line. Keeps a shadow of the last target so the curve is applied
/// across the whole journey, not per frame.
pub fn glide(ctx: &egui::Context, id: impl std::hash::Hash, target: f32, secs: f32) -> f32 {
    glide_on(ctx, id, target, secs, ease_out_expo)
}

/// The same, arriving on the house bounce — for anything that *travels*.
pub fn spring_to(ctx: &egui::Context, id: impl std::hash::Hash, target: f32, secs: f32) -> f32 {
    glide_on(ctx, id, target, secs, bounce)
}

fn glide_on(
    ctx: &egui::Context,
    id: impl std::hash::Hash,
    target: f32,
    secs: f32,
    curve: fn(f32) -> f32,
) -> f32 {
    if !enabled() {
        return target;
    }
    let id = Id::new(id);
    let raw = ctx.animate_value_with_time(id.with("raw"), target, secs);
    // shape the remaining distance, which turns a linear ramp into an ease-out
    let from = ctx.data_mut(|d| *d.get_temp_mut_or(id.with("from"), raw));
    let span = target - from;
    if span.abs() < 0.0001 {
        ctx.data_mut(|d| d.insert_temp(id.with("from"), target));
        return target;
    }
    let t = ((raw - from) / span).clamp(0.0, 1.0);
    if t >= 0.999 {
        ctx.data_mut(|d| d.insert_temp(id.with("from"), target));
    }
    from + span * curve(t)
}

/// Seconds since the app started, for anything that loops.
pub fn clock(ctx: &egui::Context) -> f32 {
    ctx.input(|i| i.time) as f32
}

/// A slow breath in 0..=1, for idle glows.
pub fn breathe(ctx: &egui::Context, period: f32) -> f32 {
    if !enabled() {
        return 0.5;
    }
    let t = clock(ctx) / period.max(0.05);
    0.5 - 0.5 * (t * std::f32::consts::TAU).cos()
}

/// Ask for another frame while something is still moving.
pub fn keep_going(ctx: &egui::Context) {
    if enabled() {
        ctx.request_repaint();
    }
}

/// How long a hover takes to arrive and to leave. Long enough to read as a
/// response rather than a blink, short enough never to lag the pointer.
pub const HOVER: f32 = 0.24;
