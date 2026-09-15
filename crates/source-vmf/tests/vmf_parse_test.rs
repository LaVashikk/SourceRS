#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use source_vmf::parser::*;

    #[test]
    fn parse_vmf_valid_input() {
        let input = "\
        versioninfo\n\
        {\n\
        \t\"editorversion\" \"400\"\n\
        \t\"editorbuild\" \"8000\"\n\
        \t\"mapversion\" \"1\"\n\
        \t\"formatversion\" \"100\"\n\
        \t\"prefab\" \"0\"\n\
        }\n\
        world\n\
        {\n\
        \t\"classname\" \"worldspawn\"\n\
        }\n";
        let vmf = parse_vmf(input).unwrap();
        assert_eq!(vmf.versioninfo.editor_version, 400);
        assert_eq!(vmf.world.key_values.get("classname").unwrap(), "worldspawn");
    }
    #[test]
    fn parse_vmf_bad_value_warns_instead_of_failing() {
        let input = "\
        versioninfo\n\
        {\n\
        \t\"editorversion\" \"abc\"\n\
        \t\"editorbuild\" \"8000\"\n\
        \t\"mapversion\" \"1\"\n\
        \t\"formatversion\" \"100\"\n\
        \t\"prefab\" \"0\"\n\
        }\n\
        world\n\
        {\n\
        \t\"classname\" \"worldspawn\"\n\
        }\n";

        let vmf = parse_vmf(input).unwrap();
        assert_eq!(vmf.versioninfo.editor_version, 0);
        assert_eq!(vmf.versioninfo.editor_build, 8000);
        assert_eq!(vmf.world.key_values.get("classname").unwrap(), "worldspawn");
        assert_eq!(vmf.warnings.len(), 1);
        assert!(vmf.warnings[0].contains("unparsable value 'abc'"));
    }

    #[test]
    fn parse_vmf_broken_syntax_is_still_an_error() {
        assert!(parse_vmf("world\n{\n\t\"classname\" \"worldspawn\"\n").is_err());
    }

    #[test]
    fn parse_vmf_keeps_unknown_root_blocks() {
        let input = "\
        world\n\
        {\n\
        \t\"classname\" \"worldspawn\"\n\
        }\n\
        custom_tool_data\n\
        {\n\
        \t\"vendor\" \"acme\"\n\
        }\n";

        let vmf = parse_vmf(input).unwrap();
        assert_eq!(vmf.extra_blocks.len(), 1);
        assert_eq!(vmf.extra_blocks[0].name, "custom_tool_data");
        assert!(vmf.to_vmf_string().contains("custom_tool_data"));
        assert!(vmf.warnings.iter().any(|w| w.contains("custom_tool_data")));
    }

    #[test]
    fn parse_vmf_empty_input() {
        let input = "";
        let vmf = parse_vmf(input).unwrap();
        assert_eq!(vmf.entities.len(), 0);
        assert!(vmf.warnings.is_empty());
    }
}
