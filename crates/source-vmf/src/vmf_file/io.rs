use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    str::FromStr,
};

use crate::{VmfError, VmfResult, parser};

use super::VmfFile;

impl VmfFile {
    /// Parses a VMF file from a string.
    ///
    /// # Arguments
    ///
    /// * `content` - The string content of the VMF file.
    ///
    /// # Returns
    ///
    /// A `VmfResult` containing the parsed `VmfFile` or a `VmfError` if parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use vmf_forge::VmfFile;
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

    /// Parses a VMF file from a `File`.
    ///
    /// # Arguments
    ///
    /// * `file` - The `File` to read from.
    ///
    /// # Returns
    ///
    /// A `VmfResult` containing the parsed `VmfFile` or a `VmfError` if parsing fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use vmf_forge::VmfFile;
    /// use std::fs::File;
    ///
    /// let mut file = File::open("your_map.vmf").unwrap();
    /// let vmf_file = VmfFile::parse_file(&mut file);
    /// assert!(vmf_file.is_ok());
    /// ```
    pub fn parse_file(file: &mut impl Read) -> VmfResult<Self> {
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        let content = String::from_utf8_lossy(&content);

        VmfFile::parse(&content)
    }

    /// Opens and parses a VMF file from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the VMF file.
    ///
    /// # Returns
    ///
    /// A `VmfResult` containing the parsed `VmfFile` or a `VmfError` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use vmf_forge::VmfFile;
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
    /// # Arguments
    ///
    /// * `path` - The path to save the VMF file to.
    ///
    /// # Returns
    ///
    /// A `VmfResult` indicating success or a `VmfError` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use vmf_forge::VmfFile;
    ///
    /// let vmf_file = VmfFile::open("your_map.vmf").unwrap();
    /// let result = vmf_file.save("new_map.vmf");
    /// assert!(result.is_ok());
    /// ```
    pub fn save(&self, path: impl AsRef<Path>) -> VmfResult<()> {
        let mut file = File::create(path)?;
        file.write_all(self.to_vmf_string().as_bytes())?;
        Ok(())
    }
}

impl FromStr for VmfFile {
    type Err = VmfError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        VmfFile::parse(s)
    }
}
