// Copyright (c) 2024 Lily Lyons
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    ffi::{c_float, c_int},
    mem::MaybeUninit,
};

use fmod_sys::*;
use lanyard::Utf8CStr;

use crate::studio::{EventInstance, LoadingState, ParameterDescription, ParameterID, UserProperty};
use crate::Guid;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)] // so we can transmute between types
pub struct EventDescription {
    pub(crate) inner: *mut FMOD_STUDIO_EVENTDESCRIPTION,
}

unsafe impl Send for EventDescription {}
unsafe impl Sync for EventDescription {}

impl EventDescription {
    /// Create a System instance from its FFI equivalent.
    ///
    /// # Safety
    /// This operation is unsafe because it's possible that the [`FMOD_STUDIO_EVENTDESCRIPTION`] will not have the right userdata type.
    pub unsafe fn from_ffi(value: *mut FMOD_STUDIO_EVENTDESCRIPTION) -> Self {
        EventDescription { inner: value }
    }
}

impl From<EventDescription> for *mut FMOD_STUDIO_EVENTDESCRIPTION {
    fn from(value: EventDescription) -> Self {
        value.inner
    }
}

impl EventDescription {
    /// Creates a playable instance.
    ///
    /// When an event instance is created, any required non-streaming sample data is loaded asynchronously.
    ///
    /// Use [`EventDescription::get_sample_loading_state`] to check the loading status.
    ///
    /// Sample data can be loaded ahead of time with [`EventDescription::load_sample_data`] or [`super::Bank::load_sample_data`]. See Sample Data Loading for more information.
    pub fn create_instance(&self) -> Result<EventInstance> {
        let mut instance = std::ptr::null_mut();
        unsafe {
            FMOD_Studio_EventDescription_CreateInstance(self.inner, &mut instance).to_result()?;
            Ok(EventInstance::from_ffi(instance))
        }
    }

    /// Retrieves the number of instances.
    pub fn instance_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_EventDescription_GetInstanceCount(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    pub fn get_instance_list(&self) -> Result<Vec<EventInstance>> {
        let expected_count = self.instance_count()?;
        let mut count = 0;
        let mut list = vec![std::ptr::null_mut(); expected_count as usize];

        unsafe {
            FMOD_Studio_EventDescription_GetInstanceList(
                self.inner,
                // eventinstance is repr transparent and has the same layout as *mut FMOD_STUDIO_EVENTINSTANCE, so this cast is ok
                list.as_mut_ptr(),
                list.capacity() as c_int,
                &mut count,
            )
            .to_result()?;

            debug_assert_eq!(count, expected_count);

            // *mut FMOD_STUDIO_EVENTINSTANCE is transmutable to EventInstance
            Ok(std::mem::transmute::<
                Vec<*mut fmod_sys::FMOD_STUDIO_EVENTINSTANCE>,
                Vec<EventInstance>,
            >(list))
        }
    }

    /// Releases all instances.
    ///
    /// This function immediately stops and releases all instances of the event.
    pub fn release_all_instances(&self) -> Result<()> {
        unsafe { FMOD_Studio_EventDescription_ReleaseAllInstances(self.inner).to_result() }
    }

    /// Loads non-streaming sample data used by the event.
    ///
    /// This function will load all non-streaming sample data required by the event and any referenced events.
    ///
    /// Sample data is loaded asynchronously, [`EventDescription::get_sample_loading_state`] may be used to poll the loading state.
    pub fn load_sample_data(&self) -> Result<()> {
        unsafe { FMOD_Studio_EventDescription_LoadSampleData(self.inner).to_result() }
    }

    /// Unloads all non-streaming sample data.
    ///
    /// Sample data will not be unloaded until all instances of the event are released.
    pub fn unload_sample_data(&self) -> Result<()> {
        unsafe { FMOD_Studio_EventDescription_UnloadSampleData(self.inner).to_result() }
    }

    /// Retrieves the sample data loading state.
    ///
    /// If the event is invalid, then the returned state is [`LoadingState::Unloaded`] and this function returns [`FMOD_RESULT::FMOD_ERR_INVALID_HANDLE`].
    pub fn get_sample_loading_state(&self) -> Result<LoadingState> {
        let mut loading_state = 0;

        let error = unsafe {
            FMOD_Studio_EventDescription_GetSampleLoadingState(self.inner, &mut loading_state)
                .to_error()
        };

        LoadingState::try_from_ffi(loading_state, error)
    }

    /// Retrieves the event's 3D status.
    ///
    /// An event is considered 3D if any of these conditions are met:
    ///  - The event has a Spatializer, 3D Object Spatializer, or a 3rd party spatializer on its master track.
    ///  - The event contains an automatic parameter that depends on the event's 3D attributes:
    ///    - Distance
    ///    - Event Cone Angle
    ///    - Event Orientation
    ///    - Direction
    ///    - Elevation
    ///    - Speed
    ///    - Speed (Absolute)
    ///  - The event contains any nested events which are 3D.
    ///
    /// Note: If the event contains nested events built to separate banks using versions of FMOD Studio prior to 2.00.10 and those banks have not been loaded then this function may fail to correctly determine the event's 3D status.
    pub fn is_3d(&self) -> Result<bool> {
        let mut is_3d = FMOD_BOOL(0);
        unsafe {
            FMOD_Studio_EventDescription_Is3D(self.inner, &mut is_3d).to_result()?;
        }
        Ok(is_3d.into())
    }

    /// Retrieves the event's doppler status.
    ///
    /// Note: If the event was built to a bank using versions of FMOD Studio prior to 2.01.09, then this function will return false regardless of the event's doppler state.
    pub fn is_doppler_enabled(&self) -> Result<bool> {
        let mut is_doppler = FMOD_BOOL(0);
        unsafe {
            FMOD_Studio_EventDescription_IsDopplerEnabled(self.inner, &mut is_doppler)
                .to_result()?;
        }
        Ok(is_doppler.into())
    }

    /// Retrieves the event's oneshot status.
    ///
    /// An event is considered oneshot if it is guaranteed to terminate without intervention in bounded time after being started.
    /// Instances of such events can be played in a fire-and-forget fashion by calling [`EventInstance::start`] immediately followed by [`EventInstance::release`].
    ///
    /// Note: If the event contains nested events built to separate banks and those banks have not been loaded then this function may fail to correctly determine the event's oneshot status.
    pub fn is_oneshot(&self) -> Result<bool> {
        let mut is_oneshot = FMOD_BOOL(0);
        unsafe {
            FMOD_Studio_EventDescription_IsOneshot(self.inner, &mut is_oneshot).to_result()?;
        }
        Ok(is_oneshot.into())
    }

    /// Retrieves the event's snapshot status.
    pub fn is_snapshot(&self) -> Result<bool> {
        let mut is_snapshot = FMOD_BOOL(0);
        unsafe {
            FMOD_Studio_EventDescription_IsSnapshot(self.inner, &mut is_snapshot).to_result()?;
        }
        Ok(is_snapshot.into())
    }

    /// Retrieves the event's stream status.
    ///
    /// Note: If the event contains nested events built to separate banks and those banks have not been loaded then this function may fail to correctly determine the event's stream status.
    pub fn is_stream(&self) -> Result<bool> {
        let mut is_stream = FMOD_BOOL(0);
        unsafe {
            FMOD_Studio_EventDescription_IsStream(self.inner, &mut is_stream).to_result()?;
        }
        Ok(is_stream.into())
    }

    /// Retrieves whether the event has any sustain points.
    pub fn has_sustain_point(&self) -> Result<bool> {
        let mut sustain_point = FMOD_BOOL(0);
        unsafe {
            FMOD_Studio_EventDescription_HasSustainPoint(self.inner, &mut sustain_point)
                .to_result()?;
        }
        Ok(sustain_point.into())
    }

    /// Retrieves the minimum and maximum distances for 3D attenuation.
    pub fn get_min_max_distance(&self) -> Result<(c_float, c_float)> {
        let mut min = 0.0;
        let mut max = 0.0;
        unsafe {
            FMOD_Studio_EventDescription_GetMinMaxDistance(self.inner, &mut min, &mut max)
                .to_result()?;
        }
        Ok((min, max))
    }

    /// Retrieves the sound size for 3D panning.
    ///
    /// Retrieves the largest Sound Size value of all Spatializers and 3D Object Spatializers on the event's master track. Returns zero if there are no Spatializers or 3D Object Spatializers.
    pub fn get_sound_size(&self) -> Result<c_float> {
        let mut size = 0.0;
        unsafe {
            FMOD_Studio_EventDescription_GetSoundSize(self.inner, &mut size).to_result()?;
        }
        Ok(size)
    }

    /// Retrieves an event parameter description by name.
    pub fn get_parameter_description_by_name(
        &self,
        name: &Utf8CStr,
    ) -> Result<ParameterDescription> {
        let mut description = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_EventDescription_GetParameterDescriptionByName(
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

    /// Retrieves an event parameter description by id.
    pub fn get_parameter_description_by_id(&self, id: ParameterID) -> Result<ParameterDescription> {
        let mut description = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_EventDescription_GetParameterDescriptionByID(
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

    /// Retrieves an event parameter description by index.
    ///
    /// May be used in combination with [`EventDescription::parameter_description_count`] to enumerate event parameters.
    ///
    /// Note: The order of parameters is not necessarily the same as what is shown in the FMOD Studio event editor.
    pub fn get_parameter_description_by_index(&self, index: c_int) -> Result<ParameterDescription> {
        let mut description = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_EventDescription_GetParameterDescriptionByIndex(
                self.inner,
                index,
                description.as_mut_ptr(),
            )
            .to_result()?;

            // FIXME lifetimes are incorrect and MUST be relaxed from 'static
            let description = ParameterDescription::from_ffi(description.assume_init());
            Ok(description)
        }
    }

    /// Retrieves the number of parameters in the event.
    ///
    /// May be used in conjunction with [`EventDescription::get_parameter_description_by_index`] to enumerate event parameters.
    pub fn parameter_description_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_EventDescription_GetParameterDescriptionCount(self.inner, &mut count)
                .to_result()?;
        }
        Ok(count)
    }

    /// Retrieves an event parameter label by name or path.
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
            let error = FMOD_Studio_EventDescription_GetParameterLabelByName(
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
            FMOD_Studio_EventDescription_GetParameterLabelByName(
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

    /// Retrieves an event parameter label by ID.
    pub fn get_parameter_label_by_id(&self, id: ParameterID, label_index: c_int) -> Result<String> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error = FMOD_Studio_EventDescription_GetParameterLabelByID(
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
            FMOD_Studio_EventDescription_GetParameterLabelByID(
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

    /// Retrieves an event parameter label by index.
    ///
    /// May be used in combination with [`EventDescription::parameter_description_count`] to enumerate event parameters.
    pub fn get_parameter_label_by_index(&self, index: c_int, label_index: c_int) -> Result<String> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error = FMOD_Studio_EventDescription_GetParameterLabelByIndex(
                self.inner,
                index,
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
            FMOD_Studio_EventDescription_GetParameterLabelByIndex(
                self.inner,
                index,
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

    /// Retrieves a user property by name.
    pub fn get_user_property(&self, name: &Utf8CStr) -> Result<UserProperty> {
        let mut property = MaybeUninit::uninit();
        unsafe {
            FMOD_Studio_EventDescription_GetUserProperty(
                self.inner,
                name.as_ptr(),
                property.as_mut_ptr(),
            )
            .to_result()?;

            // FIXME wrong lifetimes + wildly unsafe
            let property = UserProperty::from_ffi(property.assume_init());
            Ok(property)
        }
    }

    /// Retrieves a user property by index.
    ///
    /// May be used in combination with [`EventDescription::user_property_count`] to enumerate event user properties.
    pub fn get_user_property_by_index(&self, index: c_int) -> Result<UserProperty> {
        let mut property = MaybeUninit::uninit();
        unsafe {
            FMOD_Studio_EventDescription_GetUserPropertyByIndex(
                self.inner,
                index,
                property.as_mut_ptr(),
            )
            .to_result()?;

            // FIXME wrong lifetimes + wildly unsafe
            let property = UserProperty::from_ffi(property.assume_init());
            Ok(property)
        }
    }

    pub fn user_property_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_EventDescription_GetUserPropertyCount(self.inner, &mut count)
                .to_result()?;
        }
        Ok(count)
    }

    /// Retrieves the GUID.
    pub fn get_id(&self) -> Result<Guid> {
        let mut guid = MaybeUninit::zeroed();
        unsafe {
            FMOD_Studio_EventDescription_GetID(self.inner, guid.as_mut_ptr()).to_result()?;

            let guid = guid.assume_init().into();

            Ok(guid)
        }
    }

    /// Retrieves the length of the timeline.
    ///
    /// A timeline's length is the largest of any logic markers, transition leadouts and the end of any trigger boxes on the timeline.
    pub fn get_length(&self) -> Result<c_int> {
        let mut length = 0;
        unsafe {
            FMOD_Studio_EventDescription_GetLength(self.inner, &mut length).to_result()?;
        }
        Ok(length)
    }

    /// Retrieves the path.
    ///
    /// The strings bank must be loaded prior to calling this function, otherwise [`FMOD_RESULT::FMOD_ERR_EVENT_NOTFOUND`] is returned.
    // TODO: convert into possible macro for the sake of reusing code
    pub fn get_path(&self) -> Result<String> {
        let mut string_len = 0;

        // retrieve the length of the string.
        // this includes the null terminator, so we don't need to account for that.
        unsafe {
            let error = FMOD_Studio_EventDescription_GetPath(
                self.inner,
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
            FMOD_Studio_EventDescription_GetPath(
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

    /// Checks that the [`EventDescription`] reference is valid.
    pub fn is_valid(&self) -> bool {
        unsafe { FMOD_Studio_EventDescription_IsValid(self.inner).into() }
    }
}
