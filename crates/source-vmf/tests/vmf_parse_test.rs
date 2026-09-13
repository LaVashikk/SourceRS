#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use vmf_forge::errors::VmfError;
    use vmf_forge::parser::*;

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
    fn parse_vmf_invalid_input() {
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

        let result = parse_vmf(input);
        assert!(matches!(
            result,
            Err(VmfError::ParseInt { source: _, key: _ })
        ));
    }

    #[test]
    fn parse_vmf_empty_input() {
        let input = "";
        let vmf = parse_vmf(input).unwrap();
        assert_eq!(vmf.entities.len(), 0);
    }
}
