//! Telegram side: plain Bot API over the reqwest client we already carry —
//! no SDK. Announcements are pushed via `sendMessage` (HTML parse mode);
//! commands (/book, /top, /status) are served by a `getUpdates` long-poll
//! loop, so the bot works in DMs and groups alike.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::book::Book;
use crate::cash::CashRate;
use crate::config::Config;
use crate::views;

pub struct Telegram {
    client: reqwest::Client,
    base: String,
}

/// Telegram chat ids are numeric for chats/groups and `@name` for public
/// channels — send whichever the config holds.
fn chat_value(chat_id: &str) -> serde_json::Value {
    match chat_id.parse::<i64>() {
        Ok(n) => serde_json::Value::from(n),
        Err(_) => serde_json::Value::from(chat_id),
    }
}

/// Convert our (deliberately tiny) markdown dialect to Telegram HTML:
/// `**bold**`, `` `code` ``, `*italic*`, ``` fences. We control every input
/// string, so a simple toggle scanner is exact.
pub fn md_to_html(md: &str) -> String {
    let escaped = md
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let mut out = String::new();
    let (mut bold, mut italic, mut code, mut pre) = (false, false, false, false);
    let mut chars = escaped.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' if chars.peek() == Some(&'`') => {
                chars.next();
                chars.next(); // ``` fence
                out.push_str(if pre { "</pre>" } else { "<pre>" });
                pre = !pre;
            }
            '`' if !pre => {
                out.push_str(if code { "</code>" } else { "<code>" });
                code = !code;
            }
            '*' if !pre && chars.peek() == Some(&'*') => {
                chars.next();
                out.push_str(if bold { "</b>" } else { "<b>" });
                bold = !bold;
            }
            '*' if !pre => {
                out.push_str(if italic { "</i>" } else { "<i>" });
                italic = !italic;
            }
            _ => out.push(c),
        }
    }
    out
}

impl Telegram {
    pub fn new(token: &str) -> Result<Arc<Telegram>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(45)) // > getUpdates long-poll timeout
            .build()
            .context("telegram http client")?;
        Ok(Arc::new(Telegram {
            client,
            base: format!("https://api.telegram.org/bot{token}"),
        }))
    }

    /// `thread_id` targets a topic inside a forum supergroup (0 = General /
    /// plain chats).
    pub async fn send_html(&self, chat_id: &str, thread_id: u64, html: &str) -> Result<()> {
        let mut payload = serde_json::json!({
            "chat_id": chat_value(chat_id),
            "text": html,
            "parse_mode": "HTML",
            "disable_web_page_preview": true,
        });
        if thread_id != 0 {
            payload["message_thread_id"] = serde_json::Value::from(thread_id);
        }
        let resp = self
            .client
            .post(format!("{}/sendMessage", self.base))
            .json(&payload)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("sendMessage {status}: {body}");
        }
        Ok(())
    }

    /// Serve /book, /top, /status via getUpdates long-polling.
    pub async fn run_commands(
        self: Arc<Telegram>,
        book: Arc<RwLock<Book>>,
        cfg: Arc<Config>,
        cash: CashRate,
        nostr_client: nostr_sdk::Client,
        started: u64,
    ) {
        let mut offset: i64 = 0;
        loop {
            let updates = match self.get_updates(offset).await {
                Ok(u) => u,
                Err(err) => {
                    tracing::warn!("telegram: getUpdates: {err:#}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            for update in updates {
                if let Some(id) = update.get("update_id").and_then(|v| v.as_i64()) {
                    offset = offset.max(id + 1);
                }
                let Some(message) = update.get("message") else {
                    continue;
                };
                let Some(chat_id) = message.pointer("/chat/id").and_then(|v| v.as_i64()) else {
                    continue;
                };
                let Some(text) = message.get("text").and_then(|v| v.as_str()) else {
                    continue;
                };
                // Reply into the same forum topic the command came from.
                let thread_id = message
                    .get("message_thread_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if let Some(reply_md) = self
                    .handle_command(text, &book, &cfg, &cash, &nostr_client, started)
                    .await
                {
                    let html = md_to_html(&reply_md);
                    if let Err(err) = self.send_html(&chat_id.to_string(), thread_id, &html).await {
                        tracing::warn!("telegram: reply failed: {err:#}");
                    }
                }
            }
        }
    }

    async fn get_updates(&self, offset: i64) -> Result<Vec<serde_json::Value>> {
        let value: serde_json::Value = self
            .client
            .get(format!("{}/getUpdates", self.base))
            .query(&[
                ("timeout", "30".to_string()),
                ("offset", offset.to_string()),
                ("allowed_updates", "[\"message\"]".to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(value
            .get("result")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    }

    async fn handle_command(
        &self,
        text: &str,
        book: &Arc<RwLock<Book>>,
        cfg: &Config,
        cash: &CashRate,
        nostr_client: &nostr_sdk::Client,
        started: u64,
    ) -> Option<String> {
        let mut words = text.split_whitespace();
        // "/book@CrierBot btcx/btc" → command "book", arg "btcx/btc".
        let command = words.next()?.strip_prefix('/')?.split('@').next()?;
        match command {
            "book" => Some(match views::book_view(book, cfg, cash, words.next()) {
                Ok(md) => md,
                Err(err) => format!("{err:#}"),
            }),
            "top" => Some(views::top_view(book, cfg, cash)),
            "status" | "start" => {
                Some(views::status_view(book, cfg, cash, nostr_client, started).await)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_to_html_covers_our_dialect() {
        assert_eq!(
            md_to_html("**Ask** `37 BTCX @ 0.676` *(was 0.691)*"),
            "<b>Ask</b> <code>37 BTCX @ 0.676</code> <i>(was 0.691)</i>"
        );
        assert_eq!(
            md_to_html("**B/Q** — legend\n```\n  ASK 1 < 2 & 3\n```"),
            "<b>B/Q</b> — legend\n<pre>\n  ASK 1 &lt; 2 &amp; 3\n</pre>"
        );
    }
}
