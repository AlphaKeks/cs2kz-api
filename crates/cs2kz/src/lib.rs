// This crate is part of the cs2kz-api project.
//
// Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see https://www.gnu.org/licenses.

#[macro_use]
extern crate thiserror;

mod either;

pub mod steam_id;

#[doc(inline)]
pub use steam_id::SteamID;

pub mod mode;

#[doc(inline)]
pub use mode::Mode;

pub mod tier;

#[doc(inline)]
pub use tier::Tier;

pub mod jump_type;

#[doc(inline)]
pub use jump_type::JumpType;

pub mod styles;

#[doc(inline)]
pub use styles::{Style, Styles};

pub mod map_approval_status;

#[doc(inline)]
pub use map_approval_status::MapApprovalStatus;

pub mod ranked_status;

#[doc(inline)]
pub use ranked_status::RankedStatus;
