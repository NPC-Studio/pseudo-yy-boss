use super::{ResourceDescriptor, ResourceNames};
use crate::YyResource;
use serde::{Deserialize, Serialize};
use yy_typings::FilesystemPath;

#[derive(Serialize, Deserialize, Default, Debug, Eq, PartialEq, Clone, Hash, Ord, PartialOrd)]
pub struct Files(Vec<FilesystemPath>);

impl Files {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.0.iter().any(|f| f.name == *name)
    }

    pub fn load_in<T: YyResource>(&mut self, yy: &T, order: usize, rn: &mut ResourceNames) {
        self.0
            .push(FilesystemPath::new(T::SUBPATH_NAME, &yy.name()));

        // add to resource names...
        rn.load_in_resource(
            yy.name().to_string(),
            ResourceDescriptor::new(T::RESOURCE, order, yy.parent_view_path().path),
        );
    }

    pub fn add<T: YyResource>(&mut self, yy: &T, order: usize, rn: &mut ResourceNames) {
        self.attach(FilesystemPath::new(T::SUBPATH_NAME, &yy.name()));

        // add to resource names...
        rn.insert(
            yy.name().to_string(),
            ResourceDescriptor::new(T::RESOURCE, order, yy.parent_view_path().path),
        );
    }

    pub fn remove(&mut self, name: &str, rn: &mut ResourceNames) {
        self.detach(name);
        rn.remove(name);
    }

    pub fn attach(&mut self, fsyspath: FilesystemPath) {
        self.0.push(fsyspath);
    }

    pub fn detach(&mut self, name: &str) -> Option<FilesystemPath> {
        self.0
            .iter()
            .position(|v| v.name == name)
            .map(|p| self.0.remove(p))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
