use sqlx::SqlitePool;

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS issues (
            issue_key    TEXT PRIMARY KEY NOT NULL,
            summary      TEXT NOT NULL,
            project_key  TEXT NOT NULL,
            status       TEXT,
            issue_type   TEXT,
            updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS worklogs (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            issue_key        TEXT NOT NULL,
            started_at       TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL,
            description      TEXT NOT NULL DEFAULT '',
            sync_status      TEXT NOT NULL DEFAULT 'pending',
            jira_worklog_id  TEXT,
            sync_error       TEXT,
            created_at       TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS active_timer (
            id               INTEGER PRIMARY KEY CHECK (id = 1),
            issue_key        TEXT NOT NULL,
            started_at       TEXT NOT NULL,
            accumulated_secs INTEGER NOT NULL DEFAULT 0,
            is_paused        INTEGER NOT NULL DEFAULT 0,
            paused_at        TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes separately (SQLite doesn't support multiple statements well with CREATE INDEX)
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_worklogs_sync_status ON worklogs(sync_status)",
        "CREATE INDEX IF NOT EXISTS idx_worklogs_issue_key ON worklogs(issue_key)",
        "CREATE INDEX IF NOT EXISTS idx_worklogs_started_at ON worklogs(started_at DESC)",
    ];

    for idx in indexes {
        sqlx::query(idx).execute(pool).await?;
    }

    // Add jira_updated_at column (migration)
    let _ = sqlx::query("ALTER TABLE worklogs ADD COLUMN jira_updated_at TEXT")
        .execute(pool)
        .await;

    Ok(())
}
