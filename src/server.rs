//! Tiny embedded HTTP server: serves the static UI (embedded in the binary)
//! and a small JSON API for listing characters, reading equipped gear, and
//! saving/loading per-character priority-weight profiles.

use crate::catalog::Catalog;
use crate::resolve::{resolve_item, sum_all};
use crate::save_parser;
use crate::stats::{prio_score, resist_impact, PrioWeights};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tiny_http::{Header, Method, Response, Server};

const INDEX_HTML: &str = include_str!("../ui/index.html");
const APP_JS: &str = include_str!("../ui/app.js");
const STYLE_CSS: &str = include_str!("../ui/style.css");

pub struct AppState {
    pub catalog: Catalog,
    pub save_dir: Option<PathBuf>,
    pub profiles_dir: PathBuf,
}

pub fn run(state: AppState, port: u16) {
    let server = Server::http(("127.0.0.1", port)).expect("failed to bind local server");
    let state = Arc::new(state);
    println!("GD Gear Compare running at http://127.0.0.1:{port}");

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

        (Method::Get, "/api/characters") => api_list_characters(state),
        (Method::Get, "/api/stats-catalog") => {
            json_response(200, &json!({ "stats": state.catalog.all_property_ids() }))
        }
        (Method::Get, path) if path.starts_with("/api/equipped/") => {
            let name = &path["/api/equipped/".len()..];
            api_equipped(state, name)
        }
        (Method::Post, "/api/resolve-item") => api_resolve_item(state, body),
        (Method::Post, "/api/compare") => api_compare(state, body),
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
    let Some(dir) = &state.save_dir else {
        return json_response(
            200,
            &json!({ "characters": [], "save_dir_found": false }),
        );
    };
    match save_parser::list_characters(dir) {
        Ok(names) => json_response(
            200,
            &json!({ "characters": names, "save_dir_found": true }),
        ),
        Err(e) => json_response(500, &json!({ "error": e.to_string() })),
    }
}

fn api_equipped(state: &Arc<AppState>, character: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(dir) = &state.save_dir else {
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

fn json_response(status: u16, value: &serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(value).unwrap_or_default();
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    Response::from_data(body)
        .with_status_code(status)
        .with_header(header)
}

fn html_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
    Response::from_data(body.as_bytes().to_vec()).with_header(header)
}

fn js_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let header =
        Header::from_bytes(&b"Content-Type"[..], &b"application/javascript; charset=utf-8"[..])
            .unwrap();
    Response::from_data(body.as_bytes().to_vec()).with_header(header)
}

fn css_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/css; charset=utf-8"[..]).unwrap();
    Response::from_data(body.as_bytes().to_vec()).with_header(header)
}
