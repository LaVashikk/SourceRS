#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use source_vmf::vmf::connection::*;

    fn parse(value: &str) -> Connection {
        Connection::parse("OnTrigger".to_string(), value)
    }

    // --- parsing ---------------------------------------------------------

    #[test]
    fn esc_value_keeps_commas_in_params() {
        let c = parse("@player\x1bRunScriptCode\x1bSpawn(1, 2, 3)\x1b0.5\x1b-1");
        assert_eq!(c.target, "@player");
        assert_eq!(c.input, "RunScriptCode");
        assert_eq!(c.params, "Spawn(1, 2, 3)");
        assert_eq!(c.delay, 0.5);
        assert_eq!(c.fire_limit, -1);
    }

    #[test]
    fn comma_value_recovers_commas_in_params() {
        let c = parse("door,RunScriptCode,foo(1,2),0,-1");
        assert_eq!(c.target, "door");
        assert_eq!(c.input, "RunScriptCode");
        assert_eq!(c.params, "foo(1,2)");
        assert_eq!(c.delay, 0.0);
        assert_eq!(c.fire_limit, -1);
    }

    #[test]
    fn esc_wins_when_both_separators_are_present() {
        let c = parse("a\x1bb\x1bc,d,e\x1b1\x1b2");
        assert_eq!(c.params, "c,d,e");
        assert_eq!(c.delay, 1.0);
        assert_eq!(c.fire_limit, 2);
    }

    #[test]
    fn plain_connection() {
        let c = parse("my_door\x1bOpen\x1b\x1b0\x1b-1");
        assert_eq!(c.target, "my_door");
        assert_eq!(c.input, "Open");
        assert_eq!(c.params, "");
        assert_eq!(c.delay, 0.0);
        assert_eq!(c.fire_limit, -1);
    }

    #[test]
    fn truncated_values_fall_back_to_defaults() {
        assert_eq!(parse("").target, "");

        let two = parse("relay\x1bTrigger");
        assert_eq!((two.target.as_str(), two.input.as_str()), ("relay", "Trigger"));
        assert_eq!((two.params.as_str(), two.delay, two.fire_limit), ("", 0.0, -1));

        let three = parse("relay\x1bTrigger\x1bstuff");
        assert_eq!(three.params, "stuff");
        assert_eq!((three.delay, three.fire_limit), (0.0, -1));
    }

    #[test]
    fn four_fields_resolve_in_favour_of_truncation() {
        let c = parse("relay,Trigger,a,0");
        assert_eq!(c.params, "a,0");
        assert_eq!((c.delay, c.fire_limit), (0.0, -1));
    }

    // --- serialization ---------------------------------------------------

    #[test]
    fn always_serializes_with_esc() {
        let c = parse("door,Open,,0,-1");
        assert_eq!(c.to_value_string(), "door\x1bOpen\x1b\x1b0\x1b-1");
    }

    #[test]
    fn value_survives_a_round_trip() {
        for value in [
            "@player\x1bRunScriptCode\x1bSpawn(1, 2)\x1b0.5\x1b-1",
            "door,Open,,0,-1",
            "relay\x1bTrigger",
            "",
        ] {
            let once = parse(value);
            let twice = parse(&once.to_value_string());
            assert_eq!(once, twice, "value: {value:?}");
        }
    }

    // --- target classification -------------------------------------------

    #[test]
    fn target_kinds() {
        assert_eq!(parse("door\x1bOpen").target_kind(), Target::Name("door"));
        assert_eq!(
            parse("door_*\x1bOpen").target_kind(),
            Target::Wildcard("door_")
        );
        assert_eq!(parse("*\x1bOpen").target_kind(), Target::Wildcard(""));
        assert_eq!(
            parse("!activator\x1bOpen").target_kind(),
            Target::Special(Special::Activator)
        );
    }

    #[test]
    fn specials_are_case_insensitive_and_open_ended() {
        assert_eq!(Target::parse("!SELF"), Target::Special(Special::SelfEnt));
        assert_eq!(Target::parse("!PvsPlayer"), Target::Special(Special::PvsPlayer));
        assert_eq!(
            Target::parse("!speechtarget"),
            Target::Special(Special::Other("speechtarget"))
        );
    }

    #[test]
    fn target_name_accessor() {
        assert_eq!(Target::parse("door").name(), Some("door"));
        assert_eq!(Target::parse("door_*").name(), None);
        assert_eq!(Target::parse("!self").name(), None);
    }

    // --- instance io ------------------------------------------------------

    #[test]
    fn instance_io_parses_both_directions() {
        let mut c = parse("gate\x1binstance:proxy;Open\x1b\x1b0\x1b-1");
        assert_eq!(
            c.instance_input(),
            Some(InstanceIo {
                proxy: "proxy",
                io: "Open"
            })
        );
        assert_eq!(c.instance_output(), None);

        c.output = "instance:base;OnOpened".to_string();
        assert_eq!(
            c.instance_output(),
            Some(InstanceIo {
                proxy: "base",
                io: "OnOpened"
            })
        );
    }

    #[test]
    fn instance_io_rejects_plain_names_and_renders_back() {
        assert_eq!(InstanceIo::parse("Open"), None);
        assert_eq!(InstanceIo::parse("instance:no_semicolon"), None);
        assert_eq!(InstanceIo::parse("inst"), None); // shorter than the prefix
        assert_eq!(
            InstanceIo::parse("Instance:p;Io"),
            Some(InstanceIo {
                proxy: "p",
                io: "Io"
            })
        );
        assert_eq!(
            InstanceIo {
                proxy: "p",
                io: "Io"
            }
            .to_string(),
            "instance:p;Io"
        );
    }

    #[test]
    fn instance_io_setters() {
        let mut c = parse("gate\x1bOpen\x1b\x1b0\x1b-1");
        c.set_instance_input("proxy", "Open");
        c.set_instance_output("base", "OnUser1");
        assert_eq!(c.input, "instance:proxy;Open");
        assert_eq!(c.output, "instance:base;OnUser1");
        assert_eq!(c.instance_input().unwrap().proxy, "proxy");
    }
}

#[cfg(test)]
mod block_roundtrip {
    use pretty_assertions::assert_eq;
    use source_vmf::prelude::*;
    use source_vmf::{FromBlock, ParseCtx, VmfBlock};

    #[test]
    fn entity_survives_a_trip_through_vmfblock() {
        let mut ent = Entity::new("logic_relay", 7);
        ent.set("targetname".to_string(), "relay_a".to_string());
        ent.add_connection("OnTrigger", "door", "Open", "", 0.0, -1);
        ent.add_connection("OnTrigger", "light", "TurnOn", "", 0.5, 1);
        ent.add_connection("OnUser1", "@player", "RunScriptCode", "Spawn(1, 2)", 0.0, -1);

        let block: VmfBlock = ent.clone().into();
        let back = Entity::from_block(block, &mut ParseCtx::default());

        assert_eq!(back.connections, ent.connections);
        assert_eq!(back.key_values, ent.key_values);
    }

    #[test]
    fn connections_key_values_round_trip() {
        let mut ent = Entity::new("logic_relay", 1);
        ent.add_connection("OnTrigger", "a", "Open", "", 0.0, -1);
        ent.add_connection("OnTrigger", "b", "Close", "", 1.0, 2);
        let conns = ent.connections.clone().unwrap();

        let kvs = Connection::to_key_values(&conns);
        assert_eq!(kvs.len(), 1, "one entry per output name");
        assert_eq!(Connection::from_key_values(&kvs), Some(conns));
    }
}
