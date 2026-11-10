//! sizes the window to the screen, keeping the designs' 6:5 aspect ratio, like a macOS setup window
use std::cell::OnceCell;
use std::rc::Rc;

use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::winit::event_loop::ActiveEventLoop;
use slint::winit_030::winit::monitor::MonitorHandle;
use slint::winit_030::winit::window::{Window, WindowId};
use slint::winit_030::{CustomApplicationHandler, EventResult};

use crate::ui::AppWindow;

/// the designs' size, which is also the smallest the window gets
const DESIGN: (f64, f64) = (600., 500.);
/// the window's height on screens of two heights, in logical pixels: it grows slower than the
/// screen, so bigger screens get more margin around it, about half of a 1440p one
const HEIGHT_AT: ((f64, f64), (f64, f64)) = ((720., 620.), (1440., 680.));
/// the most of the screen the window takes in either direction, on small screens
const MAX_FILL: f64 = 0.9;

/// the window's height on a screen this tall, before fitting it to the screen
fn height_for(screen_height: f64) -> f64 {
    let ((h0, w0), (h1, w1)) = HEIGHT_AT;
    (w1 - w0).mul_add((screen_height - h0) / (h1 - h0), w0)
}

/// how much bigger than the designs to draw the window on a screen of this logical size
fn scale_for(screen: (f64, f64)) -> f64 {
    let height = height_for(screen.1).min(MAX_FILL * screen.1);
    let fill = (height / DESIGN.1).min(MAX_FILL * screen.0 / DESIGN.0);
    // at least the designs' size, unless the screen itself is smaller than that. Wayland gives a
    // rotated panel's size before the rotation, which only makes a usual portrait panel turned
    // landscape come out smaller, never too big
    let fit = (screen.0 / DESIGN.0).min(screen.1 / DESIGN.1);
    let scale = fill.max(1.).min(fit);
    // whole hundredths, so `rem`s land on round-ish pixel sizes
    // with a little slack, so 1.62 doesn't come out as 1.6199… and floor to 1.61
    (scale.mul_add(100., 1e-6)).floor() / 100.
}

/// `TAIDAN_SCREEN=WIDTHxHEIGHT` pretends to have a screen of that logical size
fn screen_override() -> Option<(f64, f64)> {
    let s = std::env::var("TAIDAN_SCREEN").ok()?;
    let (w, h) = s.split_once('x')?;
    let (w, h): (f64, f64) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    // `parse` takes `0`, `-5`, `inf` and `NaN`, which would break the scale math
    let valid = |v: f64| v.is_finite() && v > 0.;
    (valid(w) && valid(h)).then_some((w, h))
}

/// the logical size of the smallest screen, so the window fits whichever one it's placed on.
/// `window` is the window's real scale once it's shown, with its screen if winit knows it yet, or
/// [`None`] for every screen: winit only knows each screen's whole-number scale (Wayland's
/// `wl_output.scale`, rounded up from a fractional one), which makes the screen look smaller than
/// it is
fn screen_size(
    monitors: impl Iterator<Item = MonitorHandle>,
    window: Option<(Option<&MonitorHandle>, f64)>,
) -> Option<(f64, f64)> {
    monitors
        .map(|m| {
            let scale = window
                .filter(|&(on, scale)| {
                    on.is_none_or(|on| *on == m) && scale.is_finite() && scale > 0.
                })
                .map_or_else(|| m.scale_factor(), |(_, scale)| scale);
            let size = m.size().to_logical::<f64>(scale);
            tracing::debug!(name = ?m.name(), ?size, scale, "Found a screen");
            (size.width, size.height)
        })
        .filter(|&(w, h)| w > 0. && h > 0.)
        .min_by(|a, b| (a.0 * a.1).total_cmp(&(b.0 * b.1)))
}

/// sets `Tokens.scale` once winit knows the screens, which is right before the window is created,
/// and again once the window has its real scale, which Slint resizes it to
struct ScaleToScreen(Rc<OnceCell<slint::Weak<AppWindow>>>);

impl ScaleToScreen {
    fn apply(&self, screen: (f64, f64)) {
        let Some(ui) = self.0.get().and_then(slint::Weak::upgrade) else { return };
        let scale = scale_for(screen);
        tracing::info!(?screen, scale, "Scaling the window to the screen");
        #[expect(clippy::cast_possible_truncation, reason = "a scale around 1–3")]
        ui.set_ui_scale(scale as f32);
    }
}

impl CustomApplicationHandler for ScaleToScreen {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) -> EventResult {
        if let Some(screen) =
            screen_override().or_else(|| screen_size(event_loop.available_monitors(), None))
        {
            self.apply(screen);
        }
        EventResult::Propagate
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        winit_window: Option<&Window>,
        _slint_window: Option<&slint::Window>,
        event: &WindowEvent,
    ) -> EventResult {
        // the fractional scale comes once the window is on a screen. With fractional-scale-v1 it
        // may come before winit knows which screen, then it's taken for all of them
        if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = event
            && screen_override().is_none()
            && let monitor = winit_window.and_then(Window::current_monitor)
            && let Some(screen) = screen_size(
                event_loop.available_monitors(),
                Some((monitor.as_ref(), *scale_factor)),
            )
        {
            self.apply(screen);
        }
        EventResult::Propagate
    }
}

/// selects the backend with the screen scaling, call before creating the window and pass it
/// to the returned cell
#[must_use]
pub fn init() -> Rc<OnceCell<slint::Weak<AppWindow>>> {
    let ui = Rc::new(OnceCell::new());
    let selected = slint::BackendSelector::new()
        .with_winit_custom_application_handler(ScaleToScreen(Rc::clone(&ui)))
        .select();
    if let Err(e) = selected {
        tracing::warn!(?e, "cannot select the backend, the window won't scale to the screen");
    }
    ui
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        // half of a 1440p screen, and bigger screens never get a smaller window
        assert!(close(scale_for((2560., 1440.)), 1.36));
        assert!(close(scale_for((1920., 1080.)), 1.3));
        assert!(close(scale_for((1366., 768.)), 1.24));
        assert!(scale_for((3840., 2160.)) > scale_for((2560., 1440.)));
        // the width limits portrait screens
        assert!(close(scale_for((800., 1280.)), 1.2));
        // never below the designs' size on smaller screens
        assert!(close(scale_for((1024., 540.)), 1.));
        // unless the screen can't even fit that
        assert!((scale_for((800., 480.)) - 0.96).abs() < 1e-9);
    }
}
