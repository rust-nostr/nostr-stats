# Nostr stats

**Experimental** relays checker and stats.

## Steps

- Sync NIP65 relay lists locally: `cargo run --release --bin sync-lists`
- Extract and deduplicate relays: `cargo run --release --bin extract-relays`
- Check relays: `cargo run --release --bin check-relays`
- Read stats: `cargo run --release --bin read-stats`
