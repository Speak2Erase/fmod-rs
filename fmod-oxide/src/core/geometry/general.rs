// Copyright (c) 2024 Lily Lyons
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use fmod_sys::*;
use std::ffi::{c_float, c_int, c_void};

use crate::{Geometry, Vector};

impl Geometry {
    /// Adds a polygon.
    ///
    /// All vertices must lay in the same plane otherwise behavior may be unpredictable.
    /// The polygon is assumed to be convex. A non convex polygon will produce unpredictable behavior.
    /// Polygons with zero area will be ignored.
    ///
    /// Polygons cannot be added if already at the maximum number of polygons or if the addition of their verticies would result in exceeding the maximum number of vertices.
    ///
    /// Vertices of an object are in object space, not world space, and so are relative to the position, or center of the object.
    /// See [`Geometry::setP_psition`].
    pub fn add_polygon(
        &self,
        direct_occlusion: c_float,
        reverb_occlusion: c_float,
        double_sided: bool,
        vertices: &[Vector],
    ) -> Result<c_int> {
        let mut index = 0;
        unsafe {
            FMOD_Geometry_AddPolygon(
                self.inner,
                direct_occlusion,
                reverb_occlusion,
                double_sided.into(),
                vertices.len() as c_int,
                vertices.as_ptr().cast(),
                &mut index,
            )
            .to_result()?;
        }
        Ok(index)
    }

    /// Sets whether an object is processed by the geometry engine.
    pub fn set_active(&self, active: bool) -> Result<()> {
        unsafe { FMOD_Geometry_SetActive(self.inner, active.into()).to_result() }
    }

    /// Retrieves whether an object is processed by the geometry engine.
    pub fn get_active(&self) -> Result<bool> {
        let mut active = FMOD_BOOL::FALSE;
        unsafe {
            FMOD_Geometry_GetActive(self.inner, &mut active).to_result()?;
        }
        Ok(active.into())
    }

    /// Retrieves the maximum number of polygons and vertices allocatable for this object.
    ///
    /// The maximum number was set with [`crate::System::create_geometry`].
    pub fn get_max_polygons(&self) -> Result<(c_int, c_int)> {
        let mut max_polygons = 0;
        let mut max_vertices = 0;
        unsafe {
            FMOD_Geometry_GetMaxPolygons(self.inner, &mut max_polygons, &mut max_vertices)
                .to_result()?;
        }
        Ok((max_polygons, max_vertices))
    }

    /// Retrieves the number of polygons in this object.
    pub fn get_polygon_count(&self) -> Result<c_int> {
        let mut count = 0;
        unsafe {
            FMOD_Geometry_GetNumPolygons(self.inner, &mut count).to_result()?;
        }
        Ok(count)
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)] // fmod doesn't dereference the passed in pointer, and the user dereferencing it is unsafe anyway
    pub fn set_raw_userdata(&self, userdata: *mut c_void) -> Result<()> {
        unsafe { FMOD_Geometry_SetUserData(self.inner, userdata).to_result() }
    }

    pub fn get_raw_userdata(&self) -> Result<*mut c_void> {
        let mut userdata = std::ptr::null_mut();
        unsafe {
            FMOD_Geometry_GetUserData(self.inner, &mut userdata).to_result()?;
        }
        Ok(userdata)
    }

    /// Frees a geometry object and releases its memory.
    pub fn release(&self) -> Result<()> {
        // release userdata
        #[cfg(feature = "userdata-abstraction")]
        let userdata = self.get_raw_userdata()?;

        unsafe {
            FMOD_Geometry_Release(self.inner).to_result()?;
        }

        // release/remove userdata if it is not null
        #[cfg(feature = "userdata-abstraction")]
        if !userdata.is_null() {
            crate::userdata::remove_userdata(userdata.into());
            self.set_raw_userdata(std::ptr::null_mut())?;
        }

        Ok(())
    }

    /// Saves the geometry object as a serialized binary block to a [`Vec`].
    ///
    /// The data can be saved to a file if required and loaded later with [`crate::System::load_geometry`].
    pub fn save(&self) -> Result<Vec<u8>> {
        let mut data_size = 0;
        unsafe {
            FMOD_Geometry_Save(self.inner, std::ptr::null_mut(), &mut data_size).to_result()?;
        }

        let mut data = vec![0; data_size as usize];
        unsafe {
            FMOD_Geometry_Save(self.inner, data.as_mut_ptr().cast(), &mut data_size).to_result()?;
        }

        Ok(data)
    }
}

#[cfg(feature = "userdata-abstraction")]
impl Geometry {
    pub fn set_userdata(&self, userdata: crate::userdata::Userdata) -> Result<()> {
        use crate::userdata::{insert_userdata, set_userdata};

        let pointer = self.get_raw_userdata()?;
        if pointer.is_null() {
            let key = insert_userdata(userdata, *self);
            self.set_raw_userdata(key.into())?;
        } else {
            set_userdata(pointer.into(), userdata);
        }

        Ok(())
    }

    pub fn get_userdata(&self) -> Result<Option<crate::userdata::Userdata>> {
        use crate::userdata::get_userdata;

        let pointer = self.get_raw_userdata()?;
        Ok(get_userdata(pointer.into()))
    }
}
