// Copyright (c) 2024 Lily Lyons
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use fmod_sys::*;
use std::ffi::c_uint;

use crate::{Dsp, DspType, System};

impl Dsp {
    // TODO show dialogue config

    /// Reset a DSPs internal state ready for new input signal.
    ///
    /// This will clear all internal state derived from input signal while retaining any set parameter values.
    /// The intended use of the function is to avoid audible artifacts if moving the [`Dsp`] from one part of the [`Dsp`] network to another.
    pub fn reset(&self) -> Result<()> {
        unsafe { FMOD_DSP_Reset(self.inner).to_result() }
    }

    /// Frees a [`Dsp`] object.
    ///
    /// If [`Dsp`] is not removed from the network with ChannelControl::removeDSP after being added with ChannelControl::addDSP,
    /// it will not release and will instead return [`FMOD_RESULT::FMOD_ERR_DSP_INUSE`].
    pub fn release(self) -> Result<()> {
        unsafe { FMOD_DSP_Release(self.inner).to_result() }
    }

    /// Retrieves the pre-defined type of a FMOD registered [`Dsp`] unit.
    pub fn get_type(&self) -> Result<DspType> {
        let mut dsp_type = 0;
        unsafe { FMOD_DSP_GetType(self.inner, &mut dsp_type).to_result()? };
        let dsp_type = dsp_type.try_into()?;
        Ok(dsp_type)
    }

    // TODO getinfo

    /// Retrieves statistics on the mixer thread CPU usage for this unit.
    ///
    /// [`crate::InitFlags::PROFILE_ENABLE`] with [`crate::SystemBuilder::new`] is required to call this function.
    pub fn get_cpu_usage(&self) -> Result<(c_uint, c_uint)> {
        let mut exclusive = 0;
        let mut inclusive = 0;
        unsafe {
            FMOD_DSP_GetCPUUsage(self.inner, &mut exclusive, &mut inclusive).to_result()?;
        }
        Ok((exclusive, inclusive))
    }

    // TODO userdata

    // TODO callback

    /// Retrieves the parent System object.
    pub fn get_system(&self) -> Result<System> {
        let mut system = std::ptr::null_mut();
        unsafe { FMOD_DSP_GetSystemObject(self.inner, &mut system).to_result()? };
        Ok(system.into())
    }
}
