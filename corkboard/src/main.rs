//! Corkboard — the noticeboard. Deliberately dumb, by design and by
//! regulatory strategy (see TRADING_ROADMAP.md): it stores and serves
//! signed envelopes and blind relay blobs. It never matches orders, never
//! executes, never holds keys or funds, charges no fees, has no accounts.
//! Humans pick offers; the swap happens entirely between the two pactds.
//!
//! Single binary + SQLite so anyone can self-host; multiple independent
//! operators is the goal (Bisq model). v2 moves offer distribution and
//! relay to Nostr.
//!
//! Surface:
//!   GET  /health
//!   POST /v1/offers           signed offer envelope (type "offer")
//!   GET  /v1/offers           list active offers (filters: asset pair, network)
//!   POST /v1/offers/revoke    signed revocation (type "revoke", same identity)
//!   POST /v1/relay            {to, blob} — store-and-forward, content-blind
//!   POST /v1/relay/poll       signed poll (type "relay_poll") → messages since cursor
//!
//! All write endpoints require a valid BIP340 envelope signature
//! (pact_proto::envelope::verify) — listings can't be forged. Proof-of-funds
//! verification is the *client's* job (clients check the chain); the board
//! never talks to any chain.

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use pact_proto::envelope::{verify, Envelope};
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Reject anything bigger than this — the relay is for coordination
/// envelopes, not file transfer.
const MAX_BLOB_BYTES: usize = 64 * 1024;
const DEFAULT_OFFER_TTL_SECS: u64 = 24 * 3600;

#[derive(Parser, Debug)]
#[command(name = "corkboard", version)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:9780")]
    listen: SocketAddr,
    #[arg(long, default_value = "corkboard.sqlite")]
    db: PathBuf,
}

#[derive(Clone)]
struct App {
    db: Arc<Mutex<Connection>>,
}

struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("{:#}", self.0) })),
        )
            .into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before 1970")
        .as_secs()
}

fn open_db(path: &PathBuf) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS offers (
             offer_id  TEXT PRIMARY KEY,
             identity  TEXT NOT NULL,
             envelope  TEXT NOT NULL,
             created   INTEGER NOT NULL,
             expires   INTEGER NOT NULL,
             revoked   INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS relay (
             id        INTEGER PRIMARY KEY AUTOINCREMENT,
             recipient TEXT NOT NULL,
             blob      TEXT NOT NULL,
             created   INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS relay_recipient ON relay (recipient, id);",
    )?;
    Ok(conn)
}

fn verified(envelope: &Envelope, expected_type: &str) -> Result<(), ApiError> {
    if envelope.msg_type != expected_type {
        return Err(ApiError(anyhow::anyhow!(
            "expected a {expected_type} envelope, got {}",
            envelope.msg_type
        )));
    }
    verify(envelope).map_err(ApiError)
}

async fn health() -> &'static str {
    "ok"
}

async fn post_offer(
    State(app): State<App>,
    Json(envelope): Json<Envelope>,
) -> Result<Json<Value>, ApiError> {
    verified(&envelope, "offer")?;
    let ttl = envelope.body["ttl_secs"]
        .as_u64()
        .unwrap_or(DEFAULT_OFFER_TTL_SECS);
    let ttl = ttl.min(7 * 24 * 3600); // a week, tops — offers are not archives
    let created = now();
    let db = app.db.lock().expect("db mutex");
    db.execute(
        "INSERT INTO offers (offer_id, identity, envelope, created, expires)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(offer_id) DO NOTHING",
        params![
            envelope.swap_id,
            envelope.from,
            serde_json::to_string(&envelope)?,
            created,
            created + ttl
        ],
    )?;
    Ok(Json(json!({ "offer_id": envelope.swap_id })))
}

#[derive(Deserialize)]
struct OfferFilter {
    give: Option<String>,
    get: Option<String>,
    network: Option<String>,
}

async fn list_offers(
    State(app): State<App>,
    Query(filter): Query<OfferFilter>,
) -> Result<Json<Value>, ApiError> {
    let db = app.db.lock().expect("db mutex");
    let mut stmt = db.prepare(
        "SELECT envelope FROM offers WHERE revoked = 0 AND expires > ?1 ORDER BY created DESC LIMIT 500",
    )?;
    let rows: Vec<String> = stmt
        .query_map(params![now()], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let offers: Vec<Envelope> = rows
        .iter()
        .filter_map(|raw| serde_json::from_str::<Envelope>(raw).ok())
        .filter(|env| {
            let body = &env.body;
            filter
                .give
                .as_deref()
                .is_none_or(|v| body["give_asset"] == v)
                && filter.get.as_deref().is_none_or(|v| body["get_asset"] == v)
                && filter
                    .network
                    .as_deref()
                    .is_none_or(|v| body["network"] == v)
        })
        .collect();
    Ok(Json(json!({ "offers": offers })))
}

async fn revoke_offer(
    State(app): State<App>,
    Json(envelope): Json<Envelope>,
) -> Result<Json<Value>, ApiError> {
    verified(&envelope, "revoke")?;
    let db = app.db.lock().expect("db mutex");
    let changed = db.execute(
        "UPDATE offers SET revoked = 1 WHERE offer_id = ?1 AND identity = ?2",
        params![envelope.swap_id, envelope.from],
    )?;
    Ok(Json(json!({ "revoked": changed > 0 })))
}

#[derive(Deserialize)]
struct RelayPost {
    /// Recipient identity pubkey (x-only, hex).
    to: String,
    /// Opaque payload — a client-side-sealed envelope (PACTSEALED1: ephemeral
    /// ECDH + ChaCha20-Poly1305). The board never inspects it.
    blob: String,
}

async fn relay_post(
    State(app): State<App>,
    Json(message): Json<RelayPost>,
) -> Result<Json<Value>, ApiError> {
    if message.blob.len() > MAX_BLOB_BYTES {
        return Err(ApiError(anyhow::anyhow!(
            "blob exceeds {MAX_BLOB_BYTES} bytes"
        )));
    }
    if hex::decode(&message.to)
        .map(|b| b.len() != 32)
        .unwrap_or(true)
    {
        return Err(ApiError(anyhow::anyhow!(
            "`to` must be a 32-byte x-only pubkey in hex"
        )));
    }
    let db = app.db.lock().expect("db mutex");
    db.execute(
        "INSERT INTO relay (recipient, blob, created) VALUES (?1, ?2, ?3)",
        params![message.to, message.blob, now()],
    )?;
    let id: i64 = db.last_insert_rowid();
    Ok(Json(json!({ "id": id })))
}

async fn relay_poll(
    State(app): State<App>,
    Json(envelope): Json<Envelope>,
) -> Result<Json<Value>, ApiError> {
    // A signed poll proves the caller controls the recipient identity, so
    // strangers cannot read someone's coordination mail. Blobs are sealed
    // client-side (PACTSEALED1), so the board only ever stores ciphertext.
    verified(&envelope, "relay_poll")?;
    let since = envelope.body["since_id"].as_i64().unwrap_or(0);
    let db = app.db.lock().expect("db mutex");
    let mut stmt = db.prepare(
        "SELECT id, blob FROM relay WHERE recipient = ?1 AND id > ?2 ORDER BY id LIMIT 100",
    )?;
    let rows: Vec<(i64, String)> = stmt
        .query_map(params![envelope.from, since], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let messages: Vec<Value> = rows
        .into_iter()
        .map(|(id, blob)| json!({ "id": id, "blob": blob }))
        .collect();
    Ok(Json(json!({ "messages": messages })))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let db = open_db(&args.db).context("opening corkboard db")?;
    let app = App {
        db: Arc::new(Mutex::new(db)),
    };

    let router = Router::new()
        .route("/health", get(health))
        .route("/v1/offers", post(post_offer).get(list_offers))
        .route("/v1/offers/revoke", post(revoke_offer))
        .route("/v1/relay", post(relay_post))
        .route("/v1/relay/poll", post(relay_poll))
        .with_state(app);

    tracing::info!(listen = %args.listen, db = %args.db.display(), "corkboard listening");
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
