use super::VmfFile;

impl VmfFile {
    /// Merges the contents of another `VmfFile` into this one.
    ///
    /// This method combines the `visgroups`, `world` solids (both visible and hidden),
    /// `entities`, `hiddens`, and `cordons` from the `other` `VmfFile` into the
    /// current `VmfFile`.  `versioninfo`, `viewsettings`, and `cameras` are
    /// *not* merged; the original values in `self` are retained.
    ///
    /// This method is experimental and its behavior may change in future versions.
    /// It does not handle potential ID conflicts between the two VMF files.
    ///
    /// # Arguments
    ///
    /// * `other` - The `VmfFile` to merge into this one.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vmf_forge::prelude::*;
    ///
    /// let mut vmf1 = VmfFile::open("map1.vmf").unwrap();
    /// let vmf2 = VmfFile::open("map2.vmf").unwrap();
    ///
    /// vmf1.merge(vmf2);
    ///
    /// // vmf1 now contains the combined contents of both files.
    /// ```
    pub fn merge(&mut self, other: VmfFile) {
        self.visgroups.groups.extend(other.visgroups.groups);
        self.world.solids.extend(other.world.solids);
        self.world.hidden.extend(other.world.hidden);

        self.entities.extend(other.entities);
        self.hiddens.extend(other.hiddens);

        self.cordons.extend(other.cordons.cordons);
    }
}
