set dotenv-load := true

rustfmt := if shell('command', '-v', 'rustup') == '1' { 'cargo +nightly fmt' } else { 'cargo fmt' }

_default:
	@just --list

# Various integrity checks
check:
	# Running clippy...
	cargo clippy --workspace --all-features --no-deps -- -Dwarnings
	cargo clippy --workspace --tests --no-deps -- -Dwarnings

	# Running rustfmt...
	{{rustfmt}} --all --check

	# Running rustdoc...
	RUSTDOCFLAGS="-Dwarnings" cargo doc --workspace --all-features --document-private-items

	# Running sqlx...
	cargo sqlx prepare --workspace --check -- --tests

# Format the code
format:
	# Running rustfmt...
	{{rustfmt}} --all

# Spin up the database container
create-database:
	docker compose up --detach --wait cs2kz-database

# Remove the database container and clean volumes
clean-database:
	docker compose down --timeout=3 cs2kz-database
	sudo rm -rfv {{justfile_directory()}}/database/volumes/cs2kz

# Run database migrations
run-migrations:
	cargo sqlx migrate run --source {{justfile_directory()}}/database/migrations

prepare-query-cache:
	cargo sqlx prepare --workspace -- --tests
