//! Everything MDL, VVD and VTX have to agree on, checked once when a model loads.
//! `geometry()` indexes their data directly afterwards, so a check missing here turns into a panic there

use crate::cursor::index;
use crate::{Error, FileKind, Mdl, Result, Vtx, Vvd};

pub(super) fn validate(mdl: &Mdl, vvd: &Vvd, vtx: &Vtx) -> Result<()> {
    checksum(FileKind::Vvd, mdl.header.checksum, vvd.checksum)?;
    for weights in vvd
        .vertices
        .iter()
        .map(|vertex| &vertex.bone_weights)
    {
        for &bone in &weights.bones[..usize::from(weights.count)] {
            index(bone, mdl.bones.len(), "VVD vertex bone")?;
        }
    }

    checksum(FileKind::Vtx, mdl.header.checksum, vtx.checksum)?;
    same("body parts", mdl.body_parts.len(), vtx.body_parts.len())?;
    for (mdl_body, vtx_body) in mdl.body_parts.iter().zip(&vtx.body_parts) {
        same("models in body part", mdl_body.models.len(), vtx_body.models.len())?;
        for (mdl_model, vtx_model) in mdl_body.models.iter().zip(&vtx_body.models) {
            same("LODs in VTX model", vtx.lod_count, vtx_model.lods.len())?;
            for lod in &vtx_model.lods {
                same("meshes in model LOD", mdl_model.meshes.len(), lod.meshes.len())?;
            }
        }
    }

    for replacement in vtx.material_replacements.iter().flatten() {
        if replacement.material >= mdl.materials.len() {
            return Err(Error::index("VTX replacement material", replacement.material, mdl.materials.len()));
        }
    }

    lod_layout(mdl, vvd, vtx)
}

/// Models are laid out back to back in every LOD, mesh by mesh, over all body parts
fn lod_layout(mdl: &Mdl, vvd: &Vvd, vtx: &Vtx) -> Result<()> {
    same("VVD/VTX LOD count", vvd.lod_count, vtx.lod_count)?;
    for lod in 0..vtx.lod_count {
        let mut total = 0usize;
        for model in mdl
            .body_parts
            .iter()
            .flat_map(|body| &body.models)
        {
            let model_total = model
                .meshes
                .iter()
                .fold(0usize, |sum, mesh| sum.saturating_add(mesh.lod_vertex_counts[lod]));
            if lod == 0 {
                same("MDL model vertex count", model.vertex_count, model_total)?;
                same("MDL model vertex base", total, model.vertex_base)?;
            }
            total = total.saturating_add(model_total);
        }
        same("MDL/VVD LOD vertex count", vvd.lod_vertex_counts[lod], total)?;
    }

    Ok(())
}

fn checksum(file: FileKind, expected: u32, actual: u32) -> Result<()> {
    if expected != actual {
        return Err(Error::ChecksumMismatch { file, expected, actual });
    }
    Ok(())
}

fn same(context: &'static str, expected: usize, actual: usize) -> Result<()> {
    if expected != actual {
        return Err(Error::StructureMismatch { context, expected, actual });
    }
    Ok(())
}
