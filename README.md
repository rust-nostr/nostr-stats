# Nostr stats

**Experimental** relays checker and stats.

## Steps

- Sync NIP65 relay lists locally: `make sync`
- Extract and deduplicate relays: `make extract`
- Check relays: `make check` (this may take some hours to complete)
- Read stats: `make stats`
