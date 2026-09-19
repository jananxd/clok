extern crate dirs;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Error, Result, params, params_from_iter};

#[derive(Debug)]
pub struct Repository {
    conn: Connection,
}

impl Default for Repository {
    fn default() -> Self {
        let path = dirs::data_local_dir().expect("data local directory is required");
        let clok_dir = path.join("clok");
        std::fs::create_dir_all(&clok_dir).expect("failed to create data directory");
        let conn = Connection::open(clok_dir.join("clok.db")).expect("failed to open database");
        Repository { conn }
    }
}

#[derive(Debug)]
pub struct Task {
    id: i32,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Task {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn id(&self) -> i32 {
        self.id
    }
}

#[derive(Debug)]
pub struct TimeEntry {
    pub id: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub task_id: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TimeEntry {
    pub fn id(&self) -> i32 {
        self.id
    }
}

pub struct NewTask {
    pub name: String,
    pub description: Option<String>,
}

pub struct StartTimerPayload {
    pub start_time: DateTime<Utc>,
    pub task_id: i32,
}

impl Repository {
    pub fn init(&mut self) -> Result<()> {
        self.conn.execute("PRAGMA foreign_keys = ON", ())?;
        let version: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;

        if version < 1 {
            self.conn.execute(
                "
                CREATE TABLE IF NOT EXISTS task (
                    id    INTEGER PRIMARY KEY,
                    name  TEXT NOT NULL,
                    description  VARCHAR(255),
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )
                ",
                (),
            )?;
            self.conn.execute(
                "CREATE TRIGGER IF NOT EXISTS task_updated_at
                AFTER UPDATE ON task
                BEGIN
                    UPDATE task SET updated_at = datetime('now') WHERE id = NEW.id;
                END",
                (),
            )?;
            self.conn.execute("PRAGMA user_version = 1", ())?;
        };

        if version < 2 {
            self.conn.execute(
                "
                CREATE TABLE IF NOT EXISTS time_entry (
                    id    INTEGER PRIMARY KEY,
                    start_time DATETIME NOT NULL,
                    end_time DATETIME,
                    task_id INTEGER REFERENCES task(id),
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )
                ",
                (),
            )?;
            self.conn.execute(
                "CREATE TRIGGER IF NOT EXISTS task_updated_at
                AFTER UPDATE ON task
                BEGIN
                    UPDATE task SET updated_at = datetime('now') WHERE id = NEW.id;
                END",
                (),
            )?;
            self.conn.execute("PRAGMA user_version = 2", ())?;
        };

        Ok(())
    }

    pub fn get_tasks(&self) -> Result<Vec<Task>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description, created_at, updated_at FROM task")?;

        let task_iter = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in task_iter {
            tasks.push(task?);
        }

        Ok(tasks)
    }

    pub fn delete_task(&self, task_id: &i32) -> Result<()> {
        let deleted_rows = self
            .conn
            .execute("DELETE FROM task WHERE id = ?1", [task_id])?;

        if deleted_rows == 0 {
            Err(Error::QueryReturnedNoRows)
        } else {
            Ok(())
        }
    }

    pub fn add_task(&self, new_task: &NewTask) -> Result<Task> {
        self.conn.execute(
            "INSERT INTO task (name, description) VALUES (?1, ?2)",
            (&new_task.name, &new_task.description),
        )?;

        let id = self.conn.last_insert_rowid();

        // TODO: probably check whether we need to create
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, created_at, updated_at FROM task WHERE id = :id",
        )?;

        let task_iter = stmt.query_map(&[(":id", &id)], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        let mut test = task_iter.into_iter();
        let first_value = test.next();

        match first_value {
            Some(task) => Ok(task?),
            None => Err(Error::QueryReturnedNoRows),
        }
    }

    pub fn start_timer(&self, payload: &StartTimerPayload) -> Result<TimeEntry> {
        self.conn.execute(
            "INSERT INTO time_entry (start_time, task_id) VALUES (?1, ?2)",
            (&payload.start_time, &payload.task_id),
        )?;

        let id = self.conn.last_insert_rowid();

        let mut stmt = self.conn.prepare(
            "SELECT id, start_time, end_time, task_id, created_at, updated_at FROM time_entry WHERE id = :id",
        )?;

        let time_entry = stmt.query_map(&[(":id", &id)], |row| {
            Ok(TimeEntry {
                id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
                task_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        let first_value = time_entry.into_iter().next();

        match first_value {
            Some(time_entry) => Ok(time_entry?),
            None => Err(Error::QueryReturnedNoRows),
        }
    }

    pub fn stop_timer(&self, time_entry_id: i32) -> Result<()> {
        let end_time = Utc::now();
        self.conn.execute(
            "UPDATE time_entry SET end_time = ?1 WHERE id = ?2",
            params![end_time, time_entry_id],
        )?;
        Ok(())
    }

    pub fn get_active_timers(&self, task_ids: Vec<i32>) -> Result<Vec<TimeEntry>> {
        let placeholders: Vec<String> = task_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();

        let sql = format!(
            "SELECT id, start_time, end_time, task_id, created_at, updated_at
                   FROM time_entry WHERE task_id IN ({}) AND end_time IS NULL",
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&sql.to_string())?;

        let time_entries = stmt.query_map(params_from_iter(task_ids.iter()), |row| {
            Ok(TimeEntry {
                id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
                task_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let mut time_entry_list = Vec::new();
        for time_entry in time_entries {
            time_entry_list.push(time_entry?);
        }

        Ok(time_entry_list)
    }
}
