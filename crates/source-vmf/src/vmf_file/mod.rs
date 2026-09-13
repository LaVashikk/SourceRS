use crate::VmfSerializable;

use super::vmf::entities::Entities;
use super::vmf::metadata::{VersionInfo, ViewSettings, VisGroups};
use super::vmf::regions::{Cameras, Cordons};
use super::vmf::world::World;

mod io;
mod merge;
mod visgroup_ops;

/// Represents a parsed VMF file.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct VmfFile {
    /// The path to the VMF file, if known.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub path: Option<String>,
    /// The version info of the VMF file.
    pub versioninfo: VersionInfo,
    /// The visgroups in the VMF file.
    pub visgroups: VisGroups,
    /// The view settings in the VMF file.
    pub viewsettings: ViewSettings,
    /// The world data in the VMF file.
    pub world: World,
    /// The entities in the VMF file.
    pub entities: Entities,
    /// The hidden entities in the VMF file.
    pub hiddens: Entities,
    /// The camera data in the VMF file.
    pub cameras: Cameras,
    /// The cordon data in the VMF file.
    pub cordons: Cordons,
}

impl Default for VmfFile {
    fn default() -> Self {
        Self {
            path: None,
            versioninfo: Default::default(),
            visgroups: Default::default(),
            viewsettings: Default::default(),
            world: Default::default(),
            entities: Entities(Vec::with_capacity(128)),
            hiddens: Entities(Vec::with_capacity(16)),
            cameras: Default::default(),
            cordons: Default::default(),
        }
    }
}

impl VmfFile {
    /// Converts the `VmfFile` to a string in VMF format.
    ///
    /// # Returns
    ///
    /// A string representing the `VmfFile` in VMF format.
    pub fn to_vmf_string(&self) -> String {
        let mut output = String::new();

        // metadatas
        output.push_str(&self.versioninfo.to_vmf_string(0));
        output.push_str(&self.visgroups.to_vmf_string(0));
        output.push_str(&self.viewsettings.to_vmf_string(0));
        output.push_str(&self.world.to_vmf_string(0));

        // entities
        for entity in &*self.entities {
            output.push_str(&entity.to_vmf_string(0));
        }

        for entity in &*self.hiddens {
            output.push_str("hidden\n{\n");
            output.push_str(&entity.to_vmf_string(1));
            output.push_str("}\n");
        }

        // regions
        output.push_str(&self.cameras.to_vmf_string(0));
        output.push_str(&self.cordons.to_vmf_string(0));

        output
    }
}
