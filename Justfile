_default:
	@just --list

prepare-query-cache:
	cargo sqlx prepare --workspace -- --tests
