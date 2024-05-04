// Copyright (c) 2024 Lily Lyons
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use fmod_sys::*;
use lanyard::Utf8CStr;
use std::{
    ffi::{c_float, c_int},
    mem::MaybeUninit,
    os::raw::c_void,
};

use crate::{core, Attributes3D, Guid, Vector};

use super::{
    AdvancedSettings, Bank, BufferUsage, Bus, CommandCaptureFlags, CommandReplay,
    CommandReplayFlags, EventDescription, InitFlags, LoadBankFlags, MemoryUsage,
    ParameterDescription, ParameterID, SoundInfo, Vca,
};

/// The main system object for FMOD Studio.
///
/// Initializing the FMOD Studio System object will also initialize the core System object.
///
/// Created with [`SystemBuilder`], which handles initialization for you.
#[derive(Debug, PartialEq, Eq, Clone, Copy)] // TODO: should this logically be copy?
#[repr(transparent)] // so we can transmute between types
pub struct System {
    pub(crate) inner: *mut FMOD_STUDIO_SYSTEM,
}

/// A builder for creating and initializing a [`System`].
///
/// Handles setting values that can only be set before initialization for you.
#[must_use]
pub struct SystemBuilder {
    system: *mut FMOD_STUDIO_SYSTEM,
    core_builder: crate::SystemBuilder,
}

impl SystemBuilder {
    /// Creates a new [`SystemBuilder`].
    ///
    /// # Safety
    ///
    /// Calling either of this function concurrently with any FMOD Studio API function (including this function) may cause undefined behavior.
    /// External synchronization must be used if calls to [`SystemBuilder::new`] or [`System::release`] could overlap other FMOD Studio API calls.
    /// All other FMOD Studio API functions are thread safe and may be called freely from any thread unless otherwise documented.
    pub unsafe fn new() -> Result<Self> {
        let mut system = std::ptr::null_mut();
        unsafe { FMOD_Studio_System_Create(&mut system, FMOD_VERSION).to_result()? };

        let mut core_system = std::ptr::null_mut();
        unsafe { FMOD_Studio_System_GetCoreSystem(system, &mut core_system).to_result()? };

        Ok(SystemBuilder {
            system,
            core_builder: crate::SystemBuilder {
                system: core_system,
            },
        })
    }

    pub fn settings(&mut self, settings: &AdvancedSettings) -> Result<&mut Self> {
        let mut settings = settings.into();
        // this function expects a pointer. maybe this is incorrect?
        unsafe { FMOD_Studio_System_SetAdvancedSettings(self.system, &mut settings).to_result() }?;
        Ok(self)
    }

    pub fn build(
        self,
        max_channels: c_int,
        studio_flags: InitFlags,
        flags: crate::InitFlags,
    ) -> Result<System> {
        unsafe {
            // we don't need
            self.build_with_extra_driver_data(
                max_channels,
                studio_flags,
                flags,
                std::ptr::null_mut(),
            )
        }
    }

    pub fn core_builder(&mut self) -> &mut crate::SystemBuilder {
        &mut self.core_builder
    }

    /// # Safety
    ///
    /// See the FMOD docs explaining driver data for more safety information.
    pub unsafe fn build_with_extra_driver_data(
        self,
        max_channels: c_int,
        studio_flags: InitFlags,
        flags: crate::InitFlags,
        driver_data: *mut c_void,
    ) -> Result<System> {
        unsafe {
            FMOD_Studio_System_Initialize(
                self.system,
                max_channels,
                studio_flags.bits(),
                flags.bits(),
                driver_data,
            )
            .to_result()?;
        }
        Ok(System { inner: self.system })
    }
}

impl System {
    /// Create a System instance from its FFI equivalent.
    ///
    /// # Safety
    /// This operation is unsafe because it's possible that the [`FMOD_STUDIO_SYSTEM`] will not have the right userdata type.
    pub unsafe fn from_ffi(value: *mut FMOD_STUDIO_SYSTEM) -> Self {
        System { inner: value }
    }
}

/// Convert a System instance to its FFI equivalent.
///
/// This is safe, provided you don't use the pointer.
impl From<System> for *mut FMOD_STUDIO_SYSTEM {
    fn from(value: System) -> Self {
        value.inner
    }
}

/// Most of FMOD is thread safe.
/// There are some select functions that are not thread safe to call, those are marked as unsafe.
unsafe impl Send for System {}
unsafe impl Sync for System {}

impl System {
    /// A convenience function over [`SystemBuilder`] with sane defaults.
    ///
    /// # Safety
    ///
    /// See [`SystemBuilder::new`] for safety info.
    pub unsafe fn new() -> Result<Self> {
        unsafe { SystemBuilder::new() }?.build(0, InitFlags::NORMAL, crate::InitFlags::NORMAL)
    }

    // TODO: could we solve this with an "owned" system and a shared system?
    ///This function will free the memory used by the Studio System object and everything created under it.
    ///
    /// # Safety
    ///
    /// Calling either of this function concurrently with any FMOD Studio API function (including this function) may cause undefined behavior.
    /// External synchronization must be used if calls to [`SystemBuilder::new`] or [`System::release`] could overlap other FMOD Studio API calls.
    /// All other FMOD Studio API functions are thread safe and may be called freely from any thread unless otherwise documented.
    ///
    /// All handles or pointers to objects associated with a Studio System object become invalid when the Studio System object is released.
    /// The FMOD Studio API attempts to protect against stale handles and pointers being used with a different Studio System object but this protection cannot be guaranteed and attempting to use stale handles or pointers may cause undefined behavior.
    ///
    /// This function is not safe to be called at the same time across multiple threads.
    pub unsafe fn release(self) -> Result<()> {
        unsafe { FMOD_Studio_System_Release(self.inner).to_result() }
    }

    /// Update the FMOD Studio System.
    ///
    /// When Studio is initialized in the default asynchronous processing mode this function submits all buffered commands for execution on the Studio Update thread for asynchronous processing.
    /// This is a fast operation since the commands are not processed on the calling thread.
    /// If Studio is initialized with [`InitFlags::DEFERRED_CALLBACKS`] then any deferred callbacks fired during any asynchronous updates since the last call to this function will be called.
    /// If an error occurred during any asynchronous updates since the last call to this function then this function will return the error result.
    ///
    /// When Studio is initialized with [`InitFlags::SYNCHRONOUS_UPDATE`] queued commands will be processed immediately when calling this function, the scheduling and update logic for the Studio system are executed and all callbacks are fired.
    /// This may block the calling thread for a substantial amount of time.
    pub fn update(&self) -> Result<()> {
        unsafe { FMOD_Studio_System_Update(self.inner) }.to_result()
    }

    /// This function blocks the calling thread until all pending commands have been executed and all non-blocking bank loads have been completed.
    ///
    /// This is equivalent to calling [`System::update`] and then sleeping until the asynchronous thread has finished executing all pending commands.
    pub fn flush_commands(&self) -> Result<()> {
        unsafe { FMOD_Studio_System_FlushCommands(self.inner) }.to_result()
    }

    /// Block until all sample loading and unloading has completed.
    ///
    /// This function may stall for a long time if other threads are continuing to issue calls to load and unload sample data, e.g. by creating new event instances.
    pub fn flush_sample_loading(&self) -> Result<()> {
        unsafe { FMOD_Studio_System_FlushSampleLoading(self.inner) }.to_result()
    }

    // TODO: load bank with callbacks
    pub fn load_bank_custom(&self) -> Result<Bank> {
        todo!()
    }

    /// Sample data must be loaded separately.
    ///
    /// By default this function will block until the file load finishes.
    ///
    /// Using the [`LoadBankFlags::NONBLOCKING`] flag will cause the bank to be loaded asynchronously.
    /// In that case this function will always return [`Ok`] and bank will contain a valid bank handle.
    /// Load errors for asynchronous banks can be detected by calling [`Bank::get_loading_state`].
    /// Failed asynchronous banks should be released by calling [`Bank::unload`].
    ///
    /// If a bank has been split, separating out assets and optionally streams from the metadata bank, all parts must be loaded before any APIs that use the data are called.
    /// It is recommended you load each part one after another (order is not important), then proceed with dependent API calls such as [`Bank::load_sample_data`] or [`System::get_event`].
    pub fn load_bank_file(&self, filename: &Utf8CStr, load_flags: LoadBankFlags) -> Result<Bank> {
        let mut bank = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_LoadBankFile(
                self.inner,
                filename.as_ptr(),
                load_flags.bits(),
                &mut bank,
            )
            .to_result()?;
            Ok(Bank::from_ffi(bank))
        }
    }

    /// Sample data must be loaded separately.
    ///
    /// This function is the safe counterpart of [`System::load_bank_pointer`].
    /// FMOD will allocate an internal buffer and copy the data from the passed in buffer before using it.
    /// The buffer passed to this function may be cleaned up at any time after this function returns.
    ///
    /// By default this function will block until the load finishes.
    ///
    /// Using the [`LoadBankFlags::NONBLOCKING`] flag will cause the bank to be loaded asynchronously.
    /// In that case this function will always return [`Ok`] and bank will contain a valid bank handle.
    /// Load errors for asynchronous banks can be detected by calling [`Bank::get_loading_state`].
    /// Failed asynchronous banks should be released by calling [`Bank::unload`].
    ///
    /// This function is not compatible with [`AdvancedSettings::encryption_key`], using them together will cause an error to be returned.
    ///
    /// If a bank has been split, separating out assets and optionally streams from the metadata bank, all parts must be loaded before any APIs that use the data are called.
    /// It is recommended you load each part one after another (order is not important), then proceed with dependent API calls such as [`Bank::load_sample_data`] or [`System::get_event`].
    pub fn load_bank_memory(&self, buffer: &[u8], flags: LoadBankFlags) -> Result<Bank> {
        let mut bank = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_LoadBankMemory(
                self.inner,
                buffer.as_ptr().cast::<i8>(),
                buffer.len() as c_int,
                FMOD_STUDIO_LOAD_MEMORY_MODE_FMOD_STUDIO_LOAD_MEMORY,
                flags.bits(),
                &mut bank,
            )
            .to_result()?;
            Ok(Bank::from_ffi(bank))
        }
    }

    /// Sample data must be loaded separately.
    ///
    /// This function is the unsafe counterpart of [`System::load_bank_memory`].
    /// FMOD will use the passed memory buffer directly.
    ///
    /// By default this function will block until the load finishes.
    ///
    /// Using the [`LoadBankFlags::NONBLOCKING`] flag will cause the bank to be loaded asynchronously.
    /// In that case this function will always return [`Ok`] and bank will contain a valid bank handle.
    /// Load errors for asynchronous banks can be detected by calling [`Bank::get_loading_state`].
    /// Failed asynchronous banks should be released by calling [`Bank::unload`].
    ///
    /// This function is not compatible with [`AdvancedSettings::encryption_key`], using them together will cause an error to be returned.
    ///
    /// If a bank has been split, separating out assets and optionally streams from the metadata bank, all parts must be loaded before any APIs that use the data are called.
    /// It is recommended you load each part one after another (order is not important), then proceed with dependent API calls such as [`Bank::load_sample_data`] or [`System::get_event`].
    ///
    /// # Safety
    /// When using this function the buffer must be aligned to [`FMOD_STUDIO_LOAD_MEMORY_ALIGNMENT`]
    /// and the memory must persist until the bank has been fully unloaded, which can be some time after calling [`Bank::unload`] to unload the bank.
    /// You can ensure the memory is not being freed prematurely by only freeing it after receiving the [`FMOD_STUDIO_SYSTEM_CALLBACK_BANK_UNLOAD`] callback.
    pub unsafe fn load_bank_pointer(
        &self,
        buffer: *const [u8],
        flags: LoadBankFlags,
    ) -> Result<Bank> {
        let mut bank = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_LoadBankMemory(
                self.inner,
                buffer.cast::<i8>(),
                (*buffer).len() as c_int,
                FMOD_STUDIO_LOAD_MEMORY_MODE_FMOD_STUDIO_LOAD_MEMORY_POINT,
                flags.bits(),
                &mut bank,
            )
            .to_result()?;
            Ok(Bank::from_ffi(bank))
        }
    }

    /// Unloads all currently loaded banks.
    pub fn unload_all_banks(&self) -> Result<()> {
        unsafe { FMOD_Studio_System_UnloadAll(self.inner).to_result() }
    }

    /// Retrieves a loaded bank
    ///
    /// `path_or_id` may be a path, such as `bank:/Weapons` or an ID string such as `{793cddb6-7fa1-4e06-b805-4c74c0fd625b}`.
    ///
    /// Note that path lookups will only succeed if the strings bank has been loaded.
    pub fn get_bank(&self, path_or_id: &Utf8CStr) -> Result<Bank> {
        let mut bank = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetBank(self.inner, path_or_id.as_ptr(), &mut bank).to_result()?;
            Ok(Bank::from_ffi(bank))
        }
    }

    /// Retrieves a loaded bank.
    pub fn get_bank_by_id(&self, id: Guid) -> Result<Bank> {
        let mut bank = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetBankByID(self.inner, &id.into(), &mut bank).to_result()?;
            Ok(Bank::from_ffi(bank))
        }
    }

    /// Retrieves the number of loaded banks.
    pub fn bank_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_System_GetBankCount(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    pub fn get_bank_list(&self) -> Result<Vec<Bank>> {
        let expected_count = self.bank_count()?;
        let mut count = 0;
        let mut list = vec![std::ptr::null_mut(); expected_count as usize];

        unsafe {
            FMOD_Studio_System_GetBankList(
                self.inner,
                // bank is repr transparent and has the same layout as *mut FMOD_STUDIO_BANK, so this cast is ok
                list.as_mut_ptr(),
                list.capacity() as c_int,
                &mut count,
            )
            .to_result()?;

            debug_assert_eq!(count, expected_count);

            Ok(std::mem::transmute::<
                Vec<*mut fmod_sys::FMOD_STUDIO_BANK>,
                Vec<Bank>,
            >(list))
        }
    }

    /// Sets the 3D attributes of the listener.
    pub fn set_listener_attributes(
        &self,
        listener: c_int,
        attributes: Attributes3D,
        attenuation_position: Option<Vector>,
    ) -> Result<()> {
        // we need to do this conversion seperately, for lifetime reasons
        let attenuation_position = attenuation_position.map(Into::into);
        unsafe {
            FMOD_Studio_System_SetListenerAttributes(
                self.inner,
                listener,
                &attributes.into(),
                attenuation_position
                    .as_ref()
                    .map_or(std::ptr::null(), std::ptr::from_ref),
            )
            .to_result()
        }
    }

    /// Retrieves listener 3D attributes.
    pub fn get_listener_attributes(&self, listener: c_int) -> Result<(Attributes3D, Vector)> {
        let mut attributes = MaybeUninit::uninit();
        let mut attenuation_position = MaybeUninit::uninit();

        unsafe {
            FMOD_Studio_System_GetListenerAttributes(
                self.inner,
                listener,
                attributes.as_mut_ptr(),
                attenuation_position.as_mut_ptr(),
            )
            .to_result()?;

            // TODO: check safety
            Ok((
                attributes.assume_init().into(),
                attenuation_position.assume_init().into(),
            ))
        }
    }

    /// Sets the listener weighting.
    ///
    /// Listener weighting is a factor which determines how much the listener influences the mix.
    /// It is taken into account for 3D panning, doppler, and the automatic distance event parameter. A listener with a weight of 0 has no effect on the mix.
    ///
    /// Listener weighting can be used to fade in and out multiple listeners.
    /// For example to do a crossfade, an additional listener can be created with a weighting of 0 that ramps up to 1 while the old listener weight is ramped down to 0.
    /// After the crossfade is finished the number of listeners can be reduced to 1 again.
    ///
    /// The sum of all the listener weights should add up to at least 1. It is a user error to set all listener weights to 0.
    pub fn set_listener_weight(&self, listener: c_int, weight: c_float) -> Result<()> {
        unsafe { FMOD_Studio_System_SetListenerWeight(self.inner, listener, weight).to_result() }
    }

    /// Retrieves listener weighting.
    pub fn get_listener_weight(&self, listener: c_int) -> Result<c_float> {
        let mut weight = 0.0;
        unsafe {
            FMOD_Studio_System_GetListenerWeight(self.inner, listener, &mut weight).to_result()?;
        }
        Ok(weight)
    }

    /// Sets the number of listeners in the 3D sound scene.
    ///
    /// If the number of listeners is set to more than 1 then FMOD uses a 'closest sound to the listener' method to determine what should be heard.
    pub fn set_listener_count(&self, amount: c_int) -> Result<()> {
        unsafe { FMOD_Studio_System_SetNumListeners(self.inner, amount).to_result() }
    }

    /// Sets the number of listeners in the 3D sound scene.
    ///
    /// If the number of listeners is set to more than 1 then FMOD uses a 'closest sound to the listener' method to determine what should be heard.
    pub fn get_listener_count(&self) -> Result<c_int> {
        let mut amount = 0;
        unsafe {
            FMOD_Studio_System_GetNumListeners(self.inner, &mut amount).to_result()?;
        }
        Ok(amount)
    }

    /// Retrieves a loaded [`Bus`].
    ///
    /// This function allows you to retrieve a handle for any bus in the global mixer.
    ///
    /// `path_or_id` may be a path, such as `bus:/SFX/Ambience`, or an ID string, such as `{d9982c58-a056-4e6c-b8e3-883854b4bffb}`.
    ///
    /// Note that path lookups will only succeed if the strings bank has been loaded.
    pub fn get_bus(&self, path_or_id: &Utf8CStr) -> Result<Bus> {
        let mut bus = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetBus(self.inner, path_or_id.as_ptr(), &mut bus).to_result()?;
        }
        Ok(bus.into())
    }

    /// Retrieves a loaded [`Bus`].
    ///
    /// This function allows you to retrieve a handle for any bus in the global mixer.
    pub fn get_bus_by_id(&self, id: Guid) -> Result<Bus> {
        let mut bus = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetBusByID(self.inner, &id.into(), &mut bus).to_result()?;
        }
        Ok(bus.into())
    }

    /// Retrieves an [`EventDescription`].
    ///
    /// This function allows you to retrieve a handle to any loaded event description.
    ///
    /// `path+or_id` may be a path, such as `event:/UI/Cancel` or `snapshot:/IngamePause`, or an ID string, such as `{2a3e48e6-94fc-4363-9468-33d2dd4d7b00}`.
    ///
    /// Note that path lookups will only succeed if the strings bank has been loaded.
    pub fn get_event(&self, path_or_id: &Utf8CStr) -> Result<EventDescription> {
        let mut event = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetEvent(self.inner, path_or_id.as_ptr(), &mut event).to_result()?;
            Ok(EventDescription::from_ffi(event))
        }
    }

    /// Retrieves an [`EventDescription`].
    ///
    /// This function allows you to retrieve a handle to any loaded event description.
    pub fn get_event_by_id(&self, id: Guid) -> Result<EventDescription> {
        let mut event = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetEventByID(self.inner, &id.into(), &mut event).to_result()?;
            Ok(EventDescription::from_ffi(event))
        }
    }

    /// Retrieves a global parameter value by unique identifier.
    ///
    /// The second tuple field is the final value of the parameter after applying adjustments due to automation, modulation, seek speed, and parameter velocity to value.
    /// This is calculated asynchronously when the Studio system updates.
    pub fn get_parameter_by_id(&self, id: ParameterID) -> Result<(c_float, c_float)> {
        let mut value = 0.0;
        let mut final_value = 0.0;

        unsafe {
            FMOD_Studio_System_GetParameterByID(
                self.inner,
                id.into(),
                &mut value,
                &mut final_value,
            )
            .to_result()?;
        }

        Ok((value, final_value))
    }

    /// Sets a global parameter value by unique identifier.
    pub fn set_parameter_by_id(
        &self,
        id: ParameterID,
        value: c_float,
        ignore_seek_speed: bool,
    ) -> Result<()> {
        unsafe {
            FMOD_Studio_System_SetParameterByID(
                self.inner,
                id.into(),
                value,
                ignore_seek_speed.into(),
            )
            .to_result()
        }
    }

    /// Sets a global parameter value by unique identifier, looking up the value label.
    ///
    /// If the specified label is not found, [`FMOD_RESULT::FMOD_ERR_EVENT_NOTFOUND`] is returned.
    /// This lookup is case sensitive.
    pub fn set_parameter_by_id_with_label(
        &self,
        id: ParameterID,
        label: &Utf8CStr,
        ignore_seek_speed: bool,
    ) -> Result<()> {
        unsafe {
            FMOD_Studio_System_SetParameterByIDWithLabel(
                self.inner,
                id.into(),
                label.as_ptr(),
                ignore_seek_speed.into(),
            )
            .to_result()
        }
    }

    /// Sets multiple global parameter values by unique identifier.
    ///
    /// If any ID is set to all zeroes then the corresponding value will be ignored.
    // TODO iterator version?
    pub fn set_parameters_by_ids(
        &self,
        ids: &[ParameterID], // TODO fmod says that the size of this must range from 1-32. do we need to enforce this?
        values: &mut [c_float], // TODO is this &mut correct? does fmod perform any writes?
        ignore_seek_speed: bool,
    ) -> Result<()> {
        // TODO don't panic, return result
        assert_eq!(ids.len(), values.len());

        unsafe {
            FMOD_Studio_System_SetParametersByIDs(
                self.inner,
                ids.as_ptr().cast(),
                values.as_mut_ptr(),
                ids.len() as c_int,
                ignore_seek_speed.into(),
            )
            .to_result()
        }
    }

    /// Retrieves a global parameter value by name.
    ///
    /// The second tuple field is the final value of the parameter after applying adjustments due to automation, modulation, seek speed, and parameter velocity to value.
    /// This is calculated asynchronously when the Studio system updates.
    pub fn get_parameter_by_name(&self, name: &Utf8CStr) -> Result<(c_float, c_float)> {
        let mut value = 0.0;
        let mut final_value = 0.0;

        unsafe {
            FMOD_Studio_System_GetParameterByName(
                self.inner,
                name.as_ptr(),
                &mut value,
                &mut final_value,
            )
            .to_result()?;
        }

        Ok((value, final_value))
    }

    /// Sets a global parameter value by name.
    pub fn set_parameter_by_name(
        &self,
        name: &Utf8CStr,
        value: c_float,
        ignore_seek_speed: bool,
    ) -> Result<()> {
        unsafe {
            FMOD_Studio_System_SetParameterByName(
                self.inner,
                name.as_ptr(),
                value,
                ignore_seek_speed.into(),
            )
            .to_result()
        }
    }

    /// Sets a global parameter value by name, looking up the value label.
    ///
    /// If the specified label is not found, [`FMOD_RESULT::FMOD_ERR_EVENT_NOTFOUND`] is returned. This lookup is case sensitive.
    pub fn set_parameter_by_name_with_label(
        &self,
        name: &Utf8CStr,
        label: &Utf8CStr,
        ignore_seek_speed: bool,
    ) -> Result<()> {
        unsafe {
            FMOD_Studio_System_SetParameterByNameWithLabel(
                self.inner,
                name.as_ptr(),
                label.as_ptr(),
                ignore_seek_speed.into(),
            )
            .to_result()
        }
    }

    /// Retrieves a global parameter by name or path.
    ///
    /// `name` can be the short name (such as `Wind`) or the full path (such as `parameter:/Ambience/Wind`).
    /// Path lookups will only succeed if the strings bank has been loaded.
    pub fn get_parameter_description_by_name(
        &self,
        name: &Utf8CStr,
    ) -> Result<ParameterDescription> {
        let mut description = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_System_GetParameterDescriptionByName(
                self.inner,
                name.as_ptr(),
                description.as_mut_ptr(),
            )
            .to_result()?;

            // FIXME lifetimes are incorrect and MUST be relaxed from 'static
            let description = ParameterDescription::from_ffi(description.assume_init());
            Ok(description)
        }
    }

    /// Retrieves a global parameter by ID.
    pub fn get_parameter_description_by_id(&self, id: ParameterID) -> Result<ParameterDescription> {
        let mut description = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_System_GetParameterDescriptionByID(
                self.inner,
                id.into(),
                description.as_mut_ptr(),
            )
            .to_result()?;

            // FIXME lifetimes are incorrect and MUST be relaxed from 'static
            let description = ParameterDescription::from_ffi(description.assume_init());
            Ok(description)
        }
    }

    /// Retrieves the number of global parameters.
    pub fn parameter_description_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_System_GetParameterDescriptionCount(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    /// Retrieves a list of global parameters.
    pub fn get_parameter_description_list(&self) -> Result<Vec<ParameterDescription>> {
        let expected_count = self.parameter_description_count()?;
        let mut count = 0;
        // FIXME: is the use of MaybeUninit necessary?
        // it does imply intention though, which is ok.
        let mut list = vec![MaybeUninit::zeroed(); expected_count as usize];

        unsafe {
            FMOD_Studio_System_GetParameterDescriptionList(
                self.inner,
                // bank is repr transparent and has the same layout as *mut FMOD_STUDIO_BANK, so this cast is ok
                list.as_mut_ptr()
                    .cast::<FMOD_STUDIO_PARAMETER_DESCRIPTION>(),
                list.capacity() as c_int,
                &mut count,
            )
            .to_result()?;

            debug_assert_eq!(count, expected_count);

            // FIXME lifetimes are incorrect and MUST be relaxed from 'static
            let list = list
                .into_iter()
                .map(|uninit| {
                    let description = uninit.assume_init();
                    ParameterDescription::from_ffi(description)
                })
                .collect();

            Ok(list)
        }
    }

    /// Retrieves a global parameter label by name or path.
    ///
    /// `name` can be the short name (such as `Wind`) or the full path (such as `parameter:/Ambience/Wind`).
    /// Path lookups will only succeed if the strings bank has been loaded.
    pub fn get_parameter_label_by_name(
        &self,
        name: &Utf8CStr,
        label_index: c_int,
    ) -> Result<String> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error = FMOD_Studio_System_GetParameterLabelByName(
                self.inner,
                name.as_ptr(),
                label_index,
                std::ptr::null_mut(),
                0,
                &mut string_len,
            )
            .to_error();

            // we expect the error to be fmod_err_truncated.
            // if it isn't, we return the error.
            match error {
                Some(error) if error != FMOD_RESULT::FMOD_ERR_TRUNCATED => return Err(error),
                _ => {}
            }
        };

        let mut path = vec![0u8; string_len as usize];
        let mut expected_string_len = 0;

        unsafe {
            FMOD_Studio_System_GetParameterLabelByName(
                self.inner,
                name.as_ptr(),
                label_index,
                // u8 and i8 have the same layout, so this is ok
                path.as_mut_ptr().cast(),
                string_len,
                &mut expected_string_len,
            )
            .to_result()?;

            debug_assert_eq!(string_len, expected_string_len);

            // all public fmod apis return UTF-8 strings. this should be safe.
            // if i turn out to be wrong, perhaps we should add extra error types?
            let path = String::from_utf8_unchecked(path);

            Ok(path)
        }
    }

    /// Retrieves a global parameter label by ID.
    pub fn get_parameter_label_by_id(&self, id: ParameterID, label_index: c_int) -> Result<String> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error = FMOD_Studio_System_GetParameterLabelByID(
                self.inner,
                id.into(),
                label_index,
                std::ptr::null_mut(),
                0,
                &mut string_len,
            )
            .to_error();

            // we expect the error to be fmod_err_truncated.
            // if it isn't, we return the error.
            match error {
                Some(error) if error != FMOD_RESULT::FMOD_ERR_TRUNCATED => return Err(error),
                _ => {}
            }
        };

        let mut path = vec![0u8; string_len as usize];
        let mut expected_string_len = 0;

        unsafe {
            FMOD_Studio_System_GetParameterLabelByID(
                self.inner,
                id.into(),
                label_index,
                // u8 and i8 have the same layout, so this is ok
                path.as_mut_ptr().cast(),
                string_len,
                &mut expected_string_len,
            )
            .to_result()?;

            debug_assert_eq!(string_len, expected_string_len);

            // all public fmod apis return UTF-8 strings. this should be safe.
            // if i turn out to be wrong, perhaps we should add extra error types?
            let path = String::from_utf8_unchecked(path);

            Ok(path)
        }
    }

    /// Retrieves a loaded VCA.
    ///
    /// This function allows you to retrieve a handle for any VCA in the global mixer.
    ///
    /// `path_or_id` may be a path, such as `vca:/MyVCA`, or an ID string, such as `{d9982c58-a056-4e6c-b8e3-883854b4bffb`}.
    ///
    /// Note that path lookups will only succeed if the strings bank has been loaded.
    pub fn get_vca(&self, path_or_id: &Utf8CStr) -> Result<Vca> {
        let mut vca = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetVCA(self.inner, path_or_id.as_ptr(), &mut vca).to_result()?;
        }
        Ok(vca.into())
    }

    /// Retrieves a loaded VCA.
    ///
    /// This function allows you to retrieve a handle for any VCA in the global mixer.
    pub fn get_vca_by_id(&self, id: Guid) -> Result<Vca> {
        let mut vca = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetVCAByID(self.inner, &id.into(), &mut vca).to_result()?;
        }
        Ok(vca.into())
    }

    /// Retrieves advanced settings.
    pub fn get_advanced_settings(&self) -> Result<AdvancedSettings> {
        let mut advanced_settings = MaybeUninit::zeroed();

        unsafe {
            FMOD_Studio_System_GetAdvancedSettings(self.inner, advanced_settings.as_mut_ptr())
                .to_result()?;

            // FIXME advancedsettings here is a 'static. this is probably invalid!
            let advanced_settings = AdvancedSettings::from_ffi(advanced_settings.assume_init());

            Ok(advanced_settings)
        }
    }

    /// Recording Studio commands to a file.
    ///
    /// The commands generated by the FMOD Studio API can be captured and later replayed for debug and profiling purposes.
    ///
    /// Unless the [`CommandCaptureFlags::SKIP_INITIAL_STATE`] flag is specified, the command capture will first record the set of all banks and event instances that currently exist.
    pub fn start_command_capture(
        &self,
        filename: &Utf8CStr,
        flags: CommandCaptureFlags,
    ) -> Result<()> {
        unsafe {
            FMOD_Studio_System_StartCommandCapture(self.inner, filename.as_ptr(), flags.into())
                .to_result()
        }
    }

    /// Stop recording Studio commands.
    pub fn stop_command_capture(&self) -> Result<()> {
        unsafe { FMOD_Studio_System_StopCommandCapture(self.inner).to_result() }
    }

    /// Load a command replay.
    pub fn load_command_replay(
        &self,
        filename: &Utf8CStr,
        flags: CommandReplayFlags,
    ) -> Result<CommandReplay> {
        let mut replay = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_LoadCommandReplay(
                self.inner,
                filename.as_ptr(),
                flags.into(),
                &mut replay,
            )
            .to_result()?;
            Ok(CommandReplay::from_ffi(replay))
        }
    }

    /// Retrieves buffer usage information.
    ///
    /// Stall count and time values are cumulative. They can be reset by calling [`System::reset_buffer_usage`].
    ///
    /// Stalls due to the studio command queue overflowing can be avoided by setting a larger command queue size with [`SystemBuilder::settings`].
    pub fn get_buffer_usage(&self) -> Result<BufferUsage> {
        let mut usage = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_System_GetBufferUsage(self.inner, usage.as_mut_ptr()).to_result()?;

            let usage = usage.assume_init().into();
            Ok(usage)
        }
    }

    /// Resets memory buffer usage statistics.
    ///
    /// This function resets the buffer usage data tracked by the FMOD Studio System.
    pub fn reset_buffer_usage(&self) -> Result<()> {
        unsafe { FMOD_Studio_System_ResetBufferUsage(self.inner).to_result() }
    }

    /// Retrieves the amount of CPU used for different parts of the Studio engine.
    ///
    /// For readability, the percentage values are smoothed to provide a more stable output.
    pub fn get_cpu_usage(&self) -> Result<(super::CpuUsage, crate::CpuUsage)> {
        let mut usage = MaybeUninit::zeroed();
        let mut usage_core = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_System_GetCPUUsage(self.inner, usage.as_mut_ptr(), usage_core.as_mut_ptr())
                .to_result()?;

            let usage = usage.assume_init().into();
            let usage_core = usage_core.assume_init().into();
            Ok((usage, usage_core))
        }
    }

    /// Retrieves memory usage statistics.
    ///
    /// The memory usage `sample_data` field for the system is the total size of non-streaming sample data currently loaded.
    ///
    /// Memory usage statistics are only available in logging builds, in release builds memoryusage will contain zero for all values after calling this function.
    pub fn get_memory_usage(&self) -> Result<MemoryUsage> {
        let mut usage = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_System_GetMemoryUsage(self.inner, usage.as_mut_ptr()).to_result()?;

            let usage = usage.assume_init().into();
            Ok(usage)
        }
    }

    /// Registers a plugin DSP.
    ///
    /// Plugin DSPs used by an event must be registered using this function before loading the bank containing the event.
    ///
    /// # Safety
    /// TODO
    pub unsafe fn register_plugin(&self) {
        todo!()
    }

    /// Unregisters a plugin DSP.
    ///
    /// # Safety
    /// TODO
    pub unsafe fn unregister_plugin(&self) {
        todo!()
    }

    /// Retrieves information for loading a sound from the audio table.
    ///
    /// The [`SoundInfo`] structure contains information to be passed to [`crate::System::create_sound`] (which will create a parent sound),
    /// along with a subsound index to be passed to [`crate::Sound::get_sub_sound`] once the parent sound is loaded.
    ///
    /// The user is expected to call [`System::create_sound `]with the given information.
    /// It is up to the user to combine in any desired loading flags, such as [`FMOD_CREATESTREAM`], [`FMOD_CREATECOMPRESSEDSAMPLE`] or [`FMOD_NONBLOCKING`] with the flags in [`FMOD_STUDIO_SOUND_INFO::mode`].
    ///
    /// When the banks have been loaded via [`System::load_bank_memory`], the mode will be returned as [`FMOD_OPENMEMORY_POINT`].
    /// This won't work with the default [`FMOD_CREATESAMPLE`] mode.
    /// For memory banks, you should add in the [`FMOD_CREATECOMPRESSEDSAMPLE`] or [`FMOD_CREATESTREAM`] flag, or remove [`FMOD_OPENMEMORY_POINT`] and add [`FMOD_OPENMEMORY`] to decompress the sample into a new allocation.
    // TODO flags
    pub fn get_sound_info(&self, key: &Utf8CStr) -> Result<SoundInfo> {
        let mut sound_info = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_System_GetSoundInfo(self.inner, key.as_ptr(), sound_info.as_mut_ptr())
                .to_result()?;

            let sound_info = SoundInfo::from_ffi(sound_info.assume_init());
            Ok(sound_info)
        }
    }

    /// Retrieves the Core System.
    pub fn get_core_system(&self) -> Result<core::System> {
        let mut system = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetCoreSystem(self.inner, &mut system).to_result()?;
        }
        Ok(system.into())
    }

    /// Retrieves the ID for a bank, event, snapshot, bus or VCA.
    ///
    /// The strings bank must be loaded prior to calling this function, otherwise [`FMOD_RESULT::FMOD_ERR_EVENT_NOTFOUND`] is returned.
    ///
    /// The path can be copied to the system clipboard from FMOD Studio using the "Copy Path" context menu command.
    pub fn lookup_id(&self, path: &Utf8CStr) -> Result<Guid> {
        let mut guid = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_System_LookupID(self.inner, path.as_ptr(), guid.as_mut_ptr())
                .to_result()?;

            let guid = guid.assume_init().into();
            Ok(guid)
        }
    }

    /// Retrieves the path for a bank, event, snapshot, bus or VCA.
    ///
    /// The strings bank must be loaded prior to calling this function, otherwise [`FMOD_RESULT::FMOD_ERR_EVENT_NOTFOUND`] is returned.
    pub fn lookup_path(&self, id: Guid) -> Result<String> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error = FMOD_Studio_System_LookupPath(
                self.inner,
                &id.into(),
                std::ptr::null_mut(),
                0,
                &mut string_len,
            )
            .to_error();

            // we expect the error to be fmod_err_truncated.
            // if it isn't, we return the error.
            match error {
                Some(error) if error != FMOD_RESULT::FMOD_ERR_TRUNCATED => return Err(error),
                _ => {}
            }
        };

        let mut path = vec![0u8; string_len as usize];
        let mut expected_string_len = 0;

        unsafe {
            FMOD_Studio_System_LookupPath(
                self.inner,
                &id.into(),
                // u8 and i8 have the same layout, so this is ok
                path.as_mut_ptr().cast(),
                string_len,
                &mut expected_string_len,
            )
            .to_result()?;

            debug_assert_eq!(string_len, expected_string_len);

            // all public fmod apis return UTF-8 strings. this should be safe.
            // if i turn out to be wrong, perhaps we should add extra error types?
            let path = String::from_utf8_unchecked(path);

            Ok(path)
        }
    }

    /// Checks that the [`System`] reference is valid and has been initialized.
    pub fn is_valid(&self) -> bool {
        unsafe { FMOD_Studio_System_IsValid(self.inner).into() }
    }
}