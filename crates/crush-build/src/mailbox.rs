//! Local mail catcher — a tiny embedded SMTP sink for development.
//!
//! Almost every app sends mail (signup, password reset, receipts). In dev you
//! don't want those going anywhere real, and you don't want to stand up Mailhog
//! in Docker. Crush listens on `localhost:1025`, accepts whatever an app sends,
//! and keeps it in memory so the GUI can show it under a Mailbox tab. Point your
//! app at `SMTP_HOST=localhost` / `SMTP_PORT=1025` (crush injects these on run)
//! and every outgoing email is captured instead of delivered.
//!
//! This is a *sink*, not a real MTA: no auth, no TLS, no relaying — it accepts
//! the SMTP conversation, swallows the message, and never forwards it. That's
//! exactly what you want for local development.

use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// The conventional dev SMTP-sink port (Mailhog/Mailpit use the same).
pub const DEFAULT_PORT: u16 = 1025;

/// One captured message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedMail {
    pub id: u64,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub date: String,
    /// Best-effort human-readable body (text/plain part when multipart).
    pub body: String,
    /// The complete RFC822 message as received.
    pub raw: String,
    pub received_ms: u64,
}

/// In-memory ring of captured messages, newest last. Cheap to clone (Arc).
#[derive(Clone, Default)]
pub struct MailStore {
    inner: Arc<Mutex<MailInner>>,
}

#[derive(Default)]
struct MailInner {
    messages: Vec<CapturedMail>,
    next_id: u64,
}

/// Keep memory bounded — a dev session won't need more than this.
const MAX_MESSAGES: usize = 500;

impl MailStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&self, mut mail: CapturedMail) -> CapturedMail {
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        mail.id = inner.next_id;
        inner.messages.push(mail.clone());
        if inner.messages.len() > MAX_MESSAGES {
            let overflow = inner.messages.len() - MAX_MESSAGES;
            inner.messages.drain(0..overflow);
        }
        mail
    }

    /// All captured messages, newest first.
    pub fn list(&self) -> Vec<CapturedMail> {
        let inner = self.inner.lock().unwrap();
        inner.messages.iter().rev().cloned().collect()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().messages.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Run the SMTP sink until the process ends. `on_new` is called with each
/// captured message (after it's stored) — use it to emit a UI event or print.
/// Binds `127.0.0.1:port`; returns an error if the port is already taken.
pub async fn serve<F>(port: u16, store: MailStore, on_new: F) -> std::io::Result<()>
where
    F: Fn(&CapturedMail) + Send + Sync + 'static,
{
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let on_new = Arc::new(on_new);
    loop {
        let (socket, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let store = store.clone();
        let on_new = on_new.clone();
        tokio::spawn(async move {
            if let Err(_e) = handle_session(socket, &store, on_new.as_ref()).await {
                // A misbehaving client shouldn't take down the catcher.
            }
        });
    }
}

/// Handle one SMTP conversation. Minimal but correct enough for app mailers.
async fn handle_session<F>(socket: TcpStream, store: &MailStore, on_new: &F) -> std::io::Result<()>
where
    F: Fn(&CapturedMail),
{
    let (read_half, mut write) = socket.into_split();
    let mut reader = BufReader::new(read_half);

    write.write_all(b"220 crush mail catcher ready\r\n").await?;

    let mut from = String::new();
    let mut rcpts: Vec<String> = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // client disconnected
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let upper = trimmed.to_ascii_uppercase();

        if upper.starts_with("EHLO") || upper.starts_with("HELO") {
            write.write_all(b"250 crush\r\n").await?;
        } else if upper.starts_with("MAIL FROM") {
            from = extract_addr(trimmed);
            write.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("RCPT TO") {
            rcpts.push(extract_addr(trimmed));
            write.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("DATA") {
            write.write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n").await?;
            let raw = read_data(&mut reader).await?;
            let mail = parse_message(&from, &rcpts, &raw);
            let stored = store.push(mail);
            on_new(&stored);
            write.write_all(b"250 OK message queued\r\n").await?;
            from.clear();
            rcpts.clear();
        } else if upper.starts_with("RSET") {
            from.clear();
            rcpts.clear();
            write.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("NOOP") {
            write.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("QUIT") {
            write.write_all(b"221 Bye\r\n").await?;
            break;
        } else if upper.starts_with("VRFY") || upper.starts_with("HELP") {
            write.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("STARTTLS") {
            // No TLS in a dev sink — tell the client to proceed in plaintext.
            write.write_all(b"454 TLS not available\r\n").await?;
        } else if upper.starts_with("AUTH") {
            // Accept any auth so clients configured with creds still work.
            write.write_all(b"235 Authentication successful\r\n").await?;
        } else {
            write.write_all(b"250 OK\r\n").await?;
        }
    }
    Ok(())
}

/// Read the DATA payload up to the terminating `.` line, undoing dot-stuffing.
async fn read_data<R: AsyncReadExt + AsyncBufReadExt + Unpin>(reader: &mut R) -> std::io::Result<String> {
    let mut data = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let body_line = line.trim_end_matches(['\r', '\n']);
        if body_line == "." {
            break;
        }
        // Dot-stuffing: a leading ".." represents a literal ".".
        let unstuffed = body_line.strip_prefix('.').unwrap_or(body_line);
        data.push_str(if body_line.starts_with("..") { unstuffed } else { body_line });
        data.push('\n');
    }
    Ok(data)
}

/// Parse `<addr>` out of `MAIL FROM:<a@b>` / `RCPT TO:<c@d>` (or bare forms).
fn extract_addr(line: &str) -> String {
    if let (Some(s), Some(e)) = (line.find('<'), line.find('>')) {
        if e > s {
            return line[s + 1..e].to_string();
        }
    }
    // Fall back to whatever follows the colon.
    line.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default()
}

/// Split an RFC822 message into headers + body and pull the common fields.
pub fn parse_message(envelope_from: &str, envelope_to: &[String], raw: &str) -> CapturedMail {
    let (headers_blk, body_blk) = match raw.split_once("\n\n") {
        Some((h, b)) => (h, b),
        None => (raw, ""),
    };

    let headers = parse_headers(headers_blk);
    let get = |k: &str| headers.iter().find(|(hk, _)| hk.eq_ignore_ascii_case(k)).map(|(_, v)| v.clone());

    let from = get("From").filter(|s| !s.is_empty()).unwrap_or_else(|| envelope_from.to_string());
    let to = get("To")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect::<Vec<_>>())
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| envelope_to.to_vec());
    let subject = get("Subject").unwrap_or_default();
    let date = get("Date").unwrap_or_default();

    let content_type = get("Content-Type").unwrap_or_default();
    let body = extract_text_body(&content_type, body_blk);

    CapturedMail {
        id: 0,
        from,
        to,
        subject,
        date,
        body,
        raw: raw.to_string(),
        received_ms: now_ms(),
    }
}

/// Parse `Key: value` headers, joining folded (leading-whitespace) continuations.
fn parse_headers(block: &str) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in block.lines() {
        if (line.starts_with(' ') || line.starts_with('\t')) && !headers.is_empty() {
            // Continuation of the previous header.
            let last = headers.last_mut().unwrap();
            last.1.push(' ');
            last.1.push_str(line.trim());
        } else if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    headers
}

/// For multipart messages, pull out the first text/plain section; otherwise
/// return the body as-is.
fn extract_text_body(content_type: &str, body: &str) -> String {
    let ct = content_type.to_ascii_lowercase();
    // Locate "boundary=" case-insensitively but read the *value* from the
    // original string — boundary delimiters are case-sensitive.
    if let Some(pos) = ct.find("boundary=") {
        let raw_val = &content_type[pos + "boundary=".len()..];
        let boundary = raw_val.split(';').next().unwrap_or(raw_val).trim().trim_matches('"').to_string();
        if !boundary.is_empty() {
            let sep = format!("--{boundary}");
            for part in body.split(&sep) {
                let part_lower = part.to_ascii_lowercase();
                if part_lower.contains("text/plain") {
                    // The part has its own headers; body follows the blank line.
                    if let Some((_, p_body)) = part.split_once("\n\n") {
                        return p_body.trim().to_string();
                    }
                }
            }
            // No text/plain found — fall through to raw body.
        }
    }
    body.trim().to_string()
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_addresses() {
        assert_eq!(extract_addr("MAIL FROM:<a@b.com>"), "a@b.com");
        assert_eq!(extract_addr("RCPT TO:<c@d.com> SIZE=10"), "c@d.com");
        assert_eq!(extract_addr("MAIL FROM: bare@x.com"), "bare@x.com");
    }

    #[test]
    fn parses_basic_message() {
        let raw = "From: app@example.com\nTo: user@example.com\nSubject: Welcome\nDate: Mon, 1 Jan 2026 00:00:00 +0000\n\nHello there!\n";
        let m = parse_message("envelope@x", &["env-to@x".into()], raw);
        assert_eq!(m.from, "app@example.com");
        assert_eq!(m.to, vec!["user@example.com"]);
        assert_eq!(m.subject, "Welcome");
        assert_eq!(m.body.trim(), "Hello there!");
    }

    #[test]
    fn falls_back_to_envelope_when_headers_missing() {
        let raw = "Subject: NoFrom\n\nbody";
        let m = parse_message("env@from.com", &["env@to.com".into()], raw);
        assert_eq!(m.from, "env@from.com");
        assert_eq!(m.to, vec!["env@to.com"]);
    }

    #[test]
    fn extracts_text_plain_from_multipart() {
        let ct = "multipart/alternative; boundary=\"XYZ\"";
        let body = "--XYZ\nContent-Type: text/plain\n\nplain version\n--XYZ\nContent-Type: text/html\n\n<b>html version</b>\n--XYZ--";
        assert_eq!(extract_text_body(ct, body), "plain version");
    }

    #[test]
    fn store_caps_and_orders_newest_first() {
        let store = MailStore::new();
        for i in 0..3 {
            let raw = format!("Subject: msg{i}\n\nbody{i}");
            let m = parse_message("a@b", &["c@d".into()], &raw);
            store.push(m);
        }
        let list = store.list();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].subject, "msg2"); // newest first
        assert_eq!(list[0].id, 3);
        store.clear();
        assert!(store.is_empty());
    }
}
