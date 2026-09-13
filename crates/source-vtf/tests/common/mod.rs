#![allow(dead_code)]

use source_vtf::{ImageFormat, ResourceTag, TextureFlags};

#[derive(Debug, Clone)]
pub struct ExpectedImage {
    pub mip: u8,
    pub frame: u16,
    pub face: u8,
    pub slice: u16,
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BuiltFixture {
    pub bytes: Vec<u8>,
    pub images: Vec<ExpectedImage>,
    pub thumbnail: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct Fixture {
    pub minor: u32,
    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub frame_count: u16,
    pub start_frame: u16,
    pub mip_count: u8,
    pub flags: TextureFlags,
    pub format: ImageFormat,
    pub thumbnail: Option<(u8, u8)>,
    pub inline_resources: Vec<(ResourceTag, [u8; 4])>,
    pub data_resources: Vec<(ResourceTag, Vec<u8>)>,
}

impl Fixture {
    pub fn new(minor: u32, width: u16, height: u16, format: ImageFormat) -> Self {
        Self {
            minor,
            width,
            height,
            depth: 1,
            frame_count: 1,
            start_frame: 0,
            mip_count: 1,
            flags: TextureFlags::empty(),
            format,
            thumbnail: None,
            inline_resources: Vec::new(),
            data_resources: Vec::new(),
        }
    }

    pub fn build(&self) -> BuiltFixture {
        assert!(self.minor <= 5);
        assert!(self.width != 0 && self.height != 0 && self.depth != 0);
        assert!(self.frame_count != 0 && self.mip_count != 0);

        let face_count = self.face_count();
        let mut images = Vec::new();
        let mut image_data = Vec::new();
        let mut marker = 1u8;

        for mip in (0..self.mip_count).rev() {
            let width = mip_dimension(self.width, mip);
            let height = mip_dimension(self.height, mip);
            let depth = mip_dimension(self.depth, mip);
            let image_length = image_byte_len(self.format, width, height);
            for frame in 0..self.frame_count {
                for face in 0..face_count {
                    for slice in 0..depth {
                        let data = vec![marker; image_length];
                        image_data.extend_from_slice(&data);
                        images.push(ExpectedImage {
                            mip,
                            frame,
                            face,
                            slice,
                            width,
                            height,
                            data,
                        });
                        marker = marker.wrapping_add(1);
                    }
                }
            }
        }

        let thumbnail = self.thumbnail.map(|(width, height)| {
            vec![0xee; image_byte_len(ImageFormat::Dxt1, width.into(), height.into())]
        });
        let bytes = if self.minor < 3 {
            self.build_legacy(thumbnail.as_deref(), &image_data)
        } else {
            self.build_resource_vtf(thumbnail.as_deref(), &image_data)
        };

        BuiltFixture {
            bytes,
            images,
            thumbnail,
        }
    }

    fn build_legacy(&self, thumbnail: Option<&[u8]>, image_data: &[u8]) -> Vec<u8> {
        assert!(self.inline_resources.is_empty());
        assert!(self.data_resources.is_empty());
        let header_size = if self.minor < 2 { 64 } else { 80 };
        let mut output = self.base_header(header_size);
        if self.minor >= 2 {
            output.extend_from_slice(&self.depth.to_le_bytes());
        }
        output.resize(header_size as usize, 0);
        if let Some(thumbnail) = thumbnail {
            output.extend_from_slice(thumbnail);
        }
        output.extend_from_slice(image_data);
        output
    }

    fn build_resource_vtf(&self, thumbnail: Option<&[u8]>, image_data: &[u8]) -> Vec<u8> {
        enum Payload<'a> {
            Inline([u8; 4]),
            Data(&'a [u8]),
        }

        let mut entries = Vec::new();
        entries.push((ResourceTag::IMAGE_DATA, Payload::Data(image_data)));
        for &(tag, data) in &self.inline_resources {
            entries.push((tag, Payload::Inline(data)));
        }
        for (tag, data) in &self.data_resources {
            entries.push((*tag, Payload::Data(data)));
        }
        if let Some(thumbnail) = thumbnail {
            entries.push((ResourceTag::THUMBNAIL_DATA, Payload::Data(thumbnail)));
        }

        let header_size = 80 + entries.len() as u32 * 8;
        let mut output = self.base_header(header_size);
        output.extend_from_slice(&self.depth.to_le_bytes());
        output.extend_from_slice(&[0; 3]);
        output.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        output.extend_from_slice(&[0; 8]);
        output.resize(header_size as usize, 0);

        let mut offsets = vec![None; entries.len()];
        if thumbnail.is_some() {
            let index = entries.len() - 1;
            offsets[index] = Some(output.len() as u32);
            if let Payload::Data(data) = &entries[index].1 {
                output.extend_from_slice(data);
            }
        }
        for index in 1..entries
            .len()
            .saturating_sub(usize::from(thumbnail.is_some()))
        {
            if let Payload::Data(data) = &entries[index].1 {
                offsets[index] = Some(output.len() as u32);
                output.extend_from_slice(data);
            }
        }
        offsets[0] = Some(output.len() as u32);
        output.extend_from_slice(image_data);

        for (index, (tag, payload)) in entries.iter().enumerate() {
            let entry_offset = 80 + index * 8;
            let flags = match payload {
                Payload::Inline(_) => 1u32 << 25,
                Payload::Data(_) => 0,
            };
            let type_and_flags = tag.value() | flags;
            output[entry_offset..entry_offset + 4].copy_from_slice(&type_and_flags.to_le_bytes());
            match payload {
                Payload::Inline(data) => {
                    output[entry_offset + 4..entry_offset + 8].copy_from_slice(data);
                }
                Payload::Data(_) => {
                    output[entry_offset + 4..entry_offset + 8]
                        .copy_from_slice(&offsets[index].unwrap().to_le_bytes());
                }
            }
        }

        output
    }

    fn base_header(&self, header_size: u32) -> Vec<u8> {
        let mut output = Vec::with_capacity(header_size as usize);
        output.extend_from_slice(b"VTF\0");
        output.extend_from_slice(&7u32.to_le_bytes());
        output.extend_from_slice(&self.minor.to_le_bytes());
        output.extend_from_slice(&header_size.to_le_bytes());
        output.extend_from_slice(&self.width.to_le_bytes());
        output.extend_from_slice(&self.height.to_le_bytes());
        output.extend_from_slice(&self.flags.bits().to_le_bytes());
        output.extend_from_slice(&self.frame_count.to_le_bytes());
        output.extend_from_slice(&self.start_frame.to_le_bytes());
        output.extend_from_slice(&[0; 4]);
        output.extend_from_slice(&0.25f32.to_le_bytes());
        output.extend_from_slice(&0.5f32.to_le_bytes());
        output.extend_from_slice(&0.75f32.to_le_bytes());
        output.extend_from_slice(&[0; 4]);
        output.extend_from_slice(&1.5f32.to_le_bytes());
        output.extend_from_slice(&self.format.id().to_le_bytes());
        output.push(self.mip_count);
        match self.thumbnail {
            Some((width, height)) => {
                output.extend_from_slice(&ImageFormat::Dxt1.id().to_le_bytes());
                output.push(width);
                output.push(height);
            }
            None => {
                output.extend_from_slice(&(-1i32).to_le_bytes());
                output.extend_from_slice(&[0, 0]);
            }
        }
        output
    }

    fn face_count(&self) -> u8 {
        if !self.flags.contains(TextureFlags::ENVMAP) {
            1
        } else if self.minor == 0 || self.minor >= 5 || self.start_frame == u16::MAX {
            6
        } else {
            7
        }
    }
}

fn mip_dimension(base: u16, mip: u8) -> u16 {
    base.checked_shr(u32::from(mip)).unwrap_or(0).max(1)
}

fn image_byte_len(format: ImageFormat, width: u16, height: u16) -> usize {
    let width = usize::from(width);
    let height = usize::from(height);
    match format {
        ImageFormat::Rgba8888 => width * height * 4,
        ImageFormat::Dxt1 | ImageFormat::Dxt1OneBitAlpha => {
            width.div_ceil(4) * height.div_ceil(4) * 8
        }
        ImageFormat::Dxt3 | ImageFormat::Dxt5 => width.div_ceil(4) * height.div_ceil(4) * 16,
        other => panic!("unsupported fixture format {other:?}"),
    }
}

pub fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
