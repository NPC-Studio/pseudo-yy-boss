use super::{directory_manager::DirectoryManager, utils, FilesystemPath, YyResource};
use crate::{AssocDataLocation, YyResourceHandlerErrors};
use anyhow::Result as AnyResult;
use log::{error, info};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use yy_typings::utils::TrailingCommaUtility;

#[derive(Debug, Default)]
pub struct YyResourceHandler<T: YyResource> {
    resources: HashMap<String, YyResourceData<T>>,
    pub(crate) resources_to_reserialize: Vec<String>,
    pub(crate) associated_files_to_cleanup: Vec<PathBuf>,
    pub(crate) associated_folders_to_cleanup: Vec<PathBuf>,
    pub(crate) resources_to_remove: Vec<String>,
}

impl<T: YyResource> YyResourceHandler<T> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Adds a new sprite into the game. It requires a `CreatedResource`,
    /// which is created from the `YypBoss`, which guarantees that the resource
    /// has been created in the Yyp.
    ///
    /// This operation is used to `add` or to `replace` the resource. If it is used
    /// to replace a resource, the resource will be returned.
    pub(crate) fn set(
        &mut self,
        value: T,
        associated_data: T::AssociatedData,
    ) -> Option<YyResourceData<T>> {
        self.resources_to_reserialize.push(value.name().to_owned());
        let ret = self.insert_resource(value, Some(associated_data));

        if let Some(old) = &ret {
            old.yy_resource.cleanup_on_replace(
                &mut self.associated_files_to_cleanup,
                &mut self.associated_folders_to_cleanup,
            );
        }

        ret
    }

    /// Returns an immutable reference to a resource's data, if it exists.
    ///
    /// Since associated data is lazily loaded, and be unloaded at any time,
    /// there may not be any associated data returned. You can request that data to be
    /// loaded using [`load_resource_associated_data`].
    ///
    /// [`load_resource_associated_data`]: #method.load_resource_associated_data
    pub fn get(&self, name: &str) -> Option<&YyResourceData<T>> {
        self.resources.get(name)
    }

    /// Removes the resource out of the handler. If that resource was being used,
    /// then this will return that resource.
    pub(crate) fn remove(
        &mut self,
        value: &str,
        tcu: &TrailingCommaUtility,
    ) -> Option<(T, Option<T::AssociatedData>)> {
        let ret = self.resources.remove(value);
        if let Some(ret) = ret {
            self.resources_to_remove.push(value.to_owned());

            let (yy, mut assoc) = ret.into();

            // Try to load this guy up...
            if assoc.is_none() {
                let output = self
                    .load_resource_associated_data(yy.name(), &yy.relative_yy_directory(), tcu)
                    .map_err(|e| {
                        error!("Couldn't deserialize {}'s assoc data...{}", value, e);
                        e
                    })
                    .ok();

                assoc = output.cloned();
            }

            Some((yy, assoc))
        } else {
            None
        }
    }

    /// Loads in the associated data of a given resource name, if that resource exists and is managed.
    ///
    /// If that resource already has some associated data, it will be discarded, and the new data will be loaded.
    /// If the resource does not exist or is not of the type that this manager handles, an error will be
    /// returned.
    pub fn load_resource_associated_data(
        &mut self,
        resource_name: &str,
        path: &Path,
        tcu: &TrailingCommaUtility,
    ) -> Result<&T::AssociatedData, YyResourceHandlerErrors> {
        if let Some(resource) = self.resources.get_mut(resource_name) {
            let associated_data = resource
                .yy_resource
                .deserialize_associated_data(AssocDataLocation::Path(path), tcu)?;

            resource.associated_data = Some(associated_data);

            Ok(&resource.associated_data.as_ref().unwrap())
        } else {
            Err(YyResourceHandlerErrors::ResourceNotFound)
        }
    }

    /// Loads the resource in on startup. We don't track associated data by default,
    /// and we don't mark the resource as dirty.
    pub(crate) fn load_on_startup(&mut self, value: T) {
        self.insert_resource(value, None);
    }

    /// Writes all of the resources to disk, and cleans up excess files.
    pub(crate) fn serialize(&mut self, directory_manager: &DirectoryManager) -> AnyResult<()> {
        // Removes the resources!
        for resource_to_remove in self.resources_to_remove.drain(..) {
            let path = FilesystemPath::new_path(T::SUBPATH_NAME, &resource_to_remove);
            info!("removing resource {} at {:?}", resource_to_remove, path);
            let yy_path = directory_manager.resource_file(&path);
            fs::remove_dir_all(yy_path.parent().unwrap())?;
        }

        // Remove folders
        for folder in self.associated_folders_to_cleanup.drain(..) {
            let path = directory_manager
                .resource_file(Path::new(T::SUBPATH_NAME))
                .join(folder);
            info!("remove folder {:?}", path);
            fs::remove_dir_all(path)?;
        }

        // Remove files
        for file in self.associated_files_to_cleanup.drain(..) {
            let path = directory_manager
                .resource_file(Path::new(T::SUBPATH_NAME))
                .join(file);
            info!("removing path {:?}", path);
            fs::remove_file(path)?;
        }

        // Finally, reserialize resources
        for resource_to_reserialize in self.resources_to_reserialize.drain(..) {
            info!("reserializing {}", resource_to_reserialize);

            let resource = self
                .resources
                .get(&resource_to_reserialize)
                .expect("This should always be valid.");

            let yy_path = directory_manager.resource_file(
                &FilesystemPath::new(T::SUBPATH_NAME, resource.yy_resource.name()).path,
            );

            if let Some(parent_dir) = yy_path.parent() {
                fs::create_dir_all(parent_dir)?;
                if let Some(associated_data) = &resource.associated_data {
                    resource
                        .yy_resource
                        .serialize_associated_data(parent_dir, associated_data)?;
                }
            }

            utils::serialize_json(&yy_path, &resource.yy_resource)?;
        }

        Ok(())
    }

    /// Wrapper around inserting the resource into `self.resources`.
    pub(crate) fn insert_resource(
        &mut self,
        value: T,
        associated_data: Option<T::AssociatedData>,
    ) -> Option<YyResourceData<T>> {
        self.resources.insert(
            value.name().to_owned(),
            YyResourceData {
                yy_resource: value,
                associated_data,
            },
        )
    }
}

#[derive(Default)]
pub struct YyResourceData<T: YyResource> {
    pub yy_resource: T,
    pub associated_data: Option<T::AssociatedData>,
}

impl<T: YyResource> Into<(T, Option<T::AssociatedData>)> for YyResourceData<T> {
    fn into(self) -> (T, Option<T::AssociatedData>) {
        (self.yy_resource, self.associated_data)
    }
}

impl<T: YyResource + std::fmt::Debug> std::fmt::Debug for YyResourceData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} !!**ASSOCIATED DATA IS NOT PRINTED IN DEBUG OUTPUT**!!",
            self.yy_resource
        )
    }
}
