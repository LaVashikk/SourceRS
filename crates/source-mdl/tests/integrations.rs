#![cfg(feature = "model_system")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::studio_fixture;
use source_fs::providers::SimpleGameInfo;
use source_fs::{DummyVpk, FileSystem, FileSystemOptions};
use source_mdl::{Error, GeometryOptions, MaterialReference, ModelSystem};
use tempfile::tempdir;

#[test]
fn loads_companions_and_caches_models_by_path() {
    let directory = tempdir().unwrap();
    let game = create_game(directory.path());
    write_model(&game, &["mdl", "vvd", "dx90.vtx"]);

    let mut models = ModelSystem::new(load_fs(&game));
    let model = models.get_model("test/sample").unwrap();

    assert_eq!(model.mdl().bones[0].name, "root");
    assert!(Arc::ptr_eq(
        &model,
        &models
            .get_model("Models\\Test\\Sample.MDL")
            .unwrap()
    ));
}

#[test]
fn reports_the_missing_companion_path() {
    let directory = tempdir().unwrap();
    let game = create_game(directory.path());
    write_model(&game, &["mdl"]);

    match ModelSystem::new(load_fs(&game)).get_model("test/sample") {
        Err(Error::FileSystem(source_fs::Error::NotFound { path, .. })) => {
            assert_eq!(path, "models/test/sample.vvd");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn resolves_lod_replacement_materials() {
    let directory = tempdir().unwrap();
    let game = create_game(directory.path());
    write_model(&game, &["mdl", "vvd", "vtx"]);
    write_material(&game, "replacement/lod1", r#"UnlitGeneric { "$basetexture" "debug/white" }"#);

    let mut models = ModelSystem::new(load_fs(&game));
    let model = models
        .get_model("models/test/sample.mdl")
        .unwrap();
    let geometry = model
        .geometry(&GeometryOptions { lod: 1, skin: 1, ..Default::default() })
        .unwrap();
    let material = models
        .get_material(&geometry.meshes[0].material)
        .unwrap();

    assert_eq!(material.get_string("$basetexture").as_deref(), Some("debug/white"));
}

#[test]
fn takes_the_first_cdmaterials_directory_that_has_the_material() {
    let directory = tempdir().unwrap();
    let game = create_game(directory.path());
    write_material(&game, "found/a", r#"UnlitGeneric { "$basetexture" "found" }"#);
    write_material(&game, "broken/a", "UnlitGeneric {");

    let mut models = ModelSystem::new(load_fs(&game));
    let found = models
        .get_material(&reference(&["missing/a", "found/a", "broken/a"]))
        .unwrap();
    assert_eq!(found.get_string("$basetexture").as_deref(), Some("found"));

    // A broken VMT in an earlier dir is an error, not something to quietly skip past
    assert!(matches!(
        models.get_material(&reference(&["broken/a", "found/a"])).unwrap_err(),
        Error::Material { path, .. } if path == "broken/a"
    ));
}

fn reference(candidates: &[&str]) -> MaterialReference {
    MaterialReference {
        index: 0,
        path_candidates: candidates
            .iter()
            .map(|candidate| candidate.to_string())
            .collect(),
    }
}

fn write_model(game: &Path, extensions: &[&str]) {
    let fixture = studio_fixture(44);
    let models = game.join("models/test");
    fs::create_dir_all(&models).unwrap();
    for extension in extensions {
        let bytes = match *extension {
            "mdl" => &fixture.mdl,
            "vvd" => &fixture.vvd,
            _ => &fixture.vtx,
        };
        fs::write(models.join(format!("sample.{extension}")), bytes).unwrap();
    }
}

fn write_material(game: &Path, name: &str, vmt: &str) {
    let path = game.join("materials").join(format!("{name}.vmt"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vmt).unwrap();
}

fn load_fs(game: &Path) -> FileSystem<DummyVpk> {
    FileSystem::load_from_path::<SimpleGameInfo>(game, &FileSystemOptions::default()).unwrap()
}

fn create_game(root: &Path) -> PathBuf {
    let game = root.join("game");
    fs::create_dir_all(&game).unwrap();
    fs::write(
        game.join("gameinfo.txt"),
        r#"GameInfo
{
    FileSystem
    {
        SearchPaths
        {
            Game "|gameinfo_path|."
        }
    }
}
"#,
    )
    .unwrap();
    game
}
