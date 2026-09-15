#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use source_vmf::prelude::*;

    fn named(classname: &str, name: &str, id: u64) -> Entity {
        let mut ent = Entity::new(classname, id);
        ent.set("targetname".to_string(), name.to_string());
        ent
    }

    fn fixture() -> VmfFile {
        let mut vmf = VmfFile::default();
        vmf.entities.push(named("prop_door", "door_a", 1));
        vmf.entities.push(named("prop_door", "door_a", 2));
        vmf.entities.push(named("logic_relay", "door_b", 3));
        vmf.entities.push(Entity::new("info_player_start", 4));
        vmf.hiddens.push(named("logic_auto", "hidden_relay", 5));
        vmf
    }

    #[test]
    fn addresses_visible_and_hidden_entities() {
        let vmf = fixture();
        let ids: Vec<_> = vmf.iter_entities().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), 5);
        assert_eq!(ids[4], EntId { hidden: true, idx: 0 });

        assert_eq!(vmf.entity(ids[0]).unwrap().id(), 1);
        assert_eq!(vmf.entity(ids[4]).unwrap().classname(), Some("logic_auto"));
        assert!(vmf.entity(EntId { hidden: false, idx: 99 }).is_none());
    }

    #[test]
    fn duplicate_names_all_resolve() {
        let index = EntityIndex::build(&fixture());
        let hits = index.by_name("door_a");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].idx, 0);
        assert_eq!(hits[1].idx, 1);
    }

    #[test]
    fn hidden_entities_are_indexed() {
        let index = EntityIndex::build(&fixture());
        assert_eq!(
            index.by_name("hidden_relay"),
            &[EntId { hidden: true, idx: 0 }]
        );
    }

    #[test]
    fn missing_name_is_empty_not_absent() {
        let index = EntityIndex::build(&fixture());
        assert!(index.by_name("nope").is_empty());
    }

    #[test]
    fn wildcard_prefix_matches_every_carrier() {
        let index = EntityIndex::build(&fixture());
        let mut hits: Vec<_> = index.by_prefix("door_").collect();
        hits.sort();
        assert_eq!(hits.len(), 3);

        assert_eq!(index.by_prefix("").count(), 4);
    }

    #[test]
    fn index_survives_mutation_of_the_file() {
        let mut vmf = fixture();
        let index = EntityIndex::build(&vmf);
        let id = index.by_name("door_b")[0];
        vmf.entity_mut(id)
            .unwrap()
            .set("classname".to_string(), "func_instance".to_string());
        assert_eq!(vmf.entity(id).unwrap().classname(), Some("func_instance"));
    }
}
