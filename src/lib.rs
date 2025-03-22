#![feature(array_try_from_fn)]
#![feature(debug_closure_helpers)]
#![feature(decl_macro)]
#![feature(extend_one)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(try_blocks)]
#![feature(unqualified_local_imports)]

#[macro_use(Debug, Display, From, Into, Error)]
extern crate derive_more as _;

#[macro_use(builder, Builder)]
extern crate bon as _;

#[macro_use(pin_project)]
extern crate pin_project as _;

#[macro_use(select)]
extern crate tokio as _;

pub mod access_key;
pub mod checksum;
pub mod database;
pub mod discord;
pub mod email;
pub mod error;
pub mod event_queue;
pub mod game;
pub mod mode;
pub mod points;
pub mod python;
pub mod serde;
pub mod server_monitor;
pub mod steam;
pub mod stream;
pub mod time;

pub mod bans;
pub mod maps;
pub mod players;
pub mod records;
pub mod servers;
pub mod users;
