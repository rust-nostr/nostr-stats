use std::collections::HashSet;

use nostr_sdk::prelude::*;
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};

async fn open_sqlite_pool(path: &str) -> Result<SqlitePool> {
    if !Sqlite::database_exists(path).await? {
        Sqlite::create_database(path).await?;
    }

    // Open SQLite
    let pool = SqlitePool::connect(path).await?;

    // Migrate
    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Open LMDB
    let database = NostrLMDB::open("./data")?;

    let pool = open_sqlite_pool("./stats.db").await?;

    // Query all relay lists
    let filter = Filter::new().kind(Kind::RelayList);
    let events = database.query(filter).await?;

    println!("Found {} events.", events.len());

    println!("Extracting and deduplicating relays...");

    // Deduplicate relays and remove local addresses
    let mut relays: HashSet<RelayUrl> = HashSet::new();
    for event in events {
        relays.extend(
            nip65::extract_owned_relay_list(event)
                .map(|(u, _)| u)
                .filter(|u| !u.is_local_addr()),
        );
    }

    println!("Extracted {} relays.", relays.len());

    println!("Saving into SQLite stats database...");

    // Insert relays
    for relay_url in relays {
        sqlx::query("INSERT OR IGNORE INTO relays (url) VALUES (?)")
            .bind(relay_url.as_str_without_trailing_slash())
            .execute(&pool)
            .await?;
    }

    println!("Done.");

    Ok(())
}
