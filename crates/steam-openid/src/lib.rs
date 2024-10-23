//! # Steam OpenID authentication
//!
//! [Steam] can act as an [OpenID 2.0] provider.
//! This crate provides types and functions to perform that authentication flow.
//!
//! ## Usage
//!
//! First, create a [`LoginForm`]. You can do this with the [`LoginForm::new()`]
//! constructor. It requires you to pass a `realm`; this is the public URL of
//! your service. The `callback_route` parameter will be appended to the
//! `realm`, so it should be a relative URI. It represents the endpoint Steam
//! will redirect the user to after they completed the login process.
//!
//! ```
//! use steam_openid::LoginForm;
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let realm = Url::parse("https://api.cs2kz.org")?;
//! let form = LoginForm::new(realm, "/auth/callback");
//! # Ok(())
//! # }
//! ```
//!
//! Next, generate a redirection URL using the [`LoginForm::redirect_url()`]
//! method. Redirect your user to that URL. They will be able to login with
//! Steam as usual, and will then be redirected back to the endpoint you
//! configured earlier. This HTTP request will include query parameters that you
//! can parse into a [`CallbackPayload`]. This payload must now be verified to
//! make sure it actually came from Steam. To do this, call the
//! [`CallbackPayload::verify()`] method. If an error is returned, that means
//! the request was fake (or there is a bug in this library!).
//!
//! After you made sure the request is legit, you can extract the user's SteamID
//! using the [`CallbackPayload::user_id()`] method, and access your custom
//! [`userdata`] field.
//!
//! [Steam]: https://store.steampowered.com
//! [OpenID 2.0]: https://openid.net/specs/openid-authentication-2_0.html
//! [`userdata`]: CallbackPayload::userdata

/* This crate is part of the cs2kz-api project.
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

#[macro_use(Error)]
extern crate thiserror;

/// Steam URL to redirect the user in for login.
pub const LOGIN_URL: &str = "https://steamcommunity.com/openid/login";

mod login_form;
pub use login_form::{CreateRedirectUrlError, LoginForm};

mod callback;
pub use callback::{CallbackPayload, VerifyCallbackPayloadError};
