# Contribution Guidelines

1. If you are unsure whether a change is desired, open an issue and ask first;
   nobody wants to waste time working on something that won't get merged anyway!
2. Make sure your local environment is setup correctly as explained in
   [Local development setup](#local-development-setup).
3. Rust has [great tooling](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html).
   Use it! `cargo clippy` and `cargo +nightly fmt` will be your best friends.

# Local development setup

First make sure you have the following tools / programs installed on your
computer:

1. The Rust toolchain using [rustup](https://www.rust-lang.org/tools/install)
2. [Docker](https://www.docker.com/) for running the local database (and API if
   you want)
3. [just](https://github.com/casey/just) if you want a nice command runner

In order for the API to work properly, copy `.env.example` to `.env` (which is
`.gitignore`'d) and change any values you want to change. If you plan on running
the API inside docker, also copy `.env.docker.example` to `.env.docker`.

To run the API, simply run `cargo run`, and everything should work!

Before committing, you should run `just precommit` to make sure your code
   * compiles correctly
   * doesn't violate any linter rules
   * is formatted correctly
   * is documented properly
