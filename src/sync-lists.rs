use nostr_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database = NostrLMDB::open("./data")?;
    let client = Client::builder().database(database).build();

    client.add_relay("wss://purplepag.es").await?;
    client.add_relay("wss://relay.damus.io").await?;
    client.add_relay("wss://nostr.wine").await?;
    client.add_relay("wss://relay.primal.net").await?;

    client.connect().await;

    let filter = Filter::new().kind(Kind::RelayList);
    let opts = SyncOptions::default().direction(SyncDirection::Down);
    let output = client.sync(filter, &opts).await?;

    println!("Local: {}", output.local.len());
    println!("Remote: {}", output.remote.len());
    println!("Received: {}", output.received.len());

    Ok(())
}
