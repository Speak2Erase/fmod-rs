// Copyright (c) 2024 Lily Lyons
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use fmod_sys::*;

mod general;
mod loading;
mod lookups; // general lookups that are too small to be their own module

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(transparent)] // so we can transmute between types
pub struct Bank {
    pub(crate) inner: *mut FMOD_STUDIO_BANK,
}

unsafe impl Send for Bank {}
unsafe impl Sync for Bank {}

impl From<*mut FMOD_STUDIO_BANK> for Bank {
    fn from(value: *mut FMOD_STUDIO_BANK) -> Self {
        Self { inner: value }
    }
}

impl From<Bank> for *mut FMOD_STUDIO_BANK {
    fn from(value: Bank) -> Self {
        value.inner
    }
}
