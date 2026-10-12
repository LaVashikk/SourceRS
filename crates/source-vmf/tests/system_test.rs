#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use pretty_assertions::assert_eq;
    use source_vmf::prelude::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("vmf_examples/instances/maps/root.vmf")
    }

    /// Index of the loaded file whose path ends with `suffix`
    fn file(map: &VmfSystem, suffix: &str) -> usize {
        map.files
            .iter()
            .position(|vmf| vmf.path.as_deref().unwrap().ends_with(suffix))
            .unwrap()
    }

    /// Indices of the instances that place the file whose path ends with `suffix`
    fn placements(map: &VmfSystem, suffix: &str) -> Vec<usize> {
        let file = file(map, suffix);
        (0..map.instances.len())
            .filter(|&i| map.instances[i].file == file)
            .collect()
    }

    /// Targetname of the `func_instance` that placed `instance`
    fn placed_by(map: &VmfSystem, instance: usize) -> &str {
        let (parent, id) = map.instances[instance].parent.unwrap();
        let holder = &map.files[map.instances[parent].file];
        holder.entity(id).unwrap().targetname().unwrap()
    }

    fn key<'a>(ent: &'a Entity, key: &str) -> &'a str {
        ent.get(key).unwrap()
    }

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let path = entry.unwrap().path();
            let target = to.join(path.file_name().unwrap());
            if path.is_dir() {
                copy_dir(&path, &target);
            } else {
                std::fs::copy(&path, &target).unwrap();
            }
        }
    }

    #[test]
    fn each_file_is_loaded_once() {
        let map = VmfSystem::open(root()).unwrap();
        assert_eq!(map.files.len(), 4);
        assert_eq!(map.instances.len(), 6);
        assert_eq!(placements(&map, "room.vmf").len(), 2);
        assert_eq!(placements(&map, "lamp.vmf").len(), 2);
    }

    #[test]
    fn nested_instances_know_their_func_instances() {
        let map = VmfSystem::open(root()).unwrap();
        let chains: Vec<Vec<&str>> = placements(&map, "lamp.vmf")
            .into_iter()
            .map(|lamp| {
                map.chain(lamp)
                    .take_while(|&i| i != 0)
                    .map(|i| placed_by(&map, i))
                    .collect()
            })
            .collect();
        assert_eq!(chains, [["lamp", "room_a"], ["lamp", "room_b"]]);
    }

    #[test]
    fn transforms_compose_through_parents() {
        let map = VmfSystem::open(root()).unwrap();
        let world: Vec<_> = placements(&map, "lamp.vmf")
            .into_iter()
            .map(|i| {
                let t = map.instances[i].transform;
                (t.point([1.0, 0.0, 0.0]), t.angles([-45.0, 0.0, 0.0]))
            })
            .collect();
        // room_a: +100 on x, room_b: yaw 90, lamp inside room: +10 on x
        assert_eq!(
            world,
            [
                ([111.0, 0.0, 0.0], [-45.0, 0.0, 0.0]),
                ([0.0, 11.0, 0.0], [-45.0, 90.0, 0.0]),
            ]
        );
    }

    #[test]
    fn hidden_func_instances_are_not_expanded() {
        let map = VmfSystem::open(root()).unwrap();
        let tower = placements(&map, "tower.vmf");
        assert_eq!(tower.len(), 1);
        assert_eq!(
            map.instances[tower[0]].transform.point([0.0, 32.0, 0.0]),
            [0.0, -32.0, 500.0]
        );
    }

    #[test]
    fn missing_files_and_cycles_are_warnings() {
        let map = VmfSystem::open(root()).unwrap();
        assert_eq!(map.warnings.len(), 3);
        assert!(map.warnings[0].ends_with("instance 'instances/missing.vmf' not found"));
        // lamp.vmf places room.vmf again, once per placement of lamp.vmf
        assert!(
            map.warnings[1..]
                .iter()
                .all(|w| w.ends_with("instance 'instances/room.vmf' includes itself"))
        );
    }

    #[test]
    fn flatten_puts_instance_entities_into_world_space() {
        let map = VmfSystem::open(root()).unwrap();
        let flat = map.flatten();
        let ents = &flat.vmf.entities;

        let classes: Vec<_> = ents.iter().map(|ent| ent.classname().unwrap()).collect();
        assert_eq!(classes, ["light", "light", "light_spot", "light_spot"]);
        assert!(flat.vmf.hiddens.is_empty());
        assert!(flat.vmf.path.is_none());

        assert_eq!(key(&ents[0], "origin"), "0 0 64");
        assert_eq!(key(&ents[1], "origin"), "0 -32 500");
        // room_a only moves the lamp, so the spot keeps its angles string as written
        assert_eq!(
            [key(&ents[2], "origin"), key(&ents[2], "angles")],
            ["111 0 0", "-45.112 0 0"]
        );
        assert_eq!(
            [key(&ents[3], "origin"), key(&ents[3], "angles")],
            ["0 11 0", "-45.112 90 0"]
        );
    }

    #[test]
    fn flatten_remembers_where_each_entity_came_from() {
        let map = VmfSystem::open(root()).unwrap();
        let flat = map.flatten();
        assert_eq!(flat.sources.len(), flat.vmf.entities.len());
        for (ent, &(inst, id)) in flat.vmf.entities.iter().zip(&flat.sources) {
            let source = map.files[map.instances[inst].file].entity(id).unwrap();
            assert_eq!(ent.classname(), source.classname());
            assert_eq!(ent.id(), source.id());
        }
        let (tower_light, _) = flat.sources[1];
        assert_eq!(map.instances[tower_light].file, file(&map, "tower.vmf"));
    }

    #[test]
    fn flatten_moves_brushes_and_their_textures() {
        let map = VmfSystem::open(root()).unwrap();
        let flat = map.flatten();
        // the only brush is in tower.vmf, placed at 0 0 500 with yaw 180
        assert_eq!(flat.vmf.world.solids.len(), 1);
        let sides = &flat.vmf.world.solids[0].sides;
        assert_eq!(sides[0].plane, "(0 -64 564) (-64 -64 564) (-64 0 564)");
        assert_eq!(sides[0].u_axis, "[-1 0 0 0] 0.25");
        // the brush went 500 up and this axis points down, so the shift grows by 500 / 0.25 = 2000 texels
        assert_eq!(sides[2].v_axis, "[0 0 -1 2000] 0.25");
    }

    #[test]
    fn save_writes_each_file_to_its_own_path() {
        let dir = std::env::temp_dir().join(format!("source-vmf-system-{}", std::process::id()));
        copy_dir(root().parent().unwrap(), &dir.join("maps"));

        let mut map = VmfSystem::open(dir.join("maps/root.vmf")).unwrap();
        let tower = file(&map, "tower.vmf");
        for (file, _, ent) in map.iter_entities_mut() {
            if file == tower && ent.classname() == Some("light") {
                ent.set("_light".to_string(), "255 0 0 200".to_string());
            }
        }
        map.save().unwrap();

        let saved = VmfFile::open(dir.join("maps/instances/tower.vmf")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            saved.entities[0].get("_light").map(String::as_str),
            Some("255 0 0 200")
        );
    }
}
