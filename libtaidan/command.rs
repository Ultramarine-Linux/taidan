//! Report missing commands even when their callers recover or only log a warning.
use std::{
    collections::HashSet,
    io,
    process::ExitStatus,
    sync::{Arc, LazyLock, Mutex},
};

type Handler = Arc<dyn Fn(String) + Send + Sync>;

#[derive(Default)]
struct Reports {
    handler: Option<Handler>,
    pending: Vec<String>,
    seen: HashSet<String>,
}

static REPORTS: LazyLock<Mutex<Reports>> = LazyLock::new(Default::default);

/// Register the UI reporter and deliver errors collected before the window existed.
pub fn set_missing_command_handler(handler: impl Fn(String) + Send + Sync + 'static) {
    let handler: Handler = Arc::new(handler);
    let pending = {
        let mut reports = REPORTS.lock().unwrap();
        reports.handler = Some(Arc::clone(&handler));
        std::mem::take(&mut reports.pending)
    };
    for detail in pending {
        handler(detail);
    }
}

fn report(detail: String) {
    let handler = {
        let mut reports = REPORTS.lock().unwrap();
        if !reports.seen.insert(detail.clone()) {
            return;
        }
        if let Some(handler) = &reports.handler {
            Some(Arc::clone(handler))
        } else {
            reports.pending.push(detail.clone());
            None
        }
    };
    if let Some(handler) = handler {
        handler(detail);
    }
}

/// ENOENT from process creation can mean a missing executable, interpreter, or working directory.
pub fn report_spawn_error(program: &str, error: &io::Error) {
    if error.kind() == io::ErrorKind::NotFound {
        report(format!(
            "Cannot launch command `{program}`: {error}\n\n\
             The executable, its interpreter, or its working directory may be missing. \
             Check the distribution's package dependencies and configuration."
        ));
    }
}

/// Recognize shell exit 127, shell diagnostics, and the helper's explicit missing-executable diagnostic.
/// Ordinary errors about missing input files are deliberately not classified this way.
pub fn report_exit(program: &str, status: ExitStatus, output: &str) {
    if missing_command_exit(status.code(), output) {
        report(format!("Command `{program}` could not be found or launched.\n\n{output}"));
    }
}

fn missing_command_exit(code: Option<i32>, output: &str) -> bool {
    code == Some(127)
        || output.lines().any(|line| {
            line.starts_with("taidan-helper: missing executable `")
                || line.trim_end().ends_with(": command not found")
                || (["sh:", "/bin/sh:", "bash:", "/bin/bash:"]
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
                    && line.trim_end().ends_with(": not found"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_and_helper_missing_commands_are_recognized() {
        assert!(missing_command_exit(Some(127), "sh: missing-tool: command not found"));
        assert!(missing_command_exit(
            Some(1),
            "taidan-helper: missing executable `useradd`: No such file or directory (os error 2)"
        ));
        assert!(missing_command_exit(
            Some(0),
            "taidan-helper: missing executable `gsettings`: No such file or directory"
        ));
        assert!(missing_command_exit(Some(0), "sh: line 1: gsettings: command not found"));
        assert!(missing_command_exit(Some(0), "sh: 1: gsettings: not found"));
    }

    #[test]
    fn missing_input_files_and_other_failures_are_not_missing_commands() {
        assert!(!missing_command_exit(Some(1), "cat: input: No such file or directory"));
        assert!(!missing_command_exit(Some(126), "permission denied"));
        assert!(!missing_command_exit(Some(0), "command not found"));
        assert!(!missing_command_exit(None, ""));
    }

    #[test]
    fn reports_real_spawn_errors_once_and_ignores_other_io_errors() {
        let received = Arc::new(Mutex::new(Vec::new()));
        set_missing_command_handler({
            let received = Arc::clone(&received);
            move |detail| received.lock().unwrap().push(detail)
        });
        let program = "/taidan-test-command-that-does-not-exist";
        let error = std::process::Command::new(program).spawn().unwrap_err();
        report_spawn_error(program, &error);
        report_spawn_error(program, &error);
        report_spawn_error("permission-test", &io::Error::from(io::ErrorKind::PermissionDenied));
        let received = received.lock().unwrap();
        assert_eq!(received.iter().filter(|detail| detail.contains(program)).count(), 1);
        assert!(!received.iter().any(|detail| detail.contains("permission-test")));
    }
}
