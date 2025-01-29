use std::{fmt::Display, io::{self, Read}, ops::BitAnd, rc::Rc};
use flate2::bufread::ZlibDecoder;

/// Makes a type from a slice of little endian bytes. If it fails, it spits out 0.
/// 
/// Expected types are the integer primitives, like `u16`, `u32`, `i16`, and `i32`.
/// They must implement `from_le_bytes` that takes an array of `u8`.
macro_rules! slice_to {
    ($type_to: ty, $slice: expr) => { <$type_to>::from_le_bytes($slice.try_into().unwrap_or([0; size_of::<$type_to>()])) };
}

/// Makes an array from `from` to `from + length`. If it fails, it unwraps to an arry of `length` 0s.
/// 
/// Expected usage:
/// `slice_cnt!( vec : identifer, from : expression, length : literal )`
/// 
/// `vec` is expected to be a vector of bytes.
macro_rules! slice_cnt {
    ($vec: ident, $from: expr, $length: literal) => { $vec[$from..($from+$length)].try_into().unwrap_or([0; $length]) };
}

pub struct Aseprite {
    pub header: AsepriteHeader,
    pub frames: Vec<AsepriteFrame>
}

const ASEPRITE_MAGIC_HEADER: u16 = 0xA5E0;
const ASEPRITE_MAGIC_FRAMES: u16 = 0xF1FA;

pub struct AsepriteString {
    length: u16,
    data: Vec<u8> 
}

impl AsepriteString {
    pub fn read_from_bytes(from: &[u8]) -> Self {
        let length = slice_to!(u16, &from[0..2]);
        let data = from[2..].to_vec();
        Self {
            length,
            data
        }
    }

    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.data)
    }
}

pub struct AsepriteHeader {
    pub fsize:           u32,
	pub magic:           u16,
	pub frames:          u16,
	pub width:           u16,
	pub height:          u16,
	pub colour_depth:    u16,
	pub flags:           u32,
	pub speed:           u16,
	    zero:            [u8; 8],
	pub palette_entry:   u8,
	    ignore:          [u8; 3],
	pub colour_count:    u16,
	pub pixel_width:     u8,
	pub pixel_height:    u8,
	pub grid_xpos:       i16,
	pub grid_ypos:       i16,
	pub grid_width:      u16,
	pub grid_height:     u16,

    future:          [u8; 84]
}

pub struct AsepriteFrame {
    pub size: u32,
    pub magic: u16,
    pub old_chunks: u16,
    pub frame_duration: u16,
    future: [u8; 2],

    pub chunk_count: u32,
    pub chunks: Vec<Chunk>
}

pub enum Chunk {
    Unknown(RawAsepriteChunk),
    Layer(AsepriteLayerChunk),
    Cel(AsepriteCelChunk),
    Tag(AsepriteTagChunk)
}

impl Chunk {
    pub fn name(&self) -> &str {
        match self {
            Self::Unknown(_) => "unknown",
            Self::Layer(_)   => "layer",
            Self::Cel(_)     => "cel",
            Self::Tag(_)     => "tag",
        }
    }
}

pub struct RawAsepriteChunk {
    pub size: u32,
    pub chunk_type: u16,
    pub data: Rc<[u8]>
}

#[repr(u16)]
#[derive(Clone, Copy)]
pub enum AsepriteBlendMode {
    Normal     = 00,
    Multiply   = 01,
    Screen     = 02,
    Overlay    = 03,
    Darken     = 04,
    Lighten    = 05,
    ColorDodge = 06,
    ColorBurn  = 07,
    HardLight  = 08,
    SoftLight  = 09,
    Difference = 10,
    Exclusion  = 11,
    Hue        = 12,
    Saturation = 13,
    Color      = 14,
    Luminosity = 15,
    Addition   = 16,
    Subtract   = 17,
    Divide     = 18
}

impl From<u16> for AsepriteBlendMode {
    fn from(value: u16) -> Self {
        match value % 19 {
            00 => Self::Normal,
            01 => Self::Multiply,
            02 => Self::Screen,
            03 => Self::Overlay,
            04 => Self::Darken,
            05 => Self::Lighten,
            06 => Self::ColorDodge,
            07 => Self::ColorBurn,
            08 => Self::HardLight,
            09 => Self::SoftLight,
            10 => Self::Difference,
            11 => Self::Exclusion,
            12 => Self::Hue,
            13 => Self::Saturation,
            14 => Self::Color,
            15 => Self::Luminosity,
            16 => Self::Addition,
            17 => Self::Subtract,
            18 => Self::Divide,

            _ => unreachable!("should be impossible value from modulo bound")
        }
    }
}

impl Display for AsepriteBlendMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal     => f.write_str("Normal"),
            Self::Multiply   => f.write_str("Multiply"),
            Self::Screen     => f.write_str("Screen"),
            Self::Overlay    => f.write_str("Overlay"),
            Self::Darken     => f.write_str("Darken"),
            Self::Lighten    => f.write_str("Lighten"),
            Self::ColorDodge => f.write_str("Color Dodge"),
            Self::ColorBurn  => f.write_str("Color Burn"),
            Self::HardLight  => f.write_str("Hard Light"),
            Self::SoftLight  => f.write_str("Soft Light"),
            Self::Difference => f.write_str("Difference"),
            Self::Exclusion  => f.write_str("Exclusion"),
            Self::Hue        => f.write_str("Hue"),
            Self::Saturation => f.write_str("Saturation"),
            Self::Color      => f.write_str("Color"),
            Self::Luminosity => f.write_str("Luminosity"),
            Self::Addition   => f.write_str("Addition"),
            Self::Subtract   => f.write_str("Subtract"),
            Self::Divide     => f.write_str("Divide"),
        }
    }
}

#[repr(u16)]
#[derive(PartialEq)]
pub enum AsepriteLayerType {
    Normal = 0,
    Group,
    Tilemap
}

impl From<u16> for AsepriteLayerType {
    fn from(value: u16) -> Self {
        match value % 3 {
            0 => Self::Normal,
            1 => Self::Group,
            2 => Self::Tilemap,
            _ => unreachable!("should be impossible value from modulo bound")
        }
    }
}

const ASEPRITE_LAYER_CHUNK_MAGIC: u16 = 0x2004;
pub struct AsepriteLayerChunk {
    pub flags: u16,
    pub layer_type: AsepriteLayerType,
    pub child_level: u16,
        default_width: u16,
        default_height: u16,
    pub blend_mode: AsepriteBlendMode,
    pub opacity: u8,
        future: [u8; 3],
    pub name: AsepriteString,

    pub tileset_index: Option<u32> // only if layer type == 2
}

#[repr(u16)]
pub enum AsepriteLayerFlags {
    Visible           = 1,
    Editable          = 1 << 1,
    LockMovement      = 1 << 2,
    Background        = 1 << 3,
    PreferLinkingCels = 1 << 4,
    IsCollapsed       = 1 << 5,
    IsReference       = 1 << 6,
}

impl BitAnd<AsepriteLayerFlags> for u16 {
    type Output = u16;

    fn bitand(self, rhs: AsepriteLayerFlags) -> Self::Output {
        self & rhs as Self::Output
    }
}

#[repr(u16)]
#[derive(PartialEq, Clone, Copy)]
pub enum AsepriteCelType {
    Raw = 0,
    Linked,
    CompressedImage,
    CompressedTilemap
}

impl From<u16> for AsepriteCelType {
    fn from(value: u16) -> Self {
        match value % 4 {
            0 => Self::Raw,
            1 => Self::Linked,
            2 => Self::CompressedImage,
            3 => Self::CompressedTilemap,
            _ => unreachable!("should be impossible value from modulo bound"),
        }
    }
}

impl Display for AsepriteCelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw               => f.write_str("Raw"),
            Self::Linked            => f.write_str("Linked"),
            Self::CompressedImage   => f.write_str("CompressedImage"),
            Self::CompressedTilemap => f.write_str("CompressedTilemap"),
        }
    }
}

const ASEPRITE_CEL_CHUNK_MAGIC: u16 = 0x2005;
pub struct AsepriteCelChunk {
    pub layer_index: u16,
    pub x_pos: i16,
    pub y_pos: i16,
    pub opacity: u8,
    pub cel_type: AsepriteCelType,
    pub z_index: i16,
        future: [u8; 5],

    // cel type 0
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub raw_data: Option<Rc<[u8]>>,

    // cel type 1
    pub linked_to: Option<u16>,

    // cel type 2
    // reuse width
    // reuse height
    pub compressed_data: Option<Rc<[u8]>>,

    // cel type 3
    // i don't want to support it
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum AsepriteTagDirection {
    Forward = 0,
    Reverse,
    PingPong,
    PingPongReverse
}

impl From<u8> for AsepriteTagDirection {
    fn from(value: u8) -> Self {
        match value % 4 {
            0 => Self::Forward,
            1 => Self::Reverse,
            2 => Self::PingPong,
            3 => Self::PingPongReverse,
            _ => unreachable!("should be impossible value from modulo bound")
        }
    }
}

impl Display for AsepriteTagDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forward         => f.write_str("Forward"),
            Self::Reverse         => f.write_str("Reverse"),
            Self::PingPong        => f.write_str("PingPong"),
            Self::PingPongReverse => f.write_str("PingPongReverse"),
        }
    }
}

const ASEPRITE_TAG_CHUNK_MAGIC: u16 = 0x2018;
pub struct AsepriteTagChunk {
    pub tag_count: u16,
        future: [u8; 8],

    pub tags: Vec<AsepriteTag>
}

pub struct AsepriteTag {
    pub from: u16,
    pub to: u16,
    pub direction: AsepriteTagDirection,
    pub repeat_count: u16,
        reserved: [u8; 6],
        colour: [u8; 3],
        extra: u8,
    pub name: AsepriteString
}

pub fn read<T: io::Read + io::Seek>(from: &mut T) -> Result<Aseprite, ()> {
    let mut header: Vec<u8> = vec![];
    header.reserve(size_of::<AsepriteHeader>());
    header.resize(header.capacity(), 0);
    
    match from.read(&mut header) {
        Ok(count) => if count != size_of::<AsepriteHeader>() { return Err(()) },
        Err(_e) => return Err(())
    };
    
    let mut result = Aseprite{
        header: AsepriteHeader{
            fsize:         slice_to!(u32, &header[00..04]),
            magic:         slice_to!(u16, &header[04..06]),
            frames:        slice_to!(u16, &header[06..08]),
            width:         slice_to!(u16, &header[08..10]),
            height:        slice_to!(u16, &header[10..12]),
            colour_depth:  slice_to!(u16, &header[12..14]),
            flags:         slice_to!(u32, &header[14..18]),
            speed:         slice_to!(u16, &header[18..20]),
            zero:          slice_cnt!(header, 20, 8),
            palette_entry: header[28],
            ignore:        slice_cnt!(header, 29, 3),
            colour_count:  slice_to!(u16, &header[32..34]),
            pixel_width:   header[34],
            pixel_height:  header[35],
            grid_xpos:     slice_to!(i16, &header[36..38]),
            grid_ypos:     slice_to!(i16, &header[38..40]),
            grid_width:    slice_to!(u16, &header[40..42]),
            grid_height:   slice_to!(u16, &header[42..44]),
            future:        slice_cnt!(header, 44, 84)
        },
        frames: Default::default(),
    };

    if result.header.magic != ASEPRITE_MAGIC_HEADER {
        return Err(());
    }

    let mut frame_buffer: Vec<u8> = vec![];
    frame_buffer.reserve(16);
    frame_buffer.resize(frame_buffer.capacity(), 0);

    let mut frame_count = 0;

    while from.read(&mut frame_buffer).unwrap_or(0) > 0 {
        if result.header.frames < frame_count {
            println!("frame count in header is lower than what is in file, continuing..\n-> decoding frame no. {frame_count} when header states only {} frames", result.header.frames)
        }

        let mut frame = AsepriteFrame{
            size:           slice_to!(u32, &frame_buffer[00..04]),
            magic:          slice_to!(u16, &frame_buffer[04..06]),
            old_chunks:     slice_to!(u16, &frame_buffer[06..08]),
            frame_duration: slice_to!(u16, &frame_buffer[08..10]),
            future:         slice_cnt!(frame_buffer, 10, 2),
            chunk_count:    slice_to!(u32, &frame_buffer[12..16]),
            chunks:         Vec::new(),
        };

        if frame.magic != ASEPRITE_MAGIC_FRAMES {
            return Err(());
        }

        let frames_end = from.stream_position().unwrap_or_default() + frame.size as u64;

        for _ in 0..frame.chunk_count {
            let current_position = from.stream_position().unwrap_or_default();

            if current_position >= frames_end {
                println!("frame data spills out of the size in the header at pos {}!\ngoing to continue..", current_position)
            }

            let size = {
                let mut buffer = [0u8; size_of::<u32>()];
                from.read(&mut buffer).unwrap();
    
                u32::from_le_bytes(buffer)
            };

            let chunk_type = {
                let mut buffer = [0u8; size_of::<u16>()];
                from.read(&mut buffer).unwrap();

                u16::from_le_bytes(buffer)
            };

            let mut data: Vec<u8> = vec![];
            data.reserve(size.try_into().unwrap());
            data.resize(data.capacity(), 0);

            from.seek(io::SeekFrom::Start(current_position)).unwrap();
            from.read(&mut data).unwrap();

            frame.chunks.push(
                match chunk_type {
                    ASEPRITE_LAYER_CHUNK_MAGIC => {
                        let layer_type = AsepriteLayerType::from(slice_to!(u16, &data[08..10]));
                        let is_tilemap = layer_type == AsepriteLayerType::Tilemap;
                        Chunk::Layer(AsepriteLayerChunk {
                            flags:          slice_to!(u16, &data[06..08]),
                            // layer_type                       [08..10]
                            child_level:    slice_to!(u16, &data[10..12]),
                            default_width:  slice_to!(u16, &data[12..14]),
                            default_height: slice_to!(u16, &data[14..16]),
                            blend_mode:     AsepriteBlendMode::from(slice_to!(u16, &data[16..18])),
                            opacity:        data[18],
                            future:         slice_cnt!(data, 19, 3),
                            name:           AsepriteString::read_from_bytes(
                                                &data[22..(data.len() - if is_tilemap { 4 } else { 0 })]
                                            ),
                            
                            tileset_index:  if is_tilemap {
                                                Some(slice_to!(u32, &data[data.len()-4..data.len()]))
                                            } else { None },
                            
                            layer_type
                        })
                    },
                    ASEPRITE_CEL_CHUNK_MAGIC => {
                        let mut r = Chunk::Cel(AsepriteCelChunk {
                            layer_index: slice_to!(u16, &data[06..08]),
                            x_pos:       slice_to!(i16, &data[08..10]),
                            y_pos:       slice_to!(i16, &data[10..12]),
                            opacity:     data[12],
                            cel_type:    AsepriteCelType::from(slice_to!(u16, &data[13..15])),
                            z_index:     slice_to!(i16, &data[15..17]),
                            future:      slice_cnt!(data, 17, 5),
                            
                            // cel specific fields set below
                            // cel type 0
                            width: None,
                            height: None,
                            raw_data: None,
                            
                            // cel type 1
                            linked_to: None,
                            
                            // cel type 2
                            // width,
                            // height,
                            compressed_data: None
                        });

                        if let Chunk::Cel(ref mut c) = &mut r {
                            match c.cel_type {
                                AsepriteCelType::Raw => {
                                    c.width =    Some(slice_to!(u16, &data[22..24]));
                                    c.height =   Some(slice_to!(u16, &data[24..26]));
                                    c.raw_data = Some(data[26..].into());
                                },

                                AsepriteCelType::Linked => {
                                    c.linked_to = Some(slice_to!(u16, &data[22..24]));
                                },

                                AsepriteCelType::CompressedImage => {
                                    c.width =           Some(slice_to!(u16, &data[22..24]));
                                    c.height =          Some(slice_to!(u16, &data[24..26]));
                                    c.compressed_data = Some(data[26..].into());

                                    // decompress the image data now for easier use later

                                    let slicedata = c.compressed_data.as_ref().unwrap();
                                    let slicedata = slicedata.as_ref();
                                    let mut z = ZlibDecoder::new(slicedata);
                                    let mut r = vec![];
                                    match z.read_to_end(&mut r) {
                                        Ok(_) => (),
                                        Err(_) => return Err(()),
                                    }

                                    c.raw_data = Some(r.into());
                                },

                                AsepriteCelType::CompressedTilemap => {
                                    unimplemented!("compressed tilemap unsupported");
                                },
                            };
                        }

                        r
                    },
                    ASEPRITE_TAG_CHUNK_MAGIC => {
                        let mut tag_data = AsepriteTagChunk {
                            tag_count:  slice_to!(u16, &data[06..08]),
                            future:     slice_cnt!(data, 8, 8),
                            tags:       Vec::<AsepriteTag>::new() 
                        };

                        let mut offset = 16;
                        for _ in 0..tag_data.tag_count {
                            let name_len = slice_to!(u16, &data[(17 + offset)..(19 + offset)]) as usize;

                            tag_data.tags.push(AsepriteTag {
                                from:           slice_to!(u16, &data[(00 + offset)..(02 + offset)]),
                                to:             slice_to!(u16, &data[(02 + offset)..(04 + offset)]),
                                direction:      AsepriteTagDirection::from(data[4 + offset]),
                                repeat_count:   slice_to!(u16, &data[(05 + offset)..(07 + offset)]),
                                reserved:       slice_cnt!(data, {7 + offset}, 6),
                                colour:         slice_cnt!(data, {13 + offset}, 3),
                                extra:          data[ 16 + offset],
                                name:           AsepriteString::read_from_bytes(
                                                    &data[(17 + offset)..((19 + offset) + name_len)]
                                                )
                            });

                            offset += 19 + name_len
                        }

                        Chunk::Tag(tag_data)
                    },
                    _ => Chunk::Unknown(RawAsepriteChunk { size, chunk_type, data: data.into() })
                }
            );
        }
    
        result.frames.push(frame);
        frame_count += 1;
    }

    Ok(result)
}