mod files;
mod streaming;

use mojibake_core::CaseAnalysis;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::State;

#[derive(Clone)]
struct StoredCase {
    analysis: CaseAnalysis,
    source: Option<files::SourceFile>,
}
#[derive(Default)]
struct Cases(Mutex<VecDeque<StoredCase>>);
static NEXT_CASE: AtomicU64 = AtomicU64::new(1);

fn remember(
    state: &Cases,
    mut analysis: CaseAnalysis,
    source: Option<files::SourceFile>,
) -> Result<CaseAnalysis, String> {
    analysis.case_id = format!("case-{}", NEXT_CASE.fetch_add(1, Ordering::Relaxed));
    let mut cases = state.0.lock().map_err(|_| "state_unavailable")?;
    if cases.len() >= 8 {
        cases.pop_front();
    }
    cases.push_back(StoredCase {
        analysis: analysis.clone(),
        source,
    });
    Ok(analysis)
}
fn lookup(state: &Cases, id: &str) -> Result<StoredCase, String> {
    state
        .0
        .lock()
        .map_err(|_| "state_unavailable")?
        .iter()
        .find(|c| c.analysis.case_id == id)
        .cloned()
        .ok_or_else(|| "case_expired".into())
}

#[tauri::command]
async fn analyze_text(text: String, state: State<'_, Cases>) -> Result<CaseAnalysis, String> {
    if text.len() > files::PASTE_LIMIT {
        return Err("paste_too_large".into());
    }
    let analysis = tauri::async_runtime::spawn_blocking(move || mojibake_core::analyze_text(&text))
        .await
        .map_err(|e| e.to_string())?;
    remember(&state, analysis, None)
}
#[tauri::command]
async fn analyze_file(path: String, state: State<'_, Cases>) -> Result<CaseAnalysis, String> {
    let (analysis, source) =
        tauri::async_runtime::spawn_blocking(move || files::analyze(&PathBuf::from(path)))
            .await
            .map_err(|e| e.to_string())??;
    remember(&state, analysis, Some(source))
}
#[tauri::command]
async fn analyze_sample(id: String, state: State<'_, Cases>) -> Result<CaseAnalysis, String> {
    let mut analysis =
        tauri::async_runtime::spawn_blocking(move || mojibake_core::analyze_sample(&id))
            .await
            .map_err(|e| e.to_string())??;
    analysis.source_type = "sample".into();
    remember(&state, analysis, None)
}
#[tauri::command]
async fn save_candidate(
    case_id: String,
    candidate_id: String,
    path: String,
    state: State<'_, Cases>,
) -> Result<u64, String> {
    let case = lookup(&state, &case_id)?;
    let candidate = case
        .analysis
        .candidates
        .iter()
        .find(|c| c.id == candidate_id)
        .cloned()
        .ok_or("candidate_missing")?;
    tauri::async_runtime::spawn_blocking(move || {
        files::save(
            &PathBuf::from(path),
            case.source.as_ref(),
            &candidate,
            case.analysis.evidence.sampled,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn copy_candidate(
    case_id: String,
    candidate_id: String,
    state: State<'_, Cases>,
) -> Result<String, String> {
    let case = lookup(&state, &case_id)?;
    if case.analysis.evidence.sampled {
        return Err("copy_sample_only".into());
    }
    case.analysis
        .candidates
        .iter()
        .find(|c| c.id == candidate_id)
        .map(|c| c.full_text.clone())
        .ok_or_else(|| "candidate_missing".into())
}
#[tauri::command]
async fn read_hex(
    case_id: String,
    offset: u64,
    state: State<'_, Cases>,
) -> Result<Vec<files::HexRow>, String> {
    let case = lookup(&state, &case_id)?;
    let source = case.source.ok_or("source_unavailable")?;
    tauri::async_runtime::spawn_blocking(move || files::read_hex(&source, offset))
        .await
        .map_err(|e| e.to_string())?
}

pub fn run() {
    let portable = std::env::var_os("MOJIBAKE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|p| p.join("runtime-data")))
        })
        .expect("Cannot determine application folder");
    let temp = portable.join("temp");
    std::fs::create_dir_all(&temp).expect("Application folder must be writable");
    // Scoped to this process and its WebView child; never changes system settings.
    std::env::set_var("TEMP", &temp);
    std::env::set_var("TMP", &temp);
    std::env::set_var("TMPDIR", &temp);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Cases::default())
        .invoke_handler(tauri::generate_handler![analyze_text,analyze_file,analyze_sample,save_candidate,copy_candidate,read_hex])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app,"main",tauri::WebviewUrl::App("index.html".into()))
                .title("Mojibake Lab")
                .inner_size(820.0,640.0).min_inner_size(680.0,520.0)
                .theme(Some(tauri::Theme::Light))
                .center()
                .data_directory(portable.join("webview"))
                .enable_clipboard_access()
                .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-background-networking --disable-component-update --disable-sync --disable-domain-reliability --no-pings --disable-breakpad")
                .on_navigation(|url| url.scheme()=="tauri" || url.host_str()==Some("tauri.localhost") ||
                    (cfg!(debug_assertions) && url.host_str()==Some("127.0.0.1") && url.port()==Some(1420)))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Could not start Mojibake Lab");
}
