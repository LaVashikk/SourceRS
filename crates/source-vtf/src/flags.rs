use bitflags::bitflags;

bitflags! {
    /// Flags stored in the VTF header.
    ///
    /// Several bits were reassigned by later Source branches. Version-qualified
    /// aliases make those collisions explicit instead of pretending that one
    /// interpretation is universally correct.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TextureFlags: u32 {
        const POINT_SAMPLE = 1 << 0;
        const TRILINEAR = 1 << 1;
        const CLAMP_S = 1 << 2;
        const CLAMP_T = 1 << 3;
        const ANISOTROPIC = 1 << 4;
        const VTEX_HINT_DXT5 = 1 << 5;
        const VTEX_NO_COMPRESS_V7_0 = 1 << 6;
        const SRGB_V7_4 = 1 << 6;
        const PWL_CORRECTED_V7_5 = 1 << 6;
        const NORMAL = 1 << 7;
        const NO_MIP = 1 << 8;
        const NO_LOD = 1 << 9;
        const LOAD_SMALL_MIPS_V7_0 = 1 << 10;
        const LOAD_ALL_MIPS_V7_3 = 1 << 10;
        const PROCEDURAL = 1 << 11;
        const ONE_BIT_ALPHA = 1 << 12;
        const MULTI_BIT_ALPHA = 1 << 13;
        const ENVMAP = 1 << 14;
        const RENDER_TARGET = 1 << 15;
        const DEPTH_RENDER_TARGET = 1 << 16;
        const NO_DEBUG_OVERRIDE = 1 << 17;
        const SINGLE_COPY = 1 << 18;
        const VTEX_ONE_OVER_MIP_LEVEL_IN_ALPHA = 1 << 19;
        const TF2_STAGING_MEMORY_V7_4 = 1 << 19;
        const SRGB_V7_5 = 1 << 19;
        const VTEX_PREMULTIPLY_COLOR_BY_ONE_OVER_MIP_LEVEL = 1 << 20;
        const TF2_IMMEDIATE_CLEANUP_V7_4 = 1 << 20;
        const DEFAULT_POOL_V7_5 = 1 << 20;
        const VTEX_CONVERT_NORMAL_TO_DUDV = 1 << 21;
        const TF2_IGNORE_PICMIP_V7_4 = 1 << 21;
        const CSGO_COMBINED_V7_5 = 1 << 21;
        const VTEX_ALPHA_TEST_MIP_GENERATION = 1 << 22;
        const CSGO_ASYNC_DOWNLOAD_V7_5 = 1 << 22;
        const NO_DEPTH_BUFFER = 1 << 23;
        const VTEX_NICE_FILTERED = 1 << 24;
        const CSGO_SKIP_INITIAL_DOWNLOAD_V7_5 = 1 << 24;
        const CLAMP_U = 1 << 25;
        const VERTEX_TEXTURE_V7_3 = 1 << 26;
        const SSBUMP_V7_3 = 1 << 27;
        const LOAD_MOST_MIPS_V7_5 = 1 << 28;
        const BORDER = 1 << 29;
        const TF2_STREAMABLE_COARSE_V7_4 = 1 << 30;
        const CSGO_YCOCG_V7_5 = 1 << 30;
        const TF2_STREAMABLE_FINE_V7_4 = 1 << 31;
        const CSGO_ASYNC_SKIP_INITIAL_LOW_RES_V7_5 = 1 << 31;
    }
}
