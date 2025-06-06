use std::collections::HashMap;

use nostr_sdk::prelude::*;
use sqlx::{FromRow, SqlitePool};

const TOP_IMPL_LIMIT: usize = 20;

#[derive(FromRow)]
struct QueryRelayNip11 {
    nip11: Option<String>,
}

#[derive(Debug)]
struct RelayStats {
    total_relays: u64,
    checked_relays: u64,
    reachable_relays: u64,
    checked_percentage: f64,
    reachability_percentage: f64,
    negentropy_supported: u64,
    negentropy_percentage: f64,
    implementations: HashMap<String, u64>,
}

async fn query_relays(pool: &SqlitePool) -> Result<RelayStats> {
    // Count total relays
    let total_relays: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM relays")
        .fetch_one(pool)
        .await?;

    // Count relays that have been checked (last_check is not NULL)
    let checked_relays: u64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM relays WHERE last_check IS NOT NULL")
            .fetch_one(pool)
            .await?;

    // Count reachable relays
    let reachable_relays: u64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM relays WHERE reachable = TRUE")
            .fetch_one(pool)
            .await?;

    // Count relays that support negentropy
    let negentropy_supported: u64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM relays WHERE negentropy = TRUE")
            .fetch_one(pool)
            .await?;

    // Checked percentages
    let checked_percentage: f64 = if total_relays > 0 {
        (checked_relays as f64 / total_relays as f64) * 100.0
    } else {
        0.0
    };

    // Calculate percentages
    let reachability_percentage: f64 = if checked_relays > 0 {
        (reachable_relays as f64 / checked_relays as f64) * 100.0
    } else {
        0.0
    };

    let negentropy_percentage: f64 = if reachable_relays > 0 {
        (negentropy_supported as f64 / reachable_relays as f64) * 100.0
    } else {
        0.0
    };

    // Get implementation stats from NIP11 data
    let mut implementations: HashMap<String, u64> = HashMap::new();

    let nip11_rows: Vec<QueryRelayNip11> =
        sqlx::query_as("SELECT nip11 FROM relays WHERE nip11 IS NOT NULL AND reachable = TRUE")
            .fetch_all(pool)
            .await?;

    for nip11_json in nip11_rows.into_iter().filter_map(|v| v.nip11) {
        if let Ok(nip11_data) = serde_json::from_str::<RelayInformationDocument>(&nip11_json) {
            let software = nip11_data
                .software
                .unwrap_or_else(|| String::from("unknown"));
            *implementations.entry(software).or_insert(0) += 1;
        }
    }

    Ok(RelayStats {
        total_relays,
        checked_relays,
        reachable_relays,
        checked_percentage,
        reachability_percentage,
        negentropy_supported,
        negentropy_percentage,
        implementations,
    })
}

fn print_stats(stats: RelayStats) {
    println!("=== Relay Statistics ===");
    println!("Total known relays: {}", stats.total_relays);
    println!(
        "Checked relays: {}/{} ({:.1}%)",
        stats.checked_relays, stats.total_relays, stats.checked_percentage
    );
    println!(
        "Reachable relays: {}/{} ({:.1}%)",
        stats.reachable_relays, stats.checked_relays, stats.reachability_percentage
    );
    println!();

    println!("=== Negentropy Support (NIP77) ===");
    println!(
        "Total relays: {}/{} ({:.1}%)",
        stats.negentropy_supported, stats.reachable_relays, stats.negentropy_percentage
    );
    println!();

    println!("=== Top {TOP_IMPL_LIMIT} implementations ===");
    if stats.implementations.is_empty() {
        println!("No implementation data available");
    } else {
        // Count total implementations
        let total: u64 = stats.implementations.values().sum();

        // Convert to a vector
        let mut list: Vec<(String, u64)> = stats.implementations.into_iter().collect();

        // Sort by count, descending
        list.sort_by(|a, b| b.1.cmp(&a.1));

        for (index, (implementation, count)) in list.iter().take(TOP_IMPL_LIMIT).enumerate() {
            let percentage: f64 = (*count as f64 / total as f64) * 100.0;
            println!(
                "{}. {implementation}: {count} ({percentage:.1}%)",
                index + 1
            );
        }

        if list.len() > TOP_IMPL_LIMIT {
            println!(
                "... and {} more implementations",
                list.len() - TOP_IMPL_LIMIT
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Open SQLite
    let pool = SqlitePool::connect("./stats.db").await?;

    // Generate and display stats
    match query_relays(&pool).await {
        Ok(stats) => print_stats(stats),
        Err(e) => eprintln!("Error generating stats: {}", e),
    }

    Ok(())
}
