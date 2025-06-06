# Nostr stats

**Experimental** relays checker and stats.

## Steps

The following steps are used both for the **initial setup** and for **updating** the data:

- Sync NIP65 relay lists locally: `make sync`
- Extract and deduplicate relays: `make extract`
- Check relays: `make check` (this may take some hours to complete)
- Read stats: `make stats`

The `make check` step can be stopped and restarted at a secondary time without losing progress,
since a record is kept of which relays were checked and at what timestamp.

## Policies

### Ignored relays

Are ignored all the following relays:
- local IP addresses
- `localhost` domain
- `filter.nostr.wine` domain
