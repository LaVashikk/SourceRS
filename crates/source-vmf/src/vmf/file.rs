use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    str::FromStr,
};

use crate::{VmfBlock, VmfError, VmfResult, VmfSerializable, parser};
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use super::entities::{Entities, Entity};
use super::metadata::{VersionInfo, ViewSettings, VisGroups};
use super::regions::{Cameras, Cordons};
use super::world::{Solid, World};

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
    /// Top-level blocks this struct does not model, kept so they survive a
    /// parse/serialize round-trip.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub extra_blocks: Vec<VmfBlock>,
    /// Non-fatal warnings encountered during parsing (e.g. fallback defaults, unknown blocks).
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub warnings: Vec<String>,
    /// Number of warnings suppressed after exceeding the warning limit.
    #[cfg_attr(feature = "serialization", serde(default))]
    pub suppressed_warnings: usize,
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
            extra_blocks: Vec::new(),
            warnings: Vec::new(),
            suppressed_warnings: 0,
        }
    }
}

impl VmfFile {
    /// Converts the `VmfFile` to a string in VMF format.
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

        for block in &self.extra_blocks {
            output.push_str(&block.serialize(0));
        }

        output
    }

    /// Parses a VMF file from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use source_vmf::VmfFile;
    ///
    /// let vmf_content = r#"
    /// versioninfo
    /// {
    ///     "editorversion" "400"
    ///     "editorbuild" "8000"
    ///     "mapversion" "1"
    ///     "formatversion" "100"
    ///     "prefab" "0"
    /// }
    /// "#;
    ///
    /// let vmf_file = VmfFile::parse(vmf_content);
    /// assert!(vmf_file.is_ok());
    /// ```
    pub fn parse(content: &str) -> VmfResult<Self> {
        parser::parse_vmf(content)
    }

    /// Opens and parses a VMF file from a file path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use source_vmf::VmfFile;
    ///
    /// let vmf_file = VmfFile::open("your_map.vmf");
    /// assert!(vmf_file.is_ok());
    /// ```
    pub fn open(path: impl AsRef<Path>) -> VmfResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let mut file = File::open(path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        let content = String::from_utf8_lossy(&content);

        let mut vmf_file = VmfFile::parse(&content)?;
        vmf_file.path = Some(path_str);
        Ok(vmf_file)
    }

    /// Saves the `VmfFile` to a file at the specified path.
    ///
    /// Writes to a temporary `.part` file first and renames it into place.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use source_vmf::VmfFile;
    ///
    /// let vmf_file = VmfFile::open("your_map.vmf").unwrap();
    /// let result = vmf_file.save("new_map.vmf");
    /// assert!(result.is_ok());
    /// ```
    pub fn save(&self, path: impl AsRef<Path>) -> VmfResult<()> {
        let path = path.as_ref();

        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".part");
        let partial = path.with_file_name(name);

        if let Err(e) = self.write_to(&partial) {
            let _ = fs::remove_file(&partial);
            return Err(e.into());
        }
        fs::rename(&partial, path)?;
        Ok(())
    }

    fn write_to(&self, path: &Path) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.to_vmf_string().as_bytes())?;
        file.sync_all()
    }

    /// Merges the contents of another `VmfFile` into this one.
    ///
    /// Combines `visgroups`, `world` solids (visible and hidden), `entities`, `hiddens`,
    /// and `cordons`. `versioninfo`, `viewsettings`, and `cameras` are retained from `self`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use source_vmf::prelude::*;
    ///
    /// let mut vmf1 = VmfFile::open("map1.vmf").unwrap();
    /// let vmf2 = VmfFile::open("map2.vmf").unwrap();
    ///
    /// vmf1.merge(vmf2);
    /// ```
    pub fn merge(&mut self, other: Self) {
        self.visgroups.groups.extend(other.visgroups.groups);
        self.world.solids.extend(other.world.solids);
        self.world.hidden.extend(other.world.hidden);

        self.entities.extend(other.entities);
        self.hiddens.extend(other.hiddens);

        self.cordons.extend(other.cordons.cordons);
    }

    /// Returns an iterator over entities (including hidden ones) belonging to the specified VisGroup ID.
    pub fn get_entities_in_visgroup(
        &self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &Entity> + '_> {
        let ids = self.visgroups.resolve_ids(group_id, include_children)?;
        Some(
            self.iter_entities()
                .map(|(_, entity)| entity)
                .filter(move |entity| {
                    entity
                        .editor
                        .visgroup_ids
                        .iter()
                        .any(|ent_group_id| ids.contains(ent_group_id))
                }),
        )
    }

    /// Returns a mutable iterator over entities (including hidden ones) belonging to the specified VisGroup ID.
    pub fn get_entities_in_visgroup_mut(
        &mut self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &mut Entity> + '_> {
        let ids = self.visgroups.resolve_ids(group_id, include_children)?;
        Some(
            self.entities
                .iter_mut()
                .chain(self.hiddens.iter_mut())
                .filter(move |entity| {
                    entity
                        .editor
                        .visgroup_ids
                        .iter()
                        .any(|ent_group_id| ids.contains(ent_group_id))
                }),
        )
    }

    /// Returns an iterator over world solids (visible and hidden) belonging to the specified VisGroup ID.
    pub fn get_solids_in_visgroup(
        &self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &Solid> + '_> {
        let ids = self.visgroups.resolve_ids(group_id, include_children)?;
        Some(
            self.world
                .iter_solids()
                .filter(move |solid| {
                    solid
                        .editor
                        .visgroup_ids
                        .iter()
                        .any(|solid_group_id| ids.contains(solid_group_id))
                }),
        )
    }

    /// Returns a mutable iterator over world solids (visible and hidden) belonging to the specified VisGroup ID.
    pub fn get_solids_in_visgroup_mut(
        &mut self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &mut Solid> + '_> {
        let ids = self.visgroups.resolve_ids(group_id, include_children)?;
        Some(
            self.world
                .iter_solids_mut()
                .filter(move |solid| {
                    solid
                        .editor
                        .visgroup_ids
                        .iter()
                        .any(|solid_group_id| ids.contains(solid_group_id))
                }),
        )
    }
}

impl FromStr for VmfFile {
    type Err = VmfError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        VmfFile::parse(s)
    }
}
