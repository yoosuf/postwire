use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{MessageSummary, SmsMessage};

type MessageSummaryRow = (String, String, String, i64, String, i64, i64, i64);

pub struct Store {
    conn: Mutex<Connection>,
    max_messages: u64,
    ttl_seconds: u64,
}

pub struct NewMessage {
    pub id: String,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub size: i64,
    pub raw: Vec<u8>,
}

pub struct NewSms {
    pub id: String,
    pub from: String,
    pub to: String,
    pub body: String,
}

impl Store {
    pub fn new(db_path: &str, max_messages: u64) -> Result<Self> {
        Self::new_with_ttl(db_path, max_messages, 0)
    }

    pub fn new_with_ttl(db_path: &str, max_messages: u64, ttl_seconds: u64) -> Result<Self> {
        let conn = Connection::open(db_path).context("failed to open database")?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                from_addr TEXT NOT NULL,
                to_addrs TEXT NOT NULL,
                subject TEXT NOT NULL,
                size INTEGER NOT NULL,
                received_at TEXT NOT NULL,
                read INTEGER NOT NULL DEFAULT 0,
                has_html INTEGER NOT NULL DEFAULT 0,
                has_attachments INTEGER NOT NULL DEFAULT 0,
                raw BLOB NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_messages_received_at ON messages(received_at);
             CREATE TABLE IF NOT EXISTS sms (
                id TEXT PRIMARY KEY,
                from_phone TEXT NOT NULL,
                to_phone TEXT NOT NULL,
                body TEXT NOT NULL,
                received_at TEXT NOT NULL,
                read INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_sms_received_at ON sms(received_at);",
        )?;
        let _ = conn.execute("ALTER TABLE messages ADD COLUMN has_html INTEGER NOT NULL DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE messages ADD COLUMN has_attachments INTEGER NOT NULL DEFAULT 0", []);

        Ok(Self {
            conn: Mutex::new(conn),
            max_messages,
            ttl_seconds,
        })
    }

    pub fn counts(&self) -> Result<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let email_count: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
        let sms_count: i64 = conn.query_row("SELECT COUNT(*) FROM sms", [], |r| r.get(0))?;
        Ok((email_count, sms_count))
    }

    pub fn insert(&self, msg: NewMessage) -> Result<MessageSummary> {
        let conn = self.conn.lock().unwrap();
        let received_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let to_json = serde_json::to_string(&msg.to)?;
        let (has_html, has_attachments) = crate::mail::body_flags(&msg.raw);

        conn.execute(
            "INSERT INTO messages (id, from_addr, to_addrs, subject, size, received_at, read, has_html, has_attachments, raw)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8, ?9)",
            params![
                msg.id,
                msg.from,
                to_json,
                msg.subject,
                msg.size,
                received_at,
                has_html as i64,
                has_attachments as i64,
                msg.raw
            ],
        )?;

        if self.ttl_seconds > 0 {
            let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(self.ttl_seconds as i64))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let _ = conn.execute("DELETE FROM messages WHERE received_at < ?1", params![cutoff]);
        }

        if self.max_messages > 0 {
            conn.execute(
                "DELETE FROM messages WHERE id IN (
                    SELECT id FROM messages ORDER BY received_at DESC LIMIT -1 OFFSET ?1
                )",
                params![self.max_messages as i64],
            )?;
        }

        Ok(MessageSummary {
            id: msg.id,
            from: msg.from,
            to: msg.to,
            subject: msg.subject,
            size: msg.size,
            received_at,
            read: false,
            has_html,
            has_attachments,
        })
    }

    pub fn list(&self, search: Option<&str>, limit: i64, offset: i64) -> Result<(Vec<MessageSummary>, i64)> {
        let conn = self.conn.lock().unwrap();
        let (where_clause, params) = parse_email_search(search);

        let count_sql = format!("SELECT COUNT(*) FROM messages {where_clause}");
        let mut count_stmt = conn.prepare(&count_sql)?;
        let count_params: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let total: i64 = count_stmt.query_row(count_params.as_slice(), |r| r.get(0))?;

        let list_sql = format!(
            "SELECT id, from_addr, to_addrs, subject, size, received_at, read, has_html, has_attachments
             FROM messages {where_clause} ORDER BY received_at DESC LIMIT ? OFFSET ?"
        );
        let mut stmt = conn.prepare(&list_sql)?;
        let mut query_params: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        query_params.push(&limit);
        query_params.push(&offset);

        let rows = stmt.query_map(query_params.as_slice(), |row| {
            let to_json: String = row.get(2)?;
            Ok(MessageSummary {
                id: row.get(0)?,
                from: row.get(1)?,
                to: serde_json::from_str(&to_json).unwrap_or_default(),
                subject: row.get(3)?,
                size: row.get(4)?,
                received_at: row.get(5)?,
                read: row.get::<_, i64>(6)? != 0,
                has_html: row.get::<_, i64>(7)? != 0,
                has_attachments: row.get::<_, i64>(8)? != 0,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }

        Ok((messages, total))
    }

    /// Finds the oldest message matching the given (optional, AND-combined) filters.
    /// Used by the `/api/wait` long-poll endpoint so agents/tests can wait for a fresh email.
    pub fn find_matching(
        &self,
        to: Option<&str>,
        from: Option<&str>,
        subject: Option<&str>,
        since: Option<&str>,
    ) -> Result<Option<MessageSummary>> {
        let conn = self.conn.lock().unwrap();

        let mut clauses = Vec::new();
        let mut values: Vec<String> = Vec::new();
        if let Some(to) = to {
            clauses.push("to_addrs LIKE ?");
            values.push(format!("%{to}%"));
        }
        if let Some(from) = from {
            clauses.push("from_addr LIKE ?");
            values.push(format!("%{from}%"));
        }
        if let Some(subject) = subject {
            clauses.push("subject LIKE ?");
            values.push(format!("%{subject}%"));
        }
        if let Some(since) = since {
            clauses.push("received_at > ?");
            values.push(since.to_string());
        }

        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT id, from_addr, to_addrs, subject, size, received_at, read, has_html, has_attachments
             FROM messages {where_clause} ORDER BY received_at ASC LIMIT 1"
        );

        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let row = stmt
            .query_row(params.as_slice(), |row| {
                let to_json: String = row.get(2)?;
                Ok(MessageSummary {
                    id: row.get(0)?,
                    from: row.get(1)?,
                    to: serde_json::from_str(&to_json).unwrap_or_default(),
                    subject: row.get(3)?,
                    size: row.get(4)?,
                    received_at: row.get(5)?,
                    read: row.get::<_, i64>(6)? != 0,
                    has_html: row.get::<_, i64>(7)? != 0,
                    has_attachments: row.get::<_, i64>(8)? != 0,
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn get_raw(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();
        let raw: Option<Vec<u8>> = conn
            .query_row("SELECT raw FROM messages WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?;
        Ok(raw)
    }

    pub fn get_summary(&self, id: &str) -> Result<Option<MessageSummary>> {
        let conn = self.conn.lock().unwrap();
        let row: Option<MessageSummaryRow> = conn
            .query_row(
                "SELECT from_addr, to_addrs, subject, size, received_at, read, has_html, has_attachments FROM messages WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
            )
            .optional()?;
        Ok(row.map(|(from, to_json, subject, size, received_at, read, has_html, has_attachments)| MessageSummary {
            id: id.to_string(),
            from,
            to: serde_json::from_str(&to_json).unwrap_or_default(),
            subject,
            size,
            received_at,
            read: read != 0,
            has_html: has_html != 0,
            has_attachments: has_attachments != 0,
        }))
    }


    pub fn mark_read(&self, id: &str, read: bool) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE messages SET read = ?1 WHERE id = ?2",
            params![read as i64, id],
        )?;
        Ok(n > 0)
    }

    /// Bulk-marks read/unread. Capped at `BULK_LIMIT` ids per call so a single
    /// request can't blow past SQLite's bound-parameter limit or hold the lock too long.
    pub fn mark_read_many(&self, ids: &[String], read: bool) -> Result<usize> {
        let ids = &ids[..ids.len().min(BULK_LIMIT)];
        if ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("UPDATE messages SET read = ? WHERE id IN ({placeholders})");
        let read_i64: i64 = read as i64;
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 1);
        params.push(&read_i64);
        params.extend(ids.iter().map(|id| id as &dyn rusqlite::ToSql));
        Ok(conn.execute(&sql, params.as_slice())?)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM messages WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    /// Bulk-deletes. Capped at `BULK_LIMIT` ids per call for the same reason as `mark_read_many`.
    pub fn delete_many(&self, ids: &[String]) -> Result<usize> {
        let ids = &ids[..ids.len().min(BULK_LIMIT)];
        if ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("DELETE FROM messages WHERE id IN ({placeholders})");
        let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        Ok(conn.execute(&sql, params.as_slice())?)
    }

    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM messages", [])?;
        Ok(())
    }

    // --- SMS Operations ---

    pub fn insert_sms(&self, sms: NewSms) -> Result<SmsMessage> {
        let conn = self.conn.lock().unwrap();
        let received_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "INSERT INTO sms (id, from_phone, to_phone, body, received_at, read)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![sms.id, sms.from, sms.to, sms.body, received_at],
        )?;

        if self.ttl_seconds > 0 {
            let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(self.ttl_seconds as i64))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let _ = conn.execute("DELETE FROM sms WHERE received_at < ?1", params![cutoff]);
        }

        if self.max_messages > 0 {
            conn.execute(
                "DELETE FROM sms WHERE id IN (
                    SELECT id FROM sms ORDER BY received_at DESC LIMIT -1 OFFSET ?1
                )",
                params![self.max_messages as i64],
            )?;
        }

        Ok(SmsMessage {
            id: sms.id,
            from: sms.from,
            to: sms.to,
            body: sms.body,
            received_at,
            read: false,
        })
    }

    pub fn list_sms(&self, search: Option<&str>, limit: i64, offset: i64) -> Result<(Vec<SmsMessage>, i64)> {
        let conn = self.conn.lock().unwrap();
        let (where_clause, params) = parse_sms_search(search);

        let count_sql = format!("SELECT COUNT(*) FROM sms {where_clause}");
        let mut count_stmt = conn.prepare(&count_sql)?;
        let count_params: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let total: i64 = count_stmt.query_row(count_params.as_slice(), |r| r.get(0))?;

        let list_sql = format!(
            "SELECT id, from_phone, to_phone, body, received_at, read
             FROM sms {where_clause} ORDER BY received_at DESC LIMIT ? OFFSET ?"
        );
        let mut stmt = conn.prepare(&list_sql)?;
        let mut query_params: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        query_params.push(&limit);
        query_params.push(&offset);

        let rows = stmt.query_map(query_params.as_slice(), |row| {
            Ok(SmsMessage {
                id: row.get(0)?,
                from: row.get(1)?,
                to: row.get(2)?,
                body: row.get(3)?,
                received_at: row.get(4)?,
                read: row.get::<_, i64>(5)? != 0,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }

        Ok((messages, total))
    }

    pub fn get_sms(&self, id: &str) -> Result<Option<SmsMessage>> {
        let conn = self.conn.lock().unwrap();
        let row: Option<(String, String, String, String, i64)> = conn
            .query_row(
                "SELECT from_phone, to_phone, body, received_at, read FROM sms WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;
        Ok(row.map(|(from, to, body, received_at, read)| SmsMessage {
            id: id.to_string(),
            from,
            to,
            body,
            received_at,
            read: read != 0,
        }))
    }

    pub fn find_matching_sms(
        &self,
        to: Option<&str>,
        from: Option<&str>,
        body: Option<&str>,
        since: Option<&str>,
    ) -> Result<Option<SmsMessage>> {
        let conn = self.conn.lock().unwrap();

        let mut clauses = Vec::new();
        let mut values: Vec<String> = Vec::new();
        if let Some(to) = to {
            clauses.push("to_phone LIKE ?");
            values.push(format!("%{to}%"));
        }
        if let Some(from) = from {
            clauses.push("from_phone LIKE ?");
            values.push(format!("%{from}%"));
        }
        if let Some(body) = body {
            clauses.push("body LIKE ?");
            values.push(format!("%{body}%"));
        }
        if let Some(since) = since {
            clauses.push("received_at > ?");
            values.push(since.to_string());
        }

        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT id, from_phone, to_phone, body, received_at, read
             FROM sms {where_clause} ORDER BY received_at ASC LIMIT 1"
        );

        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let row = stmt
            .query_row(params.as_slice(), |row| {
                Ok(SmsMessage {
                    id: row.get(0)?,
                    from: row.get(1)?,
                    to: row.get(2)?,
                    body: row.get(3)?,
                    received_at: row.get(4)?,
                    read: row.get::<_, i64>(5)? != 0,
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn mark_sms_read(&self, id: &str, read: bool) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE sms SET read = ?1 WHERE id = ?2",
            params![read as i64, id],
        )?;
        Ok(n > 0)
    }

    pub fn mark_sms_read_many(&self, ids: &[String], read: bool) -> Result<usize> {
        let ids = &ids[..ids.len().min(BULK_LIMIT)];
        if ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("UPDATE sms SET read = ? WHERE id IN ({placeholders})");
        let read_i64: i64 = read as i64;
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 1);
        params.push(&read_i64);
        params.extend(ids.iter().map(|id| id as &dyn rusqlite::ToSql));
        Ok(conn.execute(&sql, params.as_slice())?)
    }

    pub fn delete_sms(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM sms WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    pub fn delete_sms_many(&self, ids: &[String]) -> Result<usize> {
        let ids = &ids[..ids.len().min(BULK_LIMIT)];
        if ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("DELETE FROM sms WHERE id IN ({placeholders})");
        let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        Ok(conn.execute(&sql, params.as_slice())?)
    }

    pub fn clear_sms(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sms", [])?;
        Ok(())
    }
}


/// Max ids accepted per bulk operation, regardless of what the client sends.
const BULK_LIMIT: usize = 1000;

fn parse_email_search(search: Option<&str>) -> (String, Vec<String>) {
    let Some(search) = search else {
        return (String::new(), Vec::new());
    };
    let s = search.trim();
    if s.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut clauses = Vec::new();
    let mut values = Vec::new();

    for token in s.split_whitespace() {
        if let Some((key, val)) = token.split_once(':') {
            match key.to_lowercase().as_str() {
                "from" => {
                    clauses.push("from_addr LIKE ?".to_string());
                    values.push(format!("%{val}%"));
                }
                "to" => {
                    clauses.push("to_addrs LIKE ?".to_string());
                    values.push(format!("%{val}%"));
                }
                "subject" => {
                    clauses.push("subject LIKE ?".to_string());
                    values.push(format!("%{val}%"));
                }
                "has" => match val.to_lowercase().as_str() {
                    "attachment" | "attachments" => clauses.push("has_attachments = 1".to_string()),
                    "html" => clauses.push("has_html = 1".to_string()),
                    _ => {
                        clauses.push("(from_addr LIKE ? OR to_addrs LIKE ? OR subject LIKE ?)".to_string());
                        values.push(format!("%{token}%"));
                        values.push(format!("%{token}%"));
                        values.push(format!("%{token}%"));
                    }
                },
                "is" => match val.to_lowercase().as_str() {
                    "read" => clauses.push("read = 1".to_string()),
                    "unread" => clauses.push("read = 0".to_string()),
                    _ => {
                        clauses.push("(from_addr LIKE ? OR to_addrs LIKE ? OR subject LIKE ?)".to_string());
                        values.push(format!("%{token}%"));
                        values.push(format!("%{token}%"));
                        values.push(format!("%{token}%"));
                    }
                },
                _ => {
                    clauses.push("(from_addr LIKE ? OR to_addrs LIKE ? OR subject LIKE ?)".to_string());
                    values.push(format!("%{token}%"));
                    values.push(format!("%{token}%"));
                    values.push(format!("%{token}%"));
                }
            }
        } else {
            clauses.push("(from_addr LIKE ? OR to_addrs LIKE ? OR subject LIKE ?)".to_string());
            values.push(format!("%{token}%"));
            values.push(format!("%{token}%"));
            values.push(format!("%{token}%"));
        }
    }

    if clauses.is_empty() {
        (String::new(), Vec::new())
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn parse_sms_search(search: Option<&str>) -> (String, Vec<String>) {
    let Some(search) = search else {
        return (String::new(), Vec::new());
    };
    let s = search.trim();
    if s.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut clauses = Vec::new();
    let mut values = Vec::new();

    for token in s.split_whitespace() {
        if let Some((key, val)) = token.split_once(':') {
            match key.to_lowercase().as_str() {
                "from" => {
                    clauses.push("from_phone LIKE ?".to_string());
                    values.push(format!("%{val}%"));
                }
                "to" => {
                    clauses.push("to_phone LIKE ?".to_string());
                    values.push(format!("%{val}%"));
                }
                "is" => match val.to_lowercase().as_str() {
                    "read" => clauses.push("read = 1".to_string()),
                    "unread" => clauses.push("read = 0".to_string()),
                    _ => {
                        clauses.push("(from_phone LIKE ? OR to_phone LIKE ? OR body LIKE ?)".to_string());
                        values.push(format!("%{token}%"));
                        values.push(format!("%{token}%"));
                        values.push(format!("%{token}%"));
                    }
                },
                _ => {
                    clauses.push("(from_phone LIKE ? OR to_phone LIKE ? OR body LIKE ?)".to_string());
                    values.push(format!("%{token}%"));
                    values.push(format!("%{token}%"));
                    values.push(format!("%{token}%"));
                }
            }
        } else {
            clauses.push("(from_phone LIKE ? OR to_phone LIKE ? OR body LIKE ?)".to_string());
            values.push(format!("%{token}%"));
            values.push(format!("%{token}%"));
            values.push(format!("%{token}%"));
        }
    }

    if clauses.is_empty() {
        (String::new(), Vec::new())
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}
