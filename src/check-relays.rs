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

    let total: usize = relays.len();
    let mut handles = Vec::with_capacity(total);

    for (index, relay_row) in relays.into_iter().enumerate() {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let db = db.clone();

        // Spawn a task
        let handle = tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = semaphore.acquire().await.unwrap();

            let url: String = relay_row.url;

            println!("[{}/{total}] Checking relay: {url}", index + 1);

            match check_relay(db, client, relay_row.id, &url).await {
                Ok(_) => println!("✓ Successfully checked relay: {url}"),
                Err(e) => println!("✗ Failed to check relay {url}: {e}"),
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

fn get_timeout_for_relay_url(url: &RelayUrl) -> Duration {
    if url.is_onion() {
        ONION_CONNECTION_TIMEOUT
    } else {
        DIRECT_CONNECTION_TIMEOUT
    }
}

async fn check_relay(
    db: Arc<SqlitePool>,
    client: Client,
    id: i64,
    url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = RelayUrl::parse(url)?;

    // Initial check
    match initial_relay_check(&client, &url).await {
        Ok(relay) => {
            // Set the relay to reachable
            update_reachable_status_and_last_check(&db, id, true).await?;

            // Check and update NIP11
            check_and_update_nip11(&db, &relay, id).await?;

            // Check and update negentropy
            check_and_update_negentropy(&db, &relay, id).await?;
        }
        Err(e) => {
            // Set the relay to unreachable
            update_reachable_status_and_last_check(&db, id, false).await?;

            // Propagate error
            return Err(e);
        }
    };

    // Remove the relay from the pool
    client.remove_relay(url).await?;

    Ok(())
}

// Add relay and check if can connect
async fn initial_relay_check(
    client: &Client,
    url: &RelayUrl,
) -> Result<Relay, Box<dyn std::error::Error + Send + Sync>> {
    client.add_relay(url).await?;

    let timeout: Duration = get_timeout_for_relay_url(url);

    client.try_connect_relay(url, timeout).await?;

    Ok(client.relay(url).await?)
}

/// Get the NIP11 document and update the database
async fn check_and_update_nip11(
    db: &SqlitePool,
    relay: &Relay,
    id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let proxy = match relay.connection_mode() {
        ConnectionMode::Direct => None,
        ConnectionMode::Proxy(proxy) => Some(*proxy),
    };
    let timeout: Duration = get_timeout_for_relay_url(relay.url());
    let opts = Nip11GetOptions { proxy, timeout };

    // Try to get the NIP11 document
    let document = RelayInformationDocument::get(relay.url().clone().into(), opts)
        .await
        .ok();

    // Save into the db
    sqlx::query("UPDATE relays SET nip11 = ? WHERE id = ?")
        .bind(document.map(|d| serde_json::to_string(&d).unwrap()))
        .bind(id)
        .execute(db)
        .await?;

    Ok(())
}

async fn check_and_update_negentropy(
    db: &SqlitePool,
    relay: &Relay,
    id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = Filter::new().kind(Kind::Metadata).limit(0);

    // Try to perform a sync
    let supports_negentropy: bool = relay.sync(filter, &SyncOptions::new()).await.is_ok();

    // Save into the db
    sqlx::query("UPDATE relays SET negentropy = ? WHERE id = ?")
        .bind(supports_negentropy)
        .bind(id)
        .execute(db)
        .await?;

    Ok(())
}

async fn update_reachable_status_and_last_check(
    db: &SqlitePool,
    id: i64,
    reachable: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now: Timestamp = Timestamp::now();

    sqlx::query("UPDATE relays SET last_check = ?, reachable = ? WHERE id = ?")
        .bind(now.as_u64() as i64)
        .bind(reachable)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
