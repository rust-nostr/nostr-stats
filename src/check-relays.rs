use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use nostr::prelude::*;
use nostr_sdk::prelude::*;
use sqlx::{FromRow, SqlitePool};
use tokio::sync::Semaphore;

const PROXY_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9050));
const DIRECT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const ONION_CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_CONCURRENT: usize = 50;

#[derive(FromRow)]
struct RelayToCheckRow {
    id: i64,
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Open SQLite
    let pool = SqlitePool::connect("./stats.db").await?;

    // Calculate the timestamp of 7 days ago
    let now = Timestamp::now();
    let seven_days_ago = now - Duration::from_secs(7 * 24 * 60 * 60);

    let relays: Vec<RelayToCheckRow> =
        sqlx::query_as("SELECT id, url FROM relays WHERE last_check IS NULL OR last_check < ?")
            .bind(seven_days_ago.as_u64() as i64)
            .fetch_all(&pool)
            .await?;

    println!("Found {} relays to check.", relays.len());

    // Check relays concurrently with limited concurrency
    check_relays(&pool, relays).await?;

    println!("Relay checking completed!");

    Ok(())
}

async fn check_relays(db: &SqlitePool, relays: Vec<RelayToCheckRow>) -> Result<()> {
    let opts = Options::new().connection(
        Connection::new()
            .proxy(PROXY_ADDR)
            .target(ConnectionTarget::Onion),
    );
    let keys = Keys::generate(); // Used for AUTH
    let client = Client::builder().signer(keys).opts(opts).build();

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let db = Arc::new(db.clone());

    let mut handles = Vec::with_capacity(relays.len());

    for (index, relay_row) in relays.into_iter().enumerate() {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let db = db.clone();

        // Spawn a task
        let handle = tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = semaphore.acquire().await.unwrap();

            let url: String = relay_row.url;

            println!("[{}/total] Checking relay: {}", index + 1, url);

            let now = Timestamp::now();

            match check_relay_with_nostr(client, &url).await {
                Ok(relay_info) => {
                    sqlx::query(
                        "UPDATE relays SET last_check = ?, nip11 = ?, negentropy = ? WHERE id = ?",
                    )
                    .bind(now.as_u64() as i64)
                    .bind(relay_info.nip11)
                    .bind(relay_info.supports_negentropy)
                    .bind(relay_row.id)
                    .execute(&*db)
                    .await
                    .unwrap();

                    println!("✓ Successfully checked relay: {}", url);
                }
                Err(e) => {
                    sqlx::query("UPDATE relays SET last_check = ? WHERE id = ?")
                        .bind(now.as_u64() as i64)
                        .bind(relay_row.id)
                        .execute(&*db)
                        .await
                        .unwrap();

                    println!("✗ Failed to check relay {}: {}", url, e);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        if let Err(e) = handle.await {
            eprintln!("Task failed: {}", e);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct RelayInfo {
    nip11: Option<String>,
    supports_negentropy: Option<bool>,
}

async fn check_relay_with_nostr(
    client: Client,
    url: &str,
) -> Result<RelayInfo, Box<dyn std::error::Error + Send + Sync>> {
    // Parse relay URL
    let url = RelayUrl::parse(url)?;

    client.add_relay(&url).await?;

    let timeout = if url.is_onion() {
        ONION_CONNECTION_TIMEOUT
    } else {
        DIRECT_CONNECTION_TIMEOUT
    };

    client.try_connect_relay(&url, timeout).await?;

    let relay = client.relay(&url).await?;

    // Get NIP11 document
    let proxy = match relay.connection_mode() {
        ConnectionMode::Direct => None,
        ConnectionMode::Proxy(proxy) => Some(*proxy),
    };
    let document = RelayInformationDocument::get(url.clone().into(), proxy, timeout)
        .await
        .ok();

    let filter = Filter::new().kind(Kind::Metadata).limit(0);
    let supports_negentropy: bool = relay.sync(filter, &SyncOptions::new()).await.is_ok();

    let relay_info = RelayInfo {
        nip11: document.map(|d| serde_json::to_string(&d).unwrap()),
        supports_negentropy: Some(supports_negentropy),
    };

    client.remove_relay(url).await?;

    Ok(relay_info)
}
