use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use slint::ComponentHandle;
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::Context;

use crate::ui::{AppWindow, Install};

const MAX_LINES: usize = 3000;

static UI: OnceLock<slint::Weak<AppWindow>> = OnceLock::new();
static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static CAPTURING: AtomicBool = AtomicBool::new(false);
static UPDATE_QUEUED: AtomicBool = AtomicBool::new(false);

pub fn initialize_ui(ui: slint::Weak<AppWindow>) {
    _ = UI.set(ui);
}

pub fn begin() {
    LINES.lock().clear();
    CAPTURING.store(true, Ordering::SeqCst);
    queue_update();
}

pub fn end() {
    CAPTURING.store(false, Ordering::SeqCst);
}

pub fn push(line: &str) {
    let line = strip_ansi_escapes::strip_str(line);
    append(line);
    queue_update();
}

fn append(line: String) {
    let mut lines = LINES.lock();
    if lines.len() == MAX_LINES {
        lines.pop_front();
    }
    lines.push_back(line);
}

fn queue_update() {
    if UPDATE_QUEUED.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(ui) = UI.get() else {
        UPDATE_QUEUED.store(false, Ordering::SeqCst);
        return;
    };
    crate::ui::on_ui(ui, |app| {
        UPDATE_QUEUED.store(false, Ordering::SeqCst);
        let text = LINES.lock().iter().map(String::as_str).collect::<Vec<_>>().join("\n");
        app.global::<Install<'_>>().set_log(text.into());
    });
}

pub struct Layer;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Layer {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        if !CAPTURING.load(Ordering::Relaxed) || !is_shown(event.metadata()) {
            return;
        }
        let mut fields = Fields::default();
        event.record(&mut fields);
        push(&format!("{}: {}{}", event.metadata().level(), fields.message, fields.rest));
    }
}

fn is_shown(metadata: &tracing::Metadata<'_>) -> bool {
    *metadata.level() <= tracing::Level::DEBUG
        && ["libtaidan", "taidan::install"].iter().any(|t| metadata.target().starts_with(t))
}

#[derive(Default)]
struct Fields {
    message: String,
    rest: String,
}

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            _ = write!(self.rest, " {}={value}", field.name());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            _ = write!(self.message, "{value:?}");
        } else {
            _ = write!(self.rest, " {}={value:?}", field.name());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_last_lines() {
        for i in 0..=MAX_LINES {
            push(&format!("\x1b[1mline {i}\x1b[0m"));
        }
        let lines = LINES.lock();
        assert_eq!(lines.len(), MAX_LINES);
        assert_eq!(lines.front().map(String::as_str), Some("line 1"));
        assert_eq!(lines.back().map(String::as_str), Some(format!("line {MAX_LINES}").as_str()));
    }
}
