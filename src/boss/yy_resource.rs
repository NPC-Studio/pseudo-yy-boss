use crate::{Resource, YyResourceHandler, YypBoss};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};
use yy_typings::ViewPath;

pub trait YyResource: Serialize + for<'de> Deserialize<'de> + Clone + Default {
    type AssociatedData: Debug;
    const SUBPATH_NAME: &'static str;
    const RESOURCE: Resource;

    /// Get's the resource's name.
    fn name(&self) -> &str;

    /// Sets the name of the resource.
    fn set_name(&mut self, name: String);

    /// Get the path to the parent in the View Virtual File System.
    fn parent_path(&self) -> ViewPath;

    fn get_handler(yyp_boss: &mut YypBoss) -> &mut YyResourceHandler<Self>;

    /// Deserialized the associated data with a given Yy File. In a sprite, for example,
    /// this would load the `pngs` into memory.
    ///
    /// The exact interpretation of what the `File` in `SerializedData` is left up to
    /// each individual implementation.
    fn deserialize_associated_data(
        &self,
        directory_path: Option<&Path>,
        data: SerializedData,
    ) -> anyhow::Result<Self::AssociatedData>;

    /// Serialize the associated data with a given Yy File.
    ///
    /// In a sprite, for example, this would serialize the `png` files,
    /// or in a script, this would serialize the associated `gml` files.
    fn serialize_associated_data(
        &self,
        directory_path: &Path,
        data: &Self::AssociatedData,
    ) -> anyhow::Result<()>;

    /// Converts associated data into `SerializedData`.
    ///
    /// This function will largely be called by the CLI, rather than directly by the YypBoss.
    /// Most resources will immediately return their data by value, but some resources, such
    /// as sprites or sounds, will likely write their files and return the path to the written
    /// audio instead.
    fn serialize_associated_data_into_data(
        &self,
        our_directory: &Path,
        working_directory: Option<&Path>,
        associated_data: Option<&Self::AssociatedData>,
    ) -> Result<SerializedData, SerializedDataError>;

    /// This cleans up any associated files which won't get cleaned up in the event of a
    /// REPLACEMENT of this resource. For example, when we replace a sprite_yy file, the old
    /// `png` files might not be replaced (because they are based on Uuids which will change).
    ///
    /// This functions is used to clean up those files. All of the paths are relative to the directory
    /// of the yy file.
    ///
    /// This function is ONLY called when a resource is being replaced. When a resource is being removed
    /// outright, then the entire folder is removed, so we don't need to carefully handle this.
    fn cleanup_on_replace(
        &self,
        files_to_delete: &mut Vec<PathBuf>,
        folders_to_delete: &mut Vec<PathBuf>,
    );
}

/// The data which is passed in as part of a Command. Each tag represents a different way to
/// pass data into the given Resource.
///
/// **NB:** the type of data which is passed in is determined by the containing Command. In a `ResourceCommand`,
/// for example, it is determined by the `Resource` which is passed in; for the `VirtualFileSystemCommand`, it is
/// determined by the `FileOrFolder` tag. See each documentation for more details.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "dataType")]
pub enum SerializedData {
    /// The data itself, represented in some valid utf8 format. Scripts, yyfiles, and most resources
    /// will likely be passed in with this tag.
    ///
    /// ## Errors
    /// It is an error to try to pass in any binary data which cannot be represented in utf8 format.
    /// To pass in images and other similar files, prefer using `Filepath`.
    Value { data: String },

    /// A path to the data iself, from some relevant directory. Symbolic links will not be followed.
    ///
    /// Anything, including gml and yy files, can be passed in with this tag, though its use is primarily
    /// for binary files, such as images and sound files.
    Filepath { data: PathBuf },

    /// A default for the type of data provided, which the YypBoss will generate for users.
    ///
    /// For example, to create a new Script, a user would want the Script's AssociatedData, which is the gml
    /// file, to be blank. This will generate such an empty string.
    /// In a more complex example, if a user is making a new Object, and is fine with default settings,
    /// included an autogenerated name, this tag will do that. Since all data can be edited afterwards,
    /// this can provide a convenient way to generate new objects.
    DefaultValue,
}

#[derive(Debug, thiserror::Error)]
pub enum SerializedDataError {
    #[error(
        "either given or tried to write a `Data::File` tag, but was not given a working directory on startup. cannot parse"
    )]
    NoFileMode,

    #[error("given a `Data::File` tag, but path didn't exist, wasn't a file, or couldn't be read. path was {}", .0.to_string_lossy())]
    BadDataFile(std::path::PathBuf),

    #[error(transparent)]
    CouldNotParseData(#[from] serde_json::Error),

    #[error(transparent)]
    CouldNotWriteImage(#[from] image::ImageError),

    #[error(
        "cannot be represented with utf8 encoding; must use `Data::File` or `Data::DefaultValue`"
    )]
    CannotUseValue,
}

impl SerializedData {
    pub fn read_data_as_file<T>(
        self,
        working_directory: Option<&std::path::Path>,
    ) -> Result<T, SerializedDataError>
    where
        for<'de> T: serde::Deserialize<'de> + Default,
    {
        match self {
            SerializedData::Value { data } => {
                serde_json::from_str(&data).map_err(SerializedDataError::CouldNotParseData)
            }
            SerializedData::Filepath { data } => {
                if let Some(wd) = working_directory {
                    let path = wd.join(data);
                    std::fs::read_to_string(&path).map_or_else(
                        |_| Err(SerializedDataError::BadDataFile(path)),
                        |data| {
                            serde_json::from_str(&data)
                                .map_err(SerializedDataError::CouldNotParseData)
                        },
                    )
                } else {
                    Err(SerializedDataError::NoFileMode)
                }
            }
            SerializedData::DefaultValue => Ok(T::default()),
        }
    }
}
