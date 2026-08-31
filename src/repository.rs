extern crate dirs;

use chrono::NaiveDateTime;
use rusqlite::{Connection, Error, Result};

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
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl Task {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn id(&self) -> i32 {
        self.id
    }
}

pub struct NewTask {
    pub name: String,
    pub description: Option<String>,
}

impl Repository {
    pub fn init(&mut self) -> Result<()> {
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
        println!("last inserted id {}", id);

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
}
