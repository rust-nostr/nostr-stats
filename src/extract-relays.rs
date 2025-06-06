use std::collections::HashSet;

use nostr_sdk::prelude::*;
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};

const IGNORED_RELAY_DOMAINS: &[&str] = &["localhost", "filter.nostr.wine"];

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
                // Filter local addr relays
                .filter(|u| !u.is_local_addr())
                // Filter ignored domains
                .filter(|u| match u.domain() {
                    // Skip the ignored relays
                    Some(domain) => !IGNORED_RELAY_DOMAINS.contains(&domain),
                    None => true,
                }),
        );
    }

    println!("Extracted {} relays.", relays.len());

    // Delete ignored relays
    for relay_url in IGNORED_RELAY_DOMAINS {
        let res = sqlx::query("DELETE FROM relays WHERE url LIKE ?")
            .bind(format!("%://{}%", relay_url))
            .execute(&pool)
            .await?;

        if res.rows_affected() > 0 {
            println!(
                "Deleted {} rows that match the following ignored domain: {relay_url}",
                res.rows_affected()
            );
        }
    }

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
