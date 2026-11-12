use std::{env, fs, process::ExitCode};

use sn_runtime::{EngineSession, EngineSessionConfig};

fn run_smoke(source: Option<&str>) -> Result<(), String> {
    let session = EngineSession::start(EngineSessionConfig::default())
        .map_err(|error| format!("failed to start EngineSession: {error}"))?;
    let handle = session.handle();

    let result = (|| -> Result<(), String> {
        if let Some(source) = source {
            let initial = handle
                .evaluate(source)
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;

            let initial_commit = initial
                .last_commit
                .as_ref()
                .ok_or_else(|| "Counter mount did not produce a Host Protocol commit".to_owned())?;
            let has_operation = |predicate: fn(&sn_core::HostOperation) -> bool| {
                initial_commit.delta.iter().any(predicate)
            };
            let has_view = has_operation(|operation| {
                matches!(
                    operation,
                    sn_core::HostOperation::CreateElement { tag, .. } if tag == "view"
                )
            });
            let has_button = has_operation(|operation| {
                matches!(
                    operation,
                    sn_core::HostOperation::CreateElement { tag, .. } if tag == "button"
                )
            });
            let has_initial_count = has_operation(|operation| {
                matches!(
                    operation,
                    sn_core::HostOperation::CreateText { text, .. } if text == "Count: 0"
                )
            });
            let has_click_binding = has_operation(|operation| {
                matches!(
                    operation,
                    sn_core::HostOperation::BindEvent { event, .. } if event == "click"
                )
            });
            let has_insert = has_operation(|operation| {
                matches!(operation, sn_core::HostOperation::InsertNode { .. })
            });
            let initial_text = handle
                .find_text_containing("Count:")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;
            let initial_status = handle
                .find_text_containing("Async:")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;

            if !has_view
                || !has_button
                || !has_initial_count
                || !has_click_binding
                || !has_insert
                || initial_text.as_ref().map(|(_, text)| text.as_str()) != Some("Count: 0")
                || initial_status.as_ref().map(|(_, text)| text.as_str()) != Some("Async: Idle")
                || initial.callback_count == 0
                || initial.roots.is_empty()
            {
                return Err(format!(
                    "Counter initial mount acceptance failed: {initial:?}"
                ));
            }
            println!(
                "SolidJS Counter mounted on the engine thread: nodes={}, callbacks={}, revision={}",
                initial.node_count, initial.callback_count, initial.revision
            );

            let button_id = handle
                .find_element("button")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "Counter button was not committed to Rust UiTree".to_owned())?;
            let dispatched = handle
                .dispatch_event(button_id, "click")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;
            if !dispatched {
                return Err("Counter click event was not dispatched".into());
            }

            let updated = handle
                .snapshot()
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;
            let updated_text = handle
                .find_text_containing("Count:")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;
            let updated_status = handle
                .find_text_containing("Async:")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;
            let replace_text_count = updated.last_commit.as_ref().map_or(0, |commit| {
                commit
                    .delta
                    .iter()
                    .filter(|operation| {
                        matches!(operation, sn_core::HostOperation::ReplaceText { .. })
                    })
                    .count()
            });
            if updated_text.as_ref().map(|(_, text)| text.as_str()) != Some("Count: 1")
                || updated_status.as_ref().map(|(_, text)| text.as_str()) != Some("Async: Ready")
                || updated.revision != initial.revision + 1
                || updated
                    .last_commit
                    .as_ref()
                    .map_or(0, |commit| commit.delta.len())
                    != 2
                || replace_text_count != 2
            {
                return Err(format!(
                    "Counter signal update acceptance failed: {updated:?}"
                ));
            }
            println!(
                "Promise + queueMicrotask updates coalesced into one Host Protocol commit with two ReplaceText operations (revision={})",
                updated.revision
            );

            handle
                .evaluate("__sn_demo_dispose();")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;

            handle
                .evaluate(
                    "for (let cycle = 0; cycle < 8; cycle++) { const cleanup = __sn_mount_counter(); cleanup(); }",
                )
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;

            let disposed = handle
                .snapshot()
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;
            if disposed.callback_count != 0 || disposed.node_count != 0 {
                return Err(format!(
                    "Repeated mount/unmount leaked runtime roots: {disposed:?}"
                ));
            }
            println!(
                "Eight mount/unmount cycles completed; callback roots={}, tree nodes={}",
                disposed.callback_count, disposed.node_count
            );
        } else {
            let runtime_name = handle
                .evaluate_text("String(globalThis.__solid_native_runtime__)")
                .map_err(|error| error.to_string())?
                .wait()
                .map_err(|error| error.to_string())?;
            println!("SolidJS Native runtime ready ({runtime_name})");
        }
        Ok(())
    })();

    if let Err(error) = &result {
        for diagnostic in handle.take_diagnostics() {
            eprintln!("runtime diagnostic: {diagnostic:?}");
        }
        eprintln!("runtime smoke failed: {error}");
    }

    let shutdown = session
        .shutdown()
        .map_err(|error| format!("EngineSession shutdown failed: {error}"));
    result?;
    shutdown
}

fn main() -> ExitCode {
    let source_path = env::args().nth(1);
    let source = match source_path.as_deref() {
        Some(path) => match fs::read_to_string(path) {
            Ok(source) => Some(source),
            Err(error) => {
                eprintln!("failed to read JavaScript source: {error}");
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    match run_smoke(source.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
