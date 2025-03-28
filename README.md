[![Dependency Status](https://deps.rs/repo/github/kzglobalteam/cs2kz-api/status.svg)](https://deps.rs/repo/github/kzglobalteam/cs2kz-api)

# CS2KZ API

## Local Setup

This project encapsulates the backend infrastructure of CS2KZ.
It is developed in tandem with [the plugin][cs2kz] and currently WIP.

If you want to run the API locally, see [Local Setup](#local-setup).
The recommended tooling for development is listed under [Tooling](#tooling).

Questions and feedback are appreciated! Feel free to open an issue or join [our Discord][discord].

## Local Setup

> \[!IMPORTANT\]
> It is expected you have the required tools described in [tooling](#tooling)
> installed on your system.

The API uses a configuration file called `cs2kz-api.toml`.

An example configuration file is provided with all the default values filled in,
copy and modify it as you see fit. `.example.env` and `.docker.example.env`
should be copied to `.env` and `.docker.env` respectively. Again, change the
default values as you see fit.

The API requires a MariaDB instance in order to run. It is recommended that you
run one using [Docker][] using the `compose.yaml` file provided by this
repository.

Install docker and run the following command:

```sh
docker compose up -d database
```

To compile the API itself, you can use `cargo`:

```sh
# also specify `--release` to enable optimizations
cargo build --locked --package=cs2kz-api --bin=cs2kz-api

# compile & run in one step
cargo run --locked --package=cs2kz-api --bin=cs2kz-api
```

To compile and run with Docker instead:

```sh
docker compose up --build api
```

The nix flake in the repository root also outputs the API binary as its default
package.

### Debugging with [tokio-console][]

The API supports sending trace data to `tokio-console` so you can inspect the
runtime in real time. In order to use it, set `tracing.console.enable` to `true`
in your configuration file.

## Tooling

1. [rustup][] to install the Rust toolchain
2. [Docker][] for running the database and (optionally) the API itself
3. [sqlx-cli][] for managing database migrations
4. [DepotDownloader][] (optional)
5. [just][] (optional) as a command runner
6. [nix][] (optional) if you know you know

[cs2kz]: https://github.com/KZGlobalTeam/cs2kz-metamod
[discord]: https://www.discord.gg/csgokz
[Docker]: https://www.docker.com
[tokio-console]: https://crates.io/crates/tokio-console
[rustup]: https://rustup.rs
[sqlx-cli]: https://github.com/launchbadge/sqlx/tree/main/sqlx-cli
[DepotDownloader]: https://github.com/SteamRE/DepotDownloader
[just]: https://just.systems
[nix]: https://nixos.org

## Licensing

This project is licensed under the
[GPL-3.0](https://www.gnu.org/licenses/gpl.html).
See [LICENSE.md](./LICENSE.md) for more information.
