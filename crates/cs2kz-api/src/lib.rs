/* CS2KZ API
 *
 * Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see https://www.gnu.org/licenses.
 */

//! The core logic of the CS2KZ API.
//!
//! This crate contains types and functions modeling the core logic of the API.
//! Other crates in the workspace, such as `cs2kz-api-server` depend on this
//! crate and act as "frontends" for it. You can think of this crate as the
//! "service layer" of the whole stack.
