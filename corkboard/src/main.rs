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
//!   POST /v1/relay            signed relay_post envelope with {to, blob, created}
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
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Reject anything bigger than this — the relay is for coordination
/// envelopes, not file transfer.
const MAX_BLOB_BYTES: usize = 64 * 1024;
const DEFAULT_OFFER_TTL_SECS: u64 = 24 * 3600;
const MAX_OFFER_TTL_SECS: u64 = 7 * 24 * 3600;

/// Expiry is signed, never renewed by replaying a publication to the server.
fn offer_window(envelope: &Envelope, clock: u64) -> Result<(u64, u64)> {
    let created = envelope.body["created"]
        .as_u64()
        .context("offer requires signed created time")?;
    anyhow::ensure!(
        created > 0 && created <= clock.saturating_add(300),
        "invalid offer creation time"
    );
    let ttl = match envelope.body.get("ttl_secs") {
        None | Some(Value::Null) => DEFAULT_OFFER_TTL_SECS,
        Some(value) => value.as_u64().context("invalid offer TTL")?,
    }
    .min(MAX_OFFER_TTL_SECS);
    anyhow::ensure!(ttl > 0, "offer TTL must be positive");
    let expires = created.checked_add(ttl).context("offer expiry overflow")?;
    anyhow::ensure!(expires > clock, "offer expired");
    Ok((created, expires))
}

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
    let has_sender = conn
        .prepare("PRAGMA table_info(relay)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == "sender");
    if !has_sender {
        conn.execute(
            "ALTER TABLE relay ADD COLUMN sender TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS relay_sender ON relay(sender)",
        [],
    )?;
    // Migrate receipt-time lifetimes from older boards, including surviving
    // revocation rows. Already-deleted historical revocations cannot be recovered.
    let rows = conn
        .prepare("SELECT offer_id, envelope FROM offers")?
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, raw) in rows {
        let window = serde_json::from_str::<Envelope>(&raw)
            .map_err(anyhow::Error::from)
            .and_then(|envelope| offer_window(&envelope, now()));
        match window {
            Ok((created, expires)) => {
                conn.execute(
                    "UPDATE offers SET created=?2, expires=?3 WHERE offer_id=?1",
                    params![id, created, expires],
                )?;
            }
            Err(_) => {
                conn.execute("DELETE FROM offers WHERE offer_id=?1", params![id])?;
            }
        }
    }
    Ok(conn)
}

fn prune(db: &Connection) -> Result<()> {
    db.execute("DELETE FROM offers WHERE expires <= ?1", params![now()])?;
    db.execute(
        "DELETE FROM relay WHERE created < ?1",
        params![now().saturating_sub(7 * 86400)],
    )?;
    Ok(())
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
    let (created, expires) = offer_window(&envelope, now())?;
    let db = app.db.lock().expect("db mutex");
    prune(&db)?;
    // A revoked row is a tombstone for the entire signed validity window.
    // Check before quotas so a live byte-identical retry still succeeds at quota.
    let existing: Option<(String, String, bool)> = db
        .query_row(
            "SELECT identity, envelope, revoked FROM offers WHERE offer_id=?1",
            params![envelope.swap_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((identity, raw, revoked)) = existing {
        if revoked {
            return Err(anyhow::anyhow!("offer revoked").into());
        }
        if identity != envelope.from || serde_json::from_str::<Envelope>(&raw)? != envelope {
            return Err(anyhow::anyhow!("conflicting offer identity").into());
        }
        return Ok(Json(json!({ "offer_id": envelope.swap_id })));
    }
    let total: i64 = db.query_row("SELECT COUNT(*) FROM offers", [], |r| r.get(0))?;
    let own: i64 = db.query_row(
        "SELECT COUNT(*) FROM offers WHERE identity = ?1",
        params![envelope.from],
        |r| r.get(0),
    )?;
    if total >= 4096 || own >= 128 {
        return Err(ApiError(anyhow::anyhow!("offer storage quota exceeded")));
    }
    db.execute(
        "INSERT INTO offers (offer_id, identity, envelope, created, expires)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(offer_id) DO NOTHING",
        params![
            envelope.swap_id,
            envelope.from,
            serde_json::to_string(&envelope)?,
            created,
            expires
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
        "SELECT envelope FROM offers WHERE revoked = 0 AND expires > ?1
         AND (?2 IS NULL OR json_extract(envelope, '$.body.give_asset') = ?2)
         AND (?3 IS NULL OR json_extract(envelope, '$.body.get_asset') = ?3)
         AND (?4 IS NULL OR json_extract(envelope, '$.body.network') = ?4)
         ORDER BY created DESC LIMIT 500",
    )?;
    let rows: Vec<String> = stmt
        .query_map(
            params![now(), filter.give, filter.get, filter.network],
            |row| row.get(0),
        )?
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
    Json(envelope): Json<Envelope>,
) -> Result<Json<Value>, ApiError> {
    verified(&envelope, "relay_post")?;
    let created = envelope.body["created"].as_u64().unwrap_or(0);
    if created > now().saturating_add(300) || created.saturating_add(86400) < now() {
        return Err(ApiError(anyhow::anyhow!(
            "relay post expired or future dated"
        )));
    }
    let message: RelayPost = serde_json::from_value(envelope.body.clone())?;
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
    prune(&db)?;
    // Retries remain idempotent even when the recipient is at its quota.
    let existing: Option<i64> = db
        .query_row(
            "SELECT id FROM relay WHERE recipient = ?1 AND blob = ?2",
            params![message.to, message.blob],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(Json(json!({ "id": id })));
    }
    let count: i64 = db.query_row("SELECT COUNT(*) FROM relay", [], |r| r.get(0))?;
    let recipient: i64 = db.query_row(
        "SELECT COUNT(*) FROM relay WHERE recipient = ?1",
        params![message.to],
        |r| r.get(0),
    )?;
    let sender: i64 = db.query_row(
        "SELECT COUNT(*) FROM relay WHERE sender = ?1",
        params![envelope.from],
        |r| r.get(0),
    )?;
    if count >= 8192 || recipient >= 256 || sender >= 256 {
        return Err(ApiError(anyhow::anyhow!("relay storage quota exceeded")));
    }
    db.execute(
        "INSERT INTO relay (recipient, blob, created, sender) SELECT ?1, ?2, ?3, ?4 WHERE NOT EXISTS (SELECT 1 FROM relay WHERE recipient = ?1 AND blob = ?2)",
        params![message.to, message.blob, now(), envelope.from],
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
    // Reset hygiene: if the caller's cursor is ahead of everything we still hold
    // for them, the board was wiped/replaced since they last polled — serve from
    // the start so a stale cursor can't silently swallow every message on the
    // fresh board. ids are AUTOINCREMENT (never reused), so cursor > max only
    // happens on a real DB reset, not on ordinary pruning. The poll is signed,
    // so this only ever re-serves the caller's own mail.
    let max_id: i64 = db.query_row(
        "SELECT IFNULL(MAX(id), 0) FROM relay WHERE recipient = ?1",
        params![envelope.from],
        |row| row.get(0),
    )?;
    let since = if since > max_id { 0 } else { since };
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

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Keypair, Secp256k1, SecretKey};

    fn app() -> App {
        App {
            db: Arc::new(Mutex::new(open_db(&PathBuf::from(":memory:")).unwrap())),
        }
    }

    fn message(blob: &str, created: u64) -> Envelope {
        let secp = Secp256k1::new();
        let key = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[7; 32]).unwrap());
        let mut envelope = Envelope {
            v: 1,
            msg_type: "relay_post".into(),
            swap_id: "relay".into(),
            from: key.x_only_public_key().0.to_string(),
            body: json!({"to": key.x_only_public_key().0.to_string(), "blob": blob, "created": created}),
            sig: String::new(),
        };
        pact_proto::envelope::sign(&mut envelope, &key).unwrap();
        envelope
    }

    fn offer(id: &str, created: u64, ttl: u64) -> Envelope {
        let key =
            Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[7; 32]).unwrap());
        let mut envelope = message("", created);
        envelope.msg_type = "offer".into();
        envelope.swap_id = id.into();
        envelope.body = json!({"created": created, "ttl_secs": ttl});
        pact_proto::envelope::sign(&mut envelope, &key).unwrap();
        envelope
    }

    #[test]
    fn signed_offer_validity_is_bounded_and_cannot_be_renewed() {
        let clock = 1_000_000;
        assert_eq!(
            offer_window(&offer("a", clock - 100, 200), clock).unwrap(),
            (clock - 100, clock + 100)
        );
        assert!(offer_window(&offer("a", clock - 200, 200), clock).is_err());
        assert!(offer_window(&offer("a", clock + 301, 200), clock).is_err());
        assert!(offer_window(&offer("a", clock, 0), clock).is_err());
        assert!(offer_window(&offer("a", u64::MAX, 1), u64::MAX).is_err());
        assert_eq!(
            offer_window(&offer("a", clock, u64::MAX), clock).unwrap().1,
            clock + MAX_OFFER_TTL_SECS
        );
        let mut envelope = offer("a", clock, 1);
        envelope.body = json!({"created": clock});
        assert_eq!(
            offer_window(&envelope, clock).unwrap().1,
            clock + DEFAULT_OFFER_TTL_SECS
        );
        envelope.body = json!({});
        assert!(offer_window(&envelope, clock).is_err());
    }

    #[tokio::test]
    async fn live_offer_retry_succeeds_at_quota_without_renewing_expiry() {
        let app = app();
        let created = now() - 100;
        let original = offer("original", created, 3600);
        assert!(post_offer(State(app.clone()), Json(original.clone()))
            .await
            .is_ok());
        for i in 0..127 {
            assert!(post_offer(
                State(app.clone()),
                Json(offer(&format!("filler-{i}"), created, 3600))
            )
            .await
            .is_ok());
        }
        assert!(
            post_offer(State(app.clone()), Json(offer("overflow", created, 3600)))
                .await
                .is_err()
        );
        assert!(post_offer(State(app.clone()), Json(original)).await.is_ok());
        let expires: u64 = app
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT expires FROM offers WHERE offer_id='original'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(expires, created + 3600);
        assert!(
            post_offer(State(app), Json(offer("original", created + 1, 3600)))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn revocation_survives_prune_restart_and_receipt_time_migration() {
        let path = std::env::temp_dir().join(format!(
            "corkboard-revoke-{}-{}.sqlite",
            std::process::id(),
            now()
        ));
        let app = App {
            db: Arc::new(Mutex::new(open_db(&path).unwrap())),
        };
        let created = now() - 100;
        let original = offer("revoked", created, 3600);
        assert!(post_offer(State(app.clone()), Json(original.clone()))
            .await
            .is_ok());
        let mut revocation = original.clone();
        revocation.msg_type = "revoke".into();
        revocation.body = json!({});
        let key =
            Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[7; 32]).unwrap());
        pact_proto::envelope::sign(&mut revocation, &key).unwrap();
        assert!(revoke_offer(State(app.clone()), Json(revocation))
            .await
            .is_ok());
        {
            let db = app.db.lock().unwrap();
            prune(&db).unwrap();
            db.execute(
                "UPDATE offers SET created=?1, expires=?2",
                params![now(), now() + 3600],
            )
            .unwrap();
        }
        drop(app);
        let app = App {
            db: Arc::new(Mutex::new(open_db(&path).unwrap())),
        };
        let row: (u64, u64, bool) = app
            .db
            .lock()
            .unwrap()
            .query_row("SELECT created, expires, revoked FROM offers", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(row, (created, created + 3600, true));
        let rejected = post_offer(State(app.clone()), Json(original))
            .await
            .err()
            .unwrap();
        assert!(rejected.0.to_string().contains("revoked"));
        assert!(post_offer(
            State(app.clone()),
            Json(offer("expired", now() - 3600, 3600))
        )
        .await
        .is_err());
        drop(app);
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn one_sender_cannot_fill_global_quota_using_many_recipients() {
        let app = app();
        let key =
            Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[7; 32]).unwrap());
        for i in 0..257 {
            let mut envelope = message("sealed", now());
            envelope.body["to"] = json!(format!("{i:064x}"));
            pact_proto::envelope::sign(&mut envelope, &key).unwrap();
            let result = relay_post(State(app.clone()), Json(envelope)).await;
            assert_eq!(result.is_ok(), i < 256);
        }
        let count: i64 = app
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM relay", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 256);
    }

    #[tokio::test]
    async fn relay_rejects_tampering_expiry_and_oversized_blobs() {
        let app = app();
        let mut tampered = message("sealed", now());
        tampered.body["blob"] = json!("changed");
        assert!(relay_post(State(app.clone()), Json(tampered))
            .await
            .is_err());
        assert!(
            relay_post(State(app.clone()), Json(message("old", now() - 86401)))
                .await
                .is_err()
        );
        assert!(
            relay_post(State(app.clone()), Json(message("future", now() + 600)))
                .await
                .is_err()
        );
        assert!(relay_post(
            State(app.clone()),
            Json(message(&"x".repeat(MAX_BLOB_BYTES + 1), now()))
        )
        .await
        .is_err());
        let count: i64 = app
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM relay", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn relay_quota_pruning_and_retry_identity() {
        let app = app();
        let original = message("original", now());
        let first = relay_post(State(app.clone()), Json(original.clone()))
            .await
            .ok()
            .unwrap()
            .0;
        {
            let db = app.db.lock().unwrap();
            for i in 0..255 {
                db.execute(
                    "INSERT INTO relay(recipient, blob, created) VALUES (?1, ?2, ?3)",
                    params![original.from, format!("filler-{i}"), now()],
                )
                .unwrap();
            }
        }
        let retry = relay_post(State(app.clone()), Json(original.clone()))
            .await
            .ok()
            .unwrap()
            .0;
        assert_eq!(first, retry);
        assert!(
            relay_post(State(app.clone()), Json(message("over quota", now())))
                .await
                .is_err()
        );
        app.db
            .lock()
            .unwrap()
            .execute(
                "UPDATE relay SET created = ?1 WHERE blob = 'filler-0'",
                params![now() - 8 * 86400],
            )
            .unwrap();
        assert!(
            relay_post(State(app.clone()), Json(message("after pruning", now())))
                .await
                .is_ok()
        );
        let count: i64 = app
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM relay", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 256);
    }
}
