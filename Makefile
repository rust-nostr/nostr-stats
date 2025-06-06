# Help target
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  sync			- Sync NIP65 relay lists locally"
	@echo "  extract		- Extract and deduplicate relays"
	@echo "  check			- Check relays (may take hours)"
	@echo "  stats			- Generate and print the stats"
	@echo "  help			- Show this help message"

.PHONY: sync
sync:
	cargo run --release --bin sync-lists

.PHONY: extract
extract:
	cargo run --release --bin extract-relays

.PHONY: check
check:
	cargo run --release --bin check-relays

.PHONY: stats
stats:
	cargo run --release --bin read-stats
