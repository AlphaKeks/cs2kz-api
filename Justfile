default:
	@just --list

create-database:
	docker compose down database
	docker compose up -d database

migrate *ARGS:
	cargo sqlx migrate {{ARGS}} \
		--source {{justfile_directory()}}/migrations/

prepare-sql-cache:
	cargo sqlx prepare --workspace -- --all-targets --all-features
