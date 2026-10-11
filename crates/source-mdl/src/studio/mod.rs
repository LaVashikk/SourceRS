mod geometry;
mod validate;

use std::fs;
use std::path::{Path, PathBuf};

use crate::mdl::material_path;
use crate::{Error, Mdl, Result, Vtx, Vvd};

pub use geometry::{Geometry, GeometryMesh, GeometryOptions};

/// VTX companion extensions in the order the engine prefers them
pub(crate) const VTX_EXTENSIONS: [&str; 4] = ["dx90.vtx", "vtx", "dx80.vtx", "sw.vtx"];

/// The VMT a mesh uses: an index into [`Mdl::materials`] and the paths to try, in order
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialReference {
    pub index: usize,
    /// Relative to `materials/`, without the `.vmt` extension
    pub path_candidates: Vec<String>,
}

/// MDL, VVD, and VTX of one model, validated against each other
#[derive(Debug, Clone, PartialEq)]
pub struct StudioModel {
    // todo: okay, maybe should make theese field public... just do any shit because we =can
    mdl: Mdl,
    vvd: Vvd,
    vtx: Vtx,
}

impl StudioModel {
    pub fn from_bytes(mdl: &[u8], vvd: &[u8], vtx: &[u8]) -> Result<Self> {
        let mdl = Mdl::from_bytes(mdl)?;
        let vvd = Vvd::from_bytes(vvd)?;
        let vtx = Vtx::from_bytes(vtx, mdl.header.version)?;
        validate::validate(&mdl, &vvd, &vtx)?;
        Ok(Self { mdl, vvd, vtx })
    }

    /// Reads an `.mdl` and the `.vvd` and `.vtx` next to it.
    /// Companion names are matched ignoring ASCII case, models made on Windows often ship them with different casing
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mdl = path.as_ref();
        let companion = |extension: &str| sibling(&mdl.with_extension(extension));
        let vvd = companion("vvd").unwrap_or_else(|| mdl.with_extension("vvd"));
        let vtx = VTX_EXTENSIONS
            .iter()
            .find_map(|extension| companion(extension))
            .unwrap_or_else(|| mdl.with_extension(VTX_EXTENSIONS[0]));

        let read = |path: &Path| fs::read(path).map_err(|source| Error::io(path, source));
        Self::from_bytes(&read(mdl)?, &read(&vvd)?, &read(&vtx)?)
    }

    pub const fn mdl(&self) -> &Mdl {
        &self.mdl
    }

    pub const fn vvd(&self) -> &Vvd {
        &self.vvd
    }

    pub const fn vtx(&self) -> &Vtx {
        &self.vtx
    }

    /// Materials of the selected meshes, in the order [`Self::geometry`] returns the meshes
    pub fn material_references(&self, options: &GeometryOptions) -> Result<Vec<MaterialReference>> {
        let mut references = Vec::new();
        for selected in self.selection(options)? {
            let model = &self.mdl.body_parts[selected.body_part].models[selected.model];
            for mesh in &model.meshes {
                references.push(self.material_reference(options.lod, options.skin, mesh.material_slot)?);
            }
        }

        Ok(references)
    }

    fn material_reference(&self, lod: usize, skin: usize, slot: usize) -> Result<MaterialReference> {
        let index = self.mdl.material_index(skin, slot)?;
        let path_candidates = match self.vtx.replacement(lod, index) {
            Some(name) if !name.is_empty() => vec![material_path("", name)],
            _ => self.mdl.material_path_candidates(index)?,
        };

        Ok(MaterialReference { index, path_candidates })
    }
}

/// `path` if it exists, otherwise a file next to it whose name differs only in ASCII case
fn sibling(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }

    // Windows modders don't care about case, Linux does. again do some shit. i hate shit.
    let name = path.file_name()?;
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|candidate| {
            candidate
                .file_name()
                .is_some_and(|file| file.eq_ignore_ascii_case(name))
        })
}
