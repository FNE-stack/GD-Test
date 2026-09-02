//! Tiny embedded HTTP server: serves the static UI (embedded in the binary)
//! and a small JSON API for listing characters, reading equipped gear, and
//! saving/loading per-character priority-weight profiles.

use crate::catalog::Catalog;
use crate::import::import_grim_gleaner_profile;
use crate::resolve::{resolve_item, sum_all};
use crate::save_parser;
use crate::settings::{self, Settings};
use crate::stats::{prio_score, resist_impact, PrioWeights};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Response, Server};

/// How stale the last heartbeat can get before the watchdog thread kills
/// the process. Generous on purpose: the normal "tab actually closed" case
/// is handled near-instantly by the /api/shutdown beacon below, so this
/// timeout only matters as a fallback (browser crash, force-kill, or any
/// other way the beacon gets skipped) — long enough that a background tab
/// throttled by the browser's timer coalescing never trips a false
/// shutdown while the user still has the page open.
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(60);
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(5);

const INDEX_HTML: &str = include_str!("../ui/index.html");
const APP_JS: &str = include_str!("../ui/app.js");
const STYLE_CSS: &str = include_str!("../ui/style.css");
// Priority tab/package/stat taxonomy, ported from grim_gleaner's own
// stats/registry.py so the priority UI matches grim_gleaner's categories
// and labels rather than an ad-hoc grouping of raw catalog property_ids.
const PRIORITY_TAXONOMY: &str = include_str!("../data/priority_taxonomy.json");

pub struct AppState {
    pub catalog: Catalog,
    /// Mutex, not a plain Option, because the save folder can be changed at
    /// runtime from the Settings panel without restarting the app.
    pub save_dir: Mutex<Option<PathBuf>>,
    pub profiles_dir: PathBuf,
    pub settings_path: PathBuf,
    /// Updated on every /api/heartbeat ping from the page (app.js pings
    /// every 5s while open). Read by the watchdog thread spawned in `run`
    /// to auto-exit if the page has gone away without sending an explicit
    /// /api/shutdown beacon.
    pub last_heartbeat: Mutex<Instant>,
}

pub fn run(state: AppState, port: u16) {
    let server = Server::http(("127.0.0.1", port)).expect("failed to bind local server");
    let state = Arc::new(state);
    println!("GD Gear Compare running at http://127.0.0.1:{port}");

    {
        let watchdog_state = Arc::clone(&state);
        thread::spawn(move || loop {
            thread::sleep(WATCHDOG_INTERVAL);
            let elapsed = watchdog_state.last_heartbeat.lock().unwrap().elapsed();
            if elapsed > HEARTBEAT_TIMEOUT {
                // No console to print to (windows_subsystem = "windows"),
                // and nothing to clean up — the OS reclaims the bound port
                // on process exit, which is the whole point.
                std::process::exit(0);
            }
        });
    }

    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        let mut body = String::new();
        if method == Method::Post {
            let _ = request.as_reader().read_to_string(&mut body);
        }

        let response = handle(&state, &method, &url, &body);
        let _ = request.respond(response);
    }
}

fn handle(
    state: &Arc<AppState>,
    method: &Method,
    url: &str,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match (method, url) {
        (Method::Get, "/") => html_response(INDEX_HTML),
        (Method::Get, "/app.js") => js_response(APP_JS),
        (Method::Get, "/style.css") => css_response(STYLE_CSS),

        (Method::Post, "/api/heartbeat") => {
            *state.last_heartbeat.lock().unwrap() = Instant::now();
            json_response(200, &json!({ "ok": true }))
        }
        // navigator.sendBeacon posts here on pagehide (app.js) — exiting
        // immediately rather than waiting out the watchdog's timeout is the
        // whole point of having this as a separate fast path.
        (Method::Post, "/api/shutdown") => {
            std::process::exit(0);
        }

        (Method::Get, "/api/characters") => api_list_characters(state),
        (Method::Get, "/api/settings") => api_get_settings(state),
        (Method::Post, "/api/settings/save-dir") => api_set_save_dir(state, body),
        (Method::Get, "/api/stats-catalog") => {
            json_response(200, &json!({ "stats": state.catalog.all_property_ids() }))
        }
        (Method::Get, "/api/priority-taxonomy") => {
            let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            Response::from_data(PRIORITY_TAXONOMY.as_bytes().to_vec())
                .with_header(header)
                .with_header(no_cache_header())
        }
        (Method::Get, path) if path.starts_with("/api/equipped/") => {
            let name = &path["/api/equipped/".len()..];
            api_equipped(state, name)
        }
        (Method::Post, "/api/resolve-item") => api_resolve_item(state, body),
        (Method::Post, "/api/compare") => api_compare(state, body),
        // Must come before the generic POST /api/profile/{name} arm below,
        // since both prefix-match "/api/profile/" and match arms are tried
        // in order — this one is the more specific of the two.
        (Method::Post, path)
            if path.starts_with("/api/profile/") && path.ends_with("/import-grim-gleaner") =>
        {
            let name = &path["/api/profile/".len()..path.len() - "/import-grim-gleaner".len()];
            api_import_grim_gleaner_profile(state, name, body)
        }
        (Method::Get, path) if path.starts_with("/api/profile/") => {
            let name = &path["/api/profile/".len()..];
            api_load_profile(state, name)
        }
        (Method::Post, path) if path.starts_with("/api/profile/") => {
            let name = &path["/api/profile/".len()..];
            api_save_profile(state, name, body)
        }

        _ => json_response(404, &json!({ "error": "not found" })),
    }
}

fn api_list_characters(state: &Arc<AppState>) -> Response<std::io::Cursor<Vec<u8>>> {
    let guard = state.save_dir.lock().unwrap();
    let Some(dir) = guard.as_ref() else {
        return json_response(
            200,
            &json!({ "characters": [], "save_dir_found": false }),
        );
    };
    match save_parser::list_characters(dir) {
        Ok(names) => json_response(
            200,
            &json!({ "characters": names, "save_dir_found": true, "save_dir": dir.display().to_string() }),
        ),
        Err(e) => json_response(500, &json!({ "error": e.to_string() })),
    }
}

fn api_get_settings(state: &Arc<AppState>) -> Response<std::io::Cursor<Vec<u8>>> {
    let guard = state.save_dir.lock().unwrap();
    json_response(
        200,
        &json!({ "save_dir": guard.as_ref().map(|p| p.display().to_string()) }),
    )
}

/// Body: { "path": "C:\\...\\save\\main" } — validates the folder actually
/// looks like a Grim Dawn save dir before accepting it, persists the choice
/// to settings.json, and updates the live server state immediately (no
/// restart needed).
fn api_set_save_dir(state: &Arc<AppState>, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    #[derive(serde::Deserialize)]
    struct Req {
        path: String,
    }
    let req: Result<Req, _> = serde_json::from_str(body);
    let req = match req {
        Ok(r) => r,
        Err(e) => return json_response(400, &json!({ "error": e.to_string() })),
    };
    let path = PathBuf::from(req.path.trim());

    if let Err(msg) = settings::validate_save_dir(&path) {
        return json_response(400, &json!({ "error": msg }));
    }

    let mut current_settings = Settings::load(&state.settings_path);
    current_settings.save_dir_override = Some(path.clone());
    if let Err(e) = current_settings.save(&state.settings_path) {
        return json_response(500, &json!({ "error": format!("could not save settings: {e}") }));
    }

    *state.save_dir.lock().unwrap() = Some(path.clone());
    json_response(200, &json!({ "ok": true, "save_dir": path.display().to_string() }))
}

fn api_equipped(state: &Arc<AppState>, character: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let guard = state.save_dir.lock().unwrap();
    let Some(dir) = guard.as_ref() else {
        return json_response(400, &json!({ "error": "no save directory configured" }));
    };
    match save_parser::read_equipped_items(dir, character) {
        Ok(raw_items) => {
            let resolved: Vec<_> = raw_items
                .iter()
                .map(|r| resolve_item(&state.catalog, r))
                .collect();
            let totals = sum_all(&resolved);
            json_response(200, &json!({ "items": resolved, "totals": totals }))
        }
        Err(e) => json_response(500, &json!({ "error": e })),
    }
}

/// Resolves a single item by its base/prefix/suffix DBR paths (used for
/// manually-entered candidate items, e.g. a drop you're comparing against
/// your current gear). Body: { base_name, prefix_name, suffix_name, ... }
fn api_resolve_item(state: &Arc<AppState>, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let raw: Result<save_parser::RawEquippedItem, _> = serde_json::from_str(body);
    match raw {
        Ok(raw) => {
            let resolved = resolve_item(&state.catalog, &raw);
            json_response(200, &serde_json::to_value(resolved).unwrap())
        }
        Err(e) => json_response(400, &json!({ "error": e.to_string() })),
    }
}

/// Body: { "weights": {stat: 0-4, ...}, "baseline_totals": {...},
///          "item_a_stats": {...}, "item_b_stats": {...} }
fn api_compare(_state: &Arc<AppState>, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    #[derive(serde::Deserialize)]
    struct CompareReq {
        weights: PrioWeights,
        baseline_totals: std::collections::HashMap<String, f64>,
        item_a_stats: std::collections::HashMap<String, f64>,
        item_b_stats: std::collections::HashMap<String, f64>,
    }
    let req: Result<CompareReq, _> = serde_json::from_str(body);
    let req = match req {
        Ok(r) => r,
        Err(e) => return json_response(400, &json!({ "error": e.to_string() })),
    };

    let score_a = prio_score(&req.item_a_stats, &req.weights);
    let score_b = prio_score(&req.item_b_stats, &req.weights);
    let max_possible = crate::stats::max_possible_score(&req.weights);
    let grade_a = crate::stats::letter_grade(score_a, max_possible);
    let grade_b = crate::stats::letter_grade(score_b, max_possible);

    let resists = resist_impact(&req.baseline_totals, &req.item_a_stats, &req.item_b_stats);

    json_response(
        200,
        &json!({
            "item_a": { "score": score_a, "grade": grade_a },
            "item_b": { "score": score_b, "grade": grade_b },
            "resist_impact": resists,
        }),
    )
}

fn api_load_profile(state: &Arc<AppState>, character: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let path = state.profiles_dir.join(format!("{character}.json"));
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(v) => json_response(200, &v),
            Err(e) => json_response(500, &json!({ "error": e.to_string() })),
        },
        Err(_) => json_response(200, &json!({ "weights": {} })),
    }
}

fn api_save_profile(
    state: &Arc<AppState>,
    character: &str,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    if let Err(e) = std::fs::create_dir_all(&state.profiles_dir) {
        return json_response(500, &json!({ "error": e.to_string() }));
    }
    let path = state.profiles_dir.join(format!("{character}.json"));
    match std::fs::write(&path, body) {
        Ok(_) => json_response(200, &json!({ "ok": true })),
        Err(e) => json_response(500, &json!({ "error": e.to_string() })),
    }
}

/// Body: a raw grim_gleaner build-profile JSON file (as downloaded/exported
/// from grim_gleaner's own UI), pasted through unmodified. Converts it to
/// GD Gear Compare's weights format and persists it as this character's
/// profile — same file `api_save_profile` writes to — so a page reload
/// picks it up exactly like a profile set by hand in this app.
fn api_import_grim_gleaner_profile(
    state: &Arc<AppState>,
    character: &str,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let result = match import_grim_gleaner_profile(body) {
        Ok(r) => r,
        Err(e) => return json_response(400, &json!({ "error": e })),
    };

    if let Err(e) = std::fs::create_dir_all(&state.profiles_dir) {
        return json_response(500, &json!({ "error": e.to_string() }));
    }
    let path = state.profiles_dir.join(format!("{character}.json"));
    let payload = json!({ "weights": result.weights });
    let text = match serde_json::to_vec_pretty(&payload) {
        Ok(t) => t,
        Err(e) => return json_response(500, &json!({ "error": e.to_string() })),
    };
    if let Err(e) = std::fs::write(&path, text) {
        return json_response(500, &json!({ "error": e.to_string() }));
    }

    json_response(
        200,
        &json!({ "ok": true, "weights": result.weights, "summary": result.summary }),
    )
}

fn json_response(status: u16, value: &serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(value).unwrap_or_default();
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    Response::from_data(body)
        .with_status_code(status)
        .with_header(header)
}

// The UI is embedded in the binary and can change between builds, but the
// browser has no way to know that from the URL alone (it's always the same
// http://127.0.0.1:PORT/app.js). Without an explicit no-store, browsers
// happily serve a stale cached copy from a previous run of the app, which
// silently hides fixes like this one. So: never cache these three files.
fn no_cache_header() -> Header {
    Header::from_bytes(&b"Cache-Control"[..], &b"no-store, must-revalidate"[..]).unwrap()
}

fn html_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
    Response::from_data(body.as_bytes().to_vec())
        .with_header(header)
        .with_header(no_cache_header())
}

fn js_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let header =
        Header::from_bytes(&b"Content-Type"[..], &b"application/javascript; charset=utf-8"[..])
            .unwrap();
    Response::from_data(body.as_bytes().to_vec())
        .with_header(header)
        .with_header(no_cache_header())
}

fn css_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/css; charset=utf-8"[..]).unwrap();
    Response::from_data(body.as_bytes().to_vec())
        .with_header(header)
        .with_header(no_cache_header())
}
