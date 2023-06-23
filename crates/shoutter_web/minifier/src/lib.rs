#![feature(stmt_expr_attributes)]

mod func;
mod symbol;
mod sys;

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use anyhow::{Context as _, Result};
use once_cell::sync::Lazy;
use sys::minifier;
use tracing_subscriber::fmt::format::{FmtSpan, Pretty};
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::console;

use crate::sys::{brotli, fs};

#[wasm_bindgen(start)]
async fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    start().await.unwrap();
}

static ORIGINAL_DIR: Lazy<&Path> = Lazy::new(|| Path::new("../../dist"));
static MINIFIED_DIR: Lazy<&Path> = Lazy::new(|| Path::new("../../dist-minified"));

async fn start() -> Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_timer(UtcTime::rfc_3339())
        .with_writer(tracing_web::MakeConsoleWriter)
        .with_span_events(FmtSpan::ACTIVE);
    let perf_layer = tracing_web::performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(EnvFilter::new("swr=none"))
        .with(fmt_layer)
        .with(perf_layer)
        .init();

    fs::rimraf(*MINIFIED_DIR).await?;
    fs::mkdir(*MINIFIED_DIR).await?;

    let mut files = fs::read_dir(*ORIGINAL_DIR).await?;
    files.retain(|x| {
        matches!(
            x.extension().and_then(|x| x.to_str()),
            Some("html" | "css" | "js" | "wasm")
        )
    });

    let bg_wasm = files
        .iter()
        .find(|x| x.to_str().unwrap().ends_with("_bg.wasm"))
        .unwrap();
    let js = files
        .iter()
        .find(|x| {
            bg_wasm
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(x.file_stem().unwrap().to_str().unwrap())
        })
        .unwrap();
    symbol::minify_symbol(bg_wasm, js).await;

    let mut file_name_max_len = 0;
    for f in &files {
        file_name_max_len =
            file_name_max_len.max(f.file_name().unwrap().to_str().unwrap().chars().count());
    }

    println(format!(
        "{1:>0$}: {2:>10} {3:>10} {4:>10}",
        file_name_max_len, "filename", "origin", "minify", "brotli",
    ));

    for f in &files {
        let file_name = f.file_name().unwrap().to_str().unwrap();
        let Some(stats) = process_file(f)
            .await
            .with_context(|| format!("processing {file_name}"))?
            else { continue };
        let kib = |n| format!("{:7.02}KiB", (n as f64) / 1024.0);
        println(format!(
            "{1:>0$}: {2:>} {3:>} {4:>}",
            file_name_max_len,
            file_name,
            kib(stats.origin_size),
            stats
                .minified_size
                .map_or_else(|| format!("{:>10}", "---KiB"), kib),
            kib(stats.brotlied_size),
        ))
    }

    Ok(())
}

fn println(s: String) {
    console::log_1(&JsValue::from(s));
}

struct ProcessStats {
    origin_size: usize,
    minified_size: Option<usize>,
    brotlied_size: usize,
}

async fn process_file(file: &Path) -> Result<Option<ProcessStats>> {
    async fn minify<TFn>(file: &Path, minifier: TFn) -> Result<ProcessStats>
    where
        TFn: FnOnce(&str) -> Pin<Box<dyn Future<Output = Result<String>> + '_>>,
    {
        let filename = file.file_name().unwrap();
        let origin = String::from_utf8(fs::read_file(file).await?)?;
        let minified = minifier(&origin).await?;
        let brotlied_size = brotli::compress(minified.as_bytes()).len();

        fs::write_file(&MINIFIED_DIR.join(filename), minified.as_bytes()).await?;

        Ok(ProcessStats {
            origin_size: origin.len(),
            minified_size: Some(minified.len()),
            brotlied_size,
        })
    }

    let stat = match file.extension().and_then(|x| x.to_str()) {
        Some("html") => minify(file, |x| Box::pin(minifier::html(x))).await?,
        Some("css") => minify(file, |x| Box::pin(minifier::css(x))).await?,
        Some("js") => {
            minify(file, |x| {
                Box::pin(async move { Ok(func::minify_function_decl(x)) })
            })
            .await?
        }
        Some("wasm") => {
            let origin = fs::read_file(file).await?;
            fs::write_file(&MINIFIED_DIR.join(file.file_name().unwrap()), &origin).await?;
            ProcessStats {
                origin_size: origin.len(),
                minified_size: None,
                brotlied_size: brotli::compress(&origin).len(),
            }
        }
        _ => return Ok(None),
    };

    Ok(Some(stat))
}
