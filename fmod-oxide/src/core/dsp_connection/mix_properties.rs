// Copyright (c) 2024 Lily Lyons
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use fmod_sys::*;
use std::ffi::c_float;

use crate::DspConnection;

impl DspConnection {
    /// Sets the connection's volume scale.
    pub fn set_mix(&self, volume: c_float) -> Result<()> {
        unsafe { FMOD_DSPConnection_SetMix(self.inner, volume).to_result() }
    }

    /// Retrieves the connection's volume scale.
    pub fn get_mix(&self) -> Result<c_float> {
        let mut volume = 0.0;
        unsafe { FMOD_DSPConnection_GetMix(self.inner, &mut volume).to_result()? };
        Ok(volume)
    }

    // TODO mix matrix
}
