// Copyright (c) 2024 Lily Lyons
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ffi::c_int, mem::MaybeUninit};

use crate::Guid;

use super::{Bus, EventDescription, LoadingState, Vca};
use fmod_sys::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)] // so we can transmute between types
pub struct Bank {
    pub(crate) inner: *mut FMOD_STUDIO_BANK,
}

unsafe impl Send for Bank {}
unsafe impl Sync for Bank {}

impl Bank {
    /// Create a System instance from its FFI equivalent.
    ///
    /// # Safety
    /// This operation is unsafe because it's possible that the [`FMOD_STUDIO_BANK`] will not have the right userdata type.
    pub unsafe fn from_ffi(value: *mut FMOD_STUDIO_BANK) -> Self {
        Bank { inner: value }
    }
}

impl From<Bank> for *mut FMOD_STUDIO_BANK {
    fn from(value: Bank) -> Self {
        value.inner
    }
}

impl Bank {
    /// This function may be used to check the loading state of a bank which has been loaded asynchronously using the [`super::LoadBankFlags::NONBLOCKING`] flag,
    /// or is pending unload following a call to [`Bank::unload`].
    ///
    /// If an asynchronous load failed due to a file error state will contain [`LoadingState::Error`] and the return code from this function will be the error code of the bank load function.
    // TODO: make LoadingState contain the error?
    pub fn get_loading_state(&self) -> Result<LoadingState> {
        let mut loading_state = 0;
        let error =
            unsafe { FMOD_Studio_Bank_GetLoadingState(self.inner, &mut loading_state).to_error() };

        LoadingState::try_from_ffi(loading_state, error)
    }

    /// Use this function to preload sample data ahead of time so that the events in the bank can play immediately when started.
    ///
    /// This function is equivalent to calling [`super::EventDescription::load_sample_data`] for all events in the bank, including referenced events.
    pub fn load_sample_data(&self) -> Result<()> {
        unsafe { FMOD_Studio_Bank_LoadSampleData(self.inner).to_result() }
    }

    /// Unloads non-streaming sample data for all events in the bank.
    ///
    /// Sample data loading is reference counted and the sample data will remain loaded until unload requests corresponding to all load requests are made, or until the bank is unloaded.
    pub fn unload_sample_data(&self) -> Result<()> {
        unsafe { FMOD_Studio_Bank_UnloadSampleData(self.inner).to_result() }
    }

    /// Retrieves the loading state of the samples in the bank.
    ///
    /// May be used for tracking the status of the [`Bank::load_sample_data`] operation.
    ///
    /// If [`Bank::load_sample_data`] has not been called for the bank then this function will return [`LoadingState::Unloaded`] even though sample data may have been loaded by other API calls.
    pub fn get_sample_loading_state(&self) -> Result<LoadingState> {
        let mut loading_state = 0;
        let error =
            unsafe { FMOD_Studio_Bank_GetLoadingState(self.inner, &mut loading_state).to_error() };
        LoadingState::try_from_ffi(loading_state, error)
    }

    /// Unloads the bank.
    ///
    /// This will destroy all objects created from the bank, unload all sample data inside the bank, and invalidate all API handles referring to the bank.
    ///
    /// If the bank was loaded from user-managed memory, e.g. by [`super::System::load_bank_pointer`], then the memory must not be freed until the unload has completed.
    /// Poll the loading state using [`Bank::get_loading_state`] or use the [`FMOD_STUDIO_SYSTEM_CALLBACK_BANK_UNLOAD`] system callback to determine when it is safe to free the memory.
    pub fn unload(self) -> Result<()> {
        // we don't deallocate userdata here because the system callback will take care of that for us
        unsafe { FMOD_Studio_Bank_Unload(self.inner).to_result() }
    }

    /// Retrieves the number of buses in the bank.
    pub fn bus_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_Bank_GetBusCount(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    /// Retrieves a list of the buses in the bank.
    pub fn get_bus_list(&self) -> Result<Vec<Bus>> {
        let expected_count = self.bus_count()?;
        let mut count = 0;
        let mut list = vec![
            Bus {
                inner: std::ptr::null_mut()
            };
            expected_count as usize
        ];

        unsafe {
            FMOD_Studio_Bank_GetBusList(
                self.inner,
                // bus is repr transparent and has the same layout as *mut FMOD_STUDIO_BUS, so this cast is ok
                list.as_mut_ptr().cast::<*mut FMOD_STUDIO_BUS>(),
                list.capacity() as c_int,
                &mut count,
            )
            .to_result()?;

            debug_assert_eq!(count, expected_count);

            Ok(list)
        }
    }

    /// Retrives the number of event descriptions in the bank.
    ///
    /// This function counts the events which were added to the bank by the sound designer.
    /// The bank may contain additional events which are referenced by event instruments but were not added to the bank, and those referenced events are not counted.
    pub fn event_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_Bank_GetEventCount(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    /// Retrieves a list of the event descriptions in the bank.
    ///
    /// This function counts the events which were added to the bank by the sound designer.
    /// The bank may contain additional events which are referenced by event instruments but were not added to the bank, and those referenced events are not counted.
    pub fn get_event_list(&self) -> Result<Vec<EventDescription>> {
        let expected_count = self.event_count()?;
        let mut count = 0;
        let mut list = vec![std::ptr::null_mut(); expected_count as usize];

        unsafe {
            FMOD_Studio_Bank_GetEventList(
                self.inner,
                // bus is repr transparent and has the same layout as *mut FMOD_STUDIO_BUS, so this cast is ok
                list.as_mut_ptr(),
                list.capacity() as c_int,
                &mut count,
            )
            .to_result()?;

            debug_assert_eq!(count, expected_count);

            Ok(std::mem::transmute::<
                Vec<*mut fmod_sys::FMOD_STUDIO_EVENTDESCRIPTION>,
                Vec<EventDescription>,
            >(list))
        }
    }

    /// Retrieves the number of string table entries in the bank.
    pub fn string_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_Bank_GetStringCount(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    /// Retrieves a string table entry.
    ///
    /// May be used in conjunction with [`Bank::string_count`] to enumerate the string table in a bank.

    pub fn get_string_info(&self, index: c_int) -> Result<(Guid, String)> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error = FMOD_Studio_Bank_GetStringInfo(
                self.inner,
                index,
                std::ptr::null_mut(),
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

        let mut guid = MaybeUninit::zeroed();
        let mut path = vec![0u8; string_len as usize];
        let mut expected_string_len = 0;

        unsafe {
            FMOD_Studio_Bank_GetStringInfo(
                self.inner,
                index,
                guid.as_mut_ptr(),
                // u8 and i8 have the same layout, so this is ok
                path.as_mut_ptr().cast(),
                string_len,
                &mut expected_string_len,
            )
            .to_result()?;

            debug_assert_eq!(string_len, expected_string_len);

            // even if fmod didn't write to guid, guid should be safe to zero initialize.
            let guid = guid.assume_init().into();
            // all public fmod apis return UTF-8 strings. this should be safe.
            // if i turn out to be wrong, perhaps we should add extra error types?
            let path = String::from_utf8_unchecked(path);

            Ok((guid, path))
        }
    }

    /// Retrieves the number of VCAs in the bank.
    pub fn vca_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_Bank_GetVCACount(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    /// Retrieves a list of the VCAs in the bank.
    pub fn get_vca_list(&self) -> Result<Vec<Vca>> {
        let expected_count = self.event_count()?;
        let mut count = 0;
        let mut list = vec![
            Vca {
                inner: std::ptr::null_mut()
            };
            expected_count as usize
        ];

        unsafe {
            FMOD_Studio_Bank_GetVCAList(
                self.inner,
                // bus is repr transparent and has the same layout as *mut FMOD_STUDIO_BUS, so this cast is ok
                list.as_mut_ptr().cast::<*mut FMOD_STUDIO_VCA>(),
                list.capacity() as c_int,
                &mut count,
            )
            .to_result()?;

            debug_assert_eq!(count, expected_count);

            Ok(list)
        }
    }

    /// Retrieves the GUID.
    pub fn get_id(&self) -> Result<Guid> {
        let mut guid = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_Bank_GetID(self.inner, guid.as_mut_ptr()).to_result()?;

            let guid = guid.assume_init().into();

            Ok(guid)
        }
    }

    /// Retrieves the path.
    pub fn get_path(&self) -> Result<String> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error =
                FMOD_Studio_Bank_GetPath(self.inner, std::ptr::null_mut(), 0, &mut string_len)
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
            FMOD_Studio_Bank_GetPath(
                self.inner,
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

    /// Checks that the Bank reference is valid.
    pub fn is_valid(&self) -> bool {
        unsafe { FMOD_Studio_Bank_IsValid(self.inner).into() }
    }
}