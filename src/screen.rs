use std::cell::OnceCell;
use std::rc::Rc;

use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::winit::event_loop::ActiveEventLoop;
use slint::winit_030::winit::monitor::MonitorHandle;
use slint::winit_030::winit::window::{Window, WindowId};
use slint::winit_030::{CustomApplicationHandler, EventResult};

use crate::ui::AppWindow;

const DESIGN_SIZE: (f64, f64) = (600., 500.);
const SMALL_SCREEN_HEIGHT: f64 = 720.;
const WINDOW_HEIGHT_ON_SMALL_SCREEN: f64 = 620.;
const LARGE_SCREEN_HEIGHT: f64 = 1440.;
const WINDOW_HEIGHT_ON_LARGE_SCREEN: f64 = 680.;
const MAX_SCREEN_SHARE: f64 = 0.9;
const ROUNDING_SLACK: f64 = 1e-6;

fn unfitted_window_height(screen_height: f64) -> f64 {
    let progress =
        (screen_height - SMALL_SCREEN_HEIGHT) / (LARGE_SCREEN_HEIGHT - SMALL_SCREEN_HEIGHT);
    (WINDOW_HEIGHT_ON_LARGE_SCREEN - WINDOW_HEIGHT_ON_SMALL_SCREEN)
        .mul_add(progress, WINDOW_HEIGHT_ON_SMALL_SCREEN)
}

fn floor_to_hundredths(value: f64) -> f64 {
    value.mul_add(100., ROUNDING_SLACK).floor() / 100.
}

fn scale_for(screen: (f64, f64)) -> f64 {
    let height = unfitted_window_height(screen.1).min(MAX_SCREEN_SHARE * screen.1);
    let fill = (height / DESIGN_SIZE.1).min(MAX_SCREEN_SHARE * screen.0 / DESIGN_SIZE.0);
    // Wayland gives a rotated panel's size before the rotation, which only ever errs smaller
    let fit_whole_screen = (screen.0 / DESIGN_SIZE.0).min(screen.1 / DESIGN_SIZE.1);
    floor_to_hundredths(fill.max(1.).min(fit_whole_screen))
}

fn screen_override() -> Option<(f64, f64)> {
    let s = std::env::var("TAIDAN_SCREEN").ok()?;
    let (w, h) = s.split_once('x')?;
    let (w, h): (f64, f64) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    let usable = |v: f64| v.is_finite() && v > 0.;
    (usable(w) && usable(h)).then_some((w, h))
}

fn smallest_screen_size(
    monitors: impl Iterator<Item = MonitorHandle>,
    window_scale: Option<(Option<&MonitorHandle>, f64)>,
) -> Option<(f64, f64)> {
    // until the window reports its real scale, winit only knows whole-number screen scales
    monitors
        .map(|m| {
            let scale = window_scale
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

struct ScaleToScreen(Rc<OnceCell<slint::Weak<AppWindow>>>);

impl ScaleToScreen {
    fn apply(&self, screen: (f64, f64)) {
        let Some(ui) = self.0.get().and_then(slint::Weak::upgrade) else { return };
        let margin = 2. * f64::from(ui.get_shadow_margin());
        let scale = scale_for(((screen.0 - margin).max(1.), (screen.1 - margin).max(1.)));
        tracing::info!(?screen, scale, "Scaling the window to the screen");
        #[expect(clippy::cast_possible_truncation, reason = "a scale around 1–3")]
        ui.set_ui_scale(scale as f32);
    }
}

impl CustomApplicationHandler for ScaleToScreen {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) -> EventResult {
        if let Some(screen) = screen_override()
            .or_else(|| smallest_screen_size(event_loop.available_monitors(), None))
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
        if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = event
            && screen_override().is_none()
            && let monitor = winit_window.and_then(Window::current_monitor)
            && let Some(screen) = smallest_screen_size(
                event_loop.available_monitors(),
                Some((monitor.as_ref(), *scale_factor)),
            )
        {
            self.apply(screen);
        }
        EventResult::Propagate
    }
}

#[must_use]
pub fn select_scaling_backend() -> Rc<OnceCell<slint::Weak<AppWindow>>> {
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
        assert!(close(scale_for((2560., 1440.)), 1.36));
        assert!(close(scale_for((1920., 1080.)), 1.3));
        assert!(close(scale_for((1366., 768.)), 1.24));
        assert!(scale_for((3840., 2160.)) > scale_for((2560., 1440.)));
        assert!(close(scale_for((800., 1280.)), 1.2));
        assert!(close(scale_for((1024., 540.)), 1.));
        assert!(close(scale_for((800., 480.)), 0.96));
    }
}
