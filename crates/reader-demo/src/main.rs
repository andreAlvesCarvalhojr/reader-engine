//! Demo HTTP do reader-engine: abre amostras e controla ReaderSession.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use reader_core::{Command, DesignSystem, Locator, ReaderSession, Registry, Source, ThemeTokens};
use reader_epub::EpubParser;
use reader_hpub::HpubParser;
use reader_json::JsonParser;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

struct AppState {
    registry: Registry,
    session: Mutex<Option<ReaderSession>>,
    book_id: Mutex<Option<String>>,
}

#[derive(Serialize)]
struct BookInfo {
    id: &'static str,
    title: &'static str,
    format: &'static str,
}

#[derive(Serialize)]
struct StateDto {
    book_id: Option<String>,
    title: Option<String>,
    authors: Vec<String>,
    spine_index: usize,
    spine_len: usize,
    font_scale: f32,
    theme: String,
    toc: Vec<TocDto>,
    page_label: Option<String>,
}

#[derive(Serialize)]
struct TocDto {
    title: String,
    spine_index: usize,
}

#[derive(Deserialize)]
struct OpenBody {
    id: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CmdBody {
    Next,
    Prev,
    GoTo { spine_index: usize },
    SetFontScale { scale: f32 },
    SetTheme { theme: String },
}

fn sample_books() -> &'static [BookInfo] {
    &[
        BookInfo {
            id: "noite-clara",
            title: "Noite Clara",
            format: "reader-json",
        },
        BookInfo {
            id: "epub-fixture",
            title: "EPUB Fixture",
            format: "epub",
        },
        BookInfo {
            id: "hpub-fixture",
            title: "HPUB Fixture",
            format: "hpub",
        },
    ]
}

fn load_source(id: &str) -> Option<Source> {
    match id {
        "noite-clara" => Some(Source {
            name: "noite-clara.json".into(),
            bytes: include_str!("../samples/noite-clara.json")
                .as_bytes()
                .to_vec(),
        }),
        "epub-fixture" => Some(Source {
            name: "sample.epub".into(),
            bytes: include_bytes!("../../reader-epub/tests/fixtures/sample.epub").to_vec(),
        }),
        "hpub-fixture" => Some(Source {
            name: "sample.hpub".into(),
            bytes: include_bytes!("../../reader-hpub/tests/fixtures/sample.hpub").to_vec(),
        }),
        _ => None,
    }
}

fn theme_named(name: &str) -> ThemeTokens {
    let json = match name {
        "hig" => include_str!("../../../themes/hig.json"),
        "fluent" => include_str!("../../../themes/fluent.json"),
        _ => include_str!("../../../themes/material3.json"),
    };
    ThemeTokens::from_json(json).expect("theme json válido")
}

fn theme_key(tokens: &ThemeTokens) -> String {
    match tokens.design_system {
        DesignSystem::Hig => "hig".into(),
        DesignSystem::Fluent => "fluent".into(),
        DesignSystem::Material3 => "material3".into(),
        DesignSystem::Custom => "custom".into(),
    }
}

fn state_dto(book_id: Option<String>, session: &ReaderSession) -> StateDto {
    let pub_ = session.publication();
    let st = session.state();
    let toc = pub_
        .toc
        .iter()
        .filter_map(|entry| {
            let spine_index = pub_.spine.iter().position(|h| h == &entry.href)?;
            Some(TocDto {
                title: entry.title.clone(),
                spine_index,
            })
        })
        .collect();
    let page_label = pub_.spine.get(st.locator.spine_index).cloned().or_else(|| {
        pub_.toc
            .get(st.locator.spine_index)
            .map(|t| t.title.clone())
    });
    StateDto {
        book_id,
        title: Some(pub_.metadata.title.clone()),
        authors: pub_.metadata.authors.clone(),
        spine_index: st.locator.spine_index,
        spine_len: st.spine_len,
        font_scale: st.font_scale,
        theme: theme_key(&st.theme),
        toc,
        page_label,
    }
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn styles() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../static/styles.css"),
    )
        .into_response()
}

async fn app_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../static/app.js"),
    )
        .into_response()
}

async fn list_books() -> Json<Vec<&'static BookInfo>> {
    Json(sample_books().iter().collect())
}

async fn open_book(
    State(state): State<Arc<AppState>>,
    Json(body): Json<OpenBody>,
) -> Result<Json<StateDto>, (StatusCode, String)> {
    let src = load_source(&body.id).ok_or((StatusCode::NOT_FOUND, "livro desconhecido".into()))?;
    let publication = state
        .registry
        .open(src)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let session = ReaderSession::new(publication, theme_named("material3"));
    let dto = state_dto(Some(body.id.clone()), &session);
    *state.session.lock().unwrap() = Some(session);
    *state.book_id.lock().unwrap() = Some(body.id);
    Ok(Json(dto))
}

async fn get_state(State(state): State<Arc<AppState>>) -> Result<Json<StateDto>, StatusCode> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let book_id = state.book_id.lock().unwrap().clone();
    Ok(Json(state_dto(book_id, session)))
}

async fn command(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CmdBody>,
) -> Result<Json<StateDto>, StatusCode> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or(StatusCode::NOT_FOUND)?;
    match body {
        CmdBody::Next => {
            session.dispatch(Command::Next);
        }
        CmdBody::Prev => {
            session.dispatch(Command::Prev);
        }
        CmdBody::GoTo { spine_index } => {
            session.dispatch(Command::GoTo(Locator {
                spine_index,
                progression: 0.0,
            }));
        }
        CmdBody::SetFontScale { scale } => {
            session.dispatch(Command::SetFontScale(scale));
        }
        CmdBody::SetTheme { theme } => {
            session.dispatch(Command::SetTheme(theme_named(&theme)));
        }
    }
    let book_id = state.book_id.lock().unwrap().clone();
    Ok(Json(state_dto(book_id, session)))
}

async fn content(State(state): State<Arc<AppState>>) -> Result<Response, StatusCode> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let st = session.state();
    let href = session
        .publication()
        .spine
        .get(st.locator.spine_index)
        .ok_or(StatusCode::NOT_FOUND)?;
    let bytes = session
        .publication()
        .read(href)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let mut html = String::from_utf8(bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let css = session
        .state()
        .theme
        .to_content_css(session.state().font_scale);
    let inject = format!("<style>{css}</style>");
    if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, &inject);
    } else if let Some(pos) = html.find("<body") {
        html.insert_str(pos, &format!("<head>{inject}</head>"));
    } else {
        html = format!("<!doctype html><html><head>{inject}</head><body>{html}</body></html>");
    }
    Ok(([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response())
}

#[tokio::main]
async fn main() {
    let mut registry = Registry::new();
    registry
        .register(Box::new(JsonParser))
        .register(Box::new(EpubParser))
        .register(Box::new(HpubParser));

    let state = Arc::new(AppState {
        registry,
        session: Mutex::new(None),
        book_id: Mutex::new(None),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/styles.css", get(styles))
        .route("/app.js", get(app_js))
        .route("/api/books", get(list_books))
        .route("/api/open", post(open_book))
        .route("/api/state", get(get_state))
        .route("/api/command", post(command))
        .route("/api/content", get(content))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8787);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("reader-demo em http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
