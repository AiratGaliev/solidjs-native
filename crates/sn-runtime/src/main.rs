use rquickjs::{Context, Runtime};
use std::{env, fs, process::ExitCode};

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

    let result = context.with(|ctx| -> rquickjs::Result<()> {
        ctx.globals().set("__solid_native_runtime__", "QuickJS-NG")?;
        if let Some(code) = source.as_deref() {
            ctx.eval::<(), _>(code)?;
            println!("JavaScript bundle executed successfully");

            let mounted: bool = ctx.eval("typeof globalThis.__sn_demo_root !== 'undefined'")?;
            if mounted {
                let root_children: usize = ctx.eval("globalThis.__sn_demo_root.children.length")?;
                let view_children: usize = ctx.eval("globalThis.__sn_demo_root.children[0].children.length")?;
                println!("SolidJS universal renderer mounted: root children={root_children}, view children={view_children}");

                ctx.eval::<(), _>("globalThis.__sn_demo_root.children[0].children[2].properties.onClick()")?;
                let updated_parts: String = ctx.eval("JSON.stringify(globalThis.__sn_demo_root.children[0].children[1].children.map((node) => node.text))")?;
                println!("SolidJS signal update executed inside QuickJS: {updated_parts}");
            }
        } else {
            let name: String = ctx.eval("globalThis.__solid_native_runtime__")?;
            println!("SolidJS Native runtime ready ({name})");
        }
        Ok(())
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("JavaScript execution failed: {error}");
            ExitCode::FAILURE
        }
    }
}
