mod host;

use host::{HostState, SharedHostState, dispatch_event_and_flush, install_host_bindings};
use rquickjs::{Context, Exception, Runtime};
use std::{cell::RefCell, env, fs, process::ExitCode, rc::Rc};

fn report_quickjs_exception(ctx: &rquickjs::Ctx<'_>, error: &rquickjs::Error) {
    if !error.is_exception() || !ctx.has_exception() {
        return;
    }

    match Exception::from_value(ctx.catch()) {
        Ok(exception) => {
            let message = exception
                .message()
                .unwrap_or_else(|| "JavaScript Error without a message".to_owned());
            eprintln!("QuickJS exception: {message}");
            if let Some(stack) = exception.stack() {
                eprintln!("QuickJS stack:\n{stack}");
            }
        }
        Err(_) => eprintln!("QuickJS raised a non-Error JavaScript value"),
    }
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

    let runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create QuickJS runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let context = match Context::full(&runtime) {
        Ok(context) => context,
        Err(error) => {
            eprintln!("failed to create QuickJS context: {error}");
            return ExitCode::FAILURE;
        }
    };
    let state: SharedHostState = Rc::new(RefCell::new(HostState::new(
        sn_core::SurfaceId::new(1),
        sn_core::SessionGeneration::new(1),
    )));

    let result = context.with(|ctx| -> rquickjs::Result<()> {
        ctx.globals().set("__solid_native_runtime__", "QuickJS-NG")?;
        install_host_bindings(ctx.clone(), state.clone())?;

        let execution = (|| -> rquickjs::Result<()> {
            if let Some(code) = source.as_deref() {
                ctx.eval::<(), _>(code)?;
                println!("JavaScript bundle executed successfully");

                let (root_id, button_id, initial_text, node_count, mount_count) = {
                    let host = state.try_borrow()
                        .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
                    let root = host.tree.find_element("root")
                        .ok_or_else(|| Exception::throw_internal(&ctx, "Counter root was not committed to Rust UiTree"))?;
                    let button = host.tree.find_element("button")
                        .ok_or_else(|| Exception::throw_internal(&ctx, "Counter button was not committed to Rust UiTree"))?;
                    let count_text = host.tree.find_text_containing("Count:")
                        .ok_or_else(|| Exception::throw_internal(&ctx, "Counter text was not committed to Rust UiTree"))?;
                    (
                        root,
                        button,
                        host.tree.text_content(count_text).map_err(|error| Exception::throw_internal(&ctx, &error.to_string()))?.to_owned(),
                        host.tree.node_count(),
                        host.sink().commits().len(),
                    )
                };
                println!("Rust UiTree committed: root={}, nodes={node_count}, revision=1, mount batches={mount_count}", root_id.get());
                println!("Solid counter initial text: {initial_text}");

                {
                    let host = state
                        .try_borrow()
                        .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
                    let mount = host
                        .sink()
                        .commits()
                        .front()
                        .ok_or_else(|| Exception::throw_internal(&ctx, "HeadlessMountSink missed the initial mount"))?;
                    let has_view = mount.delta.iter().any(|operation| {
                        matches!(operation, sn_core::HostOperation::CreateElement { tag, .. } if tag == "view")
                    });
                    let has_button = mount.delta.iter().any(|operation| {
                        matches!(operation, sn_core::HostOperation::CreateElement { tag, .. } if tag == "button")
                    });
                    let has_initial_count = mount.delta.iter().any(|operation| {
                        matches!(operation, sn_core::HostOperation::CreateText { text, .. } if text == "Count: 0")
                    });
                    let has_click_binding = mount.delta.iter().any(|operation| {
                        matches!(operation, sn_core::HostOperation::BindEvent { event, .. } if event == "click")
                    });
                    let has_insert = mount.delta.iter().any(|operation| {
                        matches!(operation, sn_core::HostOperation::InsertNode { .. })
                    });
                    if !has_view || !has_button || !has_initial_count || !has_click_binding || !has_insert {
                        return Err(Exception::throw_internal(
                            &ctx,
                            "HeadlessMountSink initial mount delta is missing the expected Counter operations",
                        ));
                    }
                }

                if !dispatch_event_and_flush(&ctx, &state, button_id, "click")? {
                    return Err(Exception::throw_internal(&ctx, "Counter click event was not dispatched"));
                }

                let host = state.try_borrow()
                    .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
                let count_text = host.tree.find_text_containing("Count:")
                    .ok_or_else(|| Exception::throw_internal(&ctx, "Updated counter text is missing"))?;
                let updated_text = host.tree.text_content(count_text)
                    .map_err(|error| Exception::throw_internal(&ctx, &error.to_string()))?;
                if updated_text != "Count: 1" {
                    return Err(Exception::throw_internal(&ctx, "Signal update did not commit Count: 1 to Rust UiTree"));
                }
                if host.sink().commits().len() != 2 {
                    return Err(Exception::throw_internal(&ctx, "HeadlessMountSink did not observe the mount and update batches"));
                }
                let update_batch = host.sink().commits().back()
                    .ok_or_else(|| Exception::throw_internal(&ctx, "Missing headless mount update"))?;
                let replace_text_count = update_batch
                    .delta
                    .iter()
                    .filter(|operation| matches!(operation, sn_core::HostOperation::ReplaceText { .. }))
                    .count();
                if update_batch.delta.len() != 1 || replace_text_count != 1 {
                    return Err(Exception::throw_internal(
                        &ctx,
                        "HeadlessMountSink signal update delta must contain exactly one ReplaceText operation",
                    ));
                }
                println!("QuickJS click dispatched to JS callback; Rust UiTree text updated: {updated_text}");
                drop(host);

                let dispose: rquickjs::Function = ctx.globals().get("__sn_demo_dispose")?;
                dispose.call::<_, ()>(())?;
                {
                    let host = state.try_borrow()
                        .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
                    if host.callback_count() != 0 || host.tree.node_count() != 0 {
                        return Err(Exception::throw_internal(&ctx, "Renderer disposal did not release the tree and callback roots"));
                    }
                }

                let mount: rquickjs::Function = ctx.globals().get("__sn_mount_counter")?;
                for _ in 0..8 {
                    let cleanup: rquickjs::Function = mount.call(())?;
                    cleanup.call::<_, ()>(())?;
                    let host = state.try_borrow()
                        .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
                    if host.callback_count() != 0 || host.tree.node_count() != 0 {
                        return Err(Exception::throw_internal(&ctx, "Repeated Solid mount/unmount leaked tree nodes or callback roots"));
                    }
                }
                let host = state.try_borrow()
                    .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
                println!("HeadlessMountSink observed {} commits; callback roots after 8 mount/unmount cycles={}", host.sink().commits().len(), host.callback_count());
            } else {
                let name: String = ctx.eval("globalThis.__solid_native_runtime__")?;
                println!("SolidJS Native runtime ready ({name})");
            }
            Ok(())
        })();

        if let Err(error) = &execution {
            report_quickjs_exception(&ctx, error);
        }
        execution
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("JavaScript/native host execution failed: {error}");
            ExitCode::FAILURE
        }
    }
}
