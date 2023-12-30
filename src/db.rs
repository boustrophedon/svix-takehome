use rusqlite::*;

use crate::Task;

use std::path::Path;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy)]
pub struct DbId(pub i64);

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Db {
        let conn = Connection::open(path).unwrap();
        conn.pragma_update(None, "foreign_keys", "on").unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        Db {
            conn,
        }
    }

    /// events table layout:
    /// status -> 0 pending, 1 complete
    pub fn create_tables(&self) {
        let conn = &self.conn;
        conn.execute(
            "CREATE TABLE events (
                id INTEGER PRIMARY KEY,
                type TEXT NOT NULL,
                status INTEGER NOT NULL,
                timestamp INTEGER NOT NULL) STRICT;", []
        ).unwrap();
        // index first on status, then filter by timestamp
        conn.execute(
            "CREATE INDEX event_timestamp ON events(status, timestamp);", []
        ).unwrap();
    }

    /// insert a new task
    pub fn insert_task(&self, task: Task, time: DateTime<Utc>) {
        let conn = &self.conn;
        let t = time.timestamp();
        conn.execute(
            "INSERT INTO events (type, status, timestamp) VALUES (?, 0, ?);", [task.to_str(), &t.to_string()]).unwrap();
    }

    /// Mark task as complete
    pub fn complete_task(&self, task_id: DbId) {
        let conn = &self.conn;
        conn.execute(
            "UPDATE events SET status = 1 WHERE id = ?;", [task_id.0,]).unwrap();
    }

    /// Returns all pending tasks due prior to the given time
    pub fn fetch_pending_tasks_due_by(&self, time: DateTime<Utc>) -> Vec<(DbId, Task)> {
        let conn = &self.conn;

        let t = time.timestamp();

        let mut stmt = conn.prepare("SELECT id, type FROM events WHERE status = 0 AND TIMESTAMP < ?").unwrap();
        let rows = stmt.query_map([t], |row| Ok((row.get(0), row.get(1)))).unwrap();

        let mut output = Vec::new();
        for row in rows {
            let row = row.unwrap();
            let id = row.0.unwrap();
            let task_type: String = row.1.unwrap();
            output.push((DbId(id), Task::from_str(&task_type)));
        }

        output
    }

    /// currently only used in tests
    pub fn fetch_all_tasks(&self) -> Vec<(DbId, Task, u8, DateTime<Utc>)> {
        let conn = &self.conn;

        let mut stmt = conn.prepare("SELECT id, type, status, timestamp FROM events").unwrap();
        let rows = stmt.query_map([], |row| Ok((row.get(0), row.get(1), row.get(2), row.get(3)))).unwrap();

        let mut output = Vec::new();
        for row in rows {
            let row = row.unwrap();
            let id = row.0.unwrap();
            let task_type: String = row.1.unwrap();
            let status: u8 = row.2.unwrap();
            let t = row.3.unwrap();
            let timestamp: DateTime<Utc> = DateTime::from_timestamp(t, 0).unwrap();
            output.push((
                    DbId(id),
                    Task::from_str(&task_type),
                    status,
                    timestamp));
        }

        output
    }
}

// TODO
// #[cfg(test)]
// mod test {}
