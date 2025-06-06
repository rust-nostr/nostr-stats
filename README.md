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

## Example output

```bash
=== Relay Statistics ===
Total known relays: 5735
Progress: 5735/5735 (100.0%)
Reachable relays: 1341/5735 (23.4%)

=== Negentropy Support (NIP77) ===
Total relays: 219/1341 (16.3%)

=== Top 20 implementations ===
1. https://git.sr.ht/~gheartsfield/nostr-rs-relay: 426 (34.8%)
2. git+https://github.com/hoytech/strfry.git: 283 (23.1%)
3. https://github.com/bitvora/haven: 137 (11.2%)
4. https://relay.nostr.band: 52 (4.2%)
5. git+https://github.com/Cameri/nostream.git: 49 (4.0%)
6. https://github.com/bitvora/wot-relay: 35 (2.9%)
7. https://github.com/fiatjaf/khatru: 34 (2.8%)
8. git+https://github.com/cameri/nostream.git: 34 (2.8%)
9. chorus: 12 (1.0%)
10. https://github.com/bitvora/sw2: 9 (0.7%)
11. unknown: 9 (0.7%)
12. https://github.com/rnostr/rnostr: 7 (0.6%)
13. https://github.com/CodyTseng/nostr-relay-tray: 7 (0.6%)
14. NFDB: 6 (0.5%)
15. https://github.com/github-tijlxyz/khatru-pyramid: 6 (0.5%)
16. https://github.com/Spl0itable/nosflare: 6 (0.5%)
17. https://github.com/haorendashu/cfrelay: 6 (0.5%)
18. https://gitlab.com/soapbox-pub/ditto: 6 (0.5%)
19. https://github.com/quentintaranpino/nostrcheck-server: 5 (0.4%)
20. LNbits: 5 (0.4%)
... and 60 more implementations
```
