use std::collections::HashMap;
use std::usize;
use std::{f32::consts::FRAC_PI_3, ffi::CString, fs::File};

use raylib::prelude::*;
use raylib::{camera::Camera2D, color::Color, math::{Rectangle, Vector2}, texture::{RaylibTexture2D, Texture2D}, RaylibHandle, RaylibThread};

use crate::ase::aseprite::{self, Aseprite, AsepriteBlendMode, AsepriteError, AsepriteLayerFlags, AsepriteLayerType, AsepriteTagDirection};

use super::ui_main::{FONT_SIZE_BIG, FONT_SIZE_REG};

/// Used as the gap between cels on the grid.
pub(crate) const GAP: u16 = 4;

/// Used for the small lines separating the layers and frames, alpha is ignored.
const SMALL_LINE_COLOR: Color = Color::WHITESMOKE;
/// Used for the bigger lines between cells on the grid, alpha is ignored.
const BIG_LINE_COLOR:   Color = Color::GRAY;

/// Used for the layer names and frame numbers on the grid.
const LABEL_COLOR:      Color = SMALL_LINE_COLOR;

/// The colour used to signify linked cels and the animation pointing to the cel.
const LINKED_COLOR:     Color = Color::ORANGERED;
/// A generic error colour for trying to determine if something was drawn proper.
const ERR_COLOR:        Color = Color::FUCHSIA;

/// A general number to signify no parent. Should be a reasonably infeasible
/// number.
const NO_PARENT:       usize = usize::MAX;
/// How far the recursive functions can go until they stop.
const RECURSIVE_LIMIT: u8    = 16;

const DEBUG_VISUALS: bool = false;

pub struct PreparedCel {
    // image:       Option<Image>,
    texture:     Option<Texture2D>,
    frame_index: usize,
    layer_index: u16,
    position:    Vector2,
    size:        Vector2,
    opacity:     u8,

    linked_to_frame: Option<u16>,

    content_bounds: Rectangle,
    collision_bounds: Rectangle,

    hover: bool
}

pub struct PreparedLayer {
    pub child_level:  u16,
    pub blend_mode:   AsepriteBlendMode,
    pub opacity:      u8,
    pub name:         String,
    pub layer_type:   AsepriteLayerType,

    pub visible:      bool,
    pub background:   bool,
    pub is_reference: bool,

    pub parent_index: usize,
    pub full_name:    Option<String>
}

pub struct PreparedTag {
    from:      usize,
    to:        usize,
    direction: AsepriteTagDirection,
    name:      String
}

pub(crate) struct LoadedSprite {
    pub main_data: Aseprite,

    pub loaded_cels:   Vec<PreparedCel>,
    pub loaded_layers: Vec<PreparedLayer>,
    pub loaded_tags:   Vec<PreparedTag>,
    pub frame_count:   usize,

    offset: Vector2,

    cached_list: Option<Box<CString>>
}

impl LoadedSprite {
    /// Gets a layer's visibilty depending on its parents. Care should be taken
    /// as it is *recursive* with a depth limit specified by `RECURSIVE_LIMIT`.
    /// 
    /// If any layer in the process is not visible, the recursion stops early.
    pub(crate) fn is_layer_visible(&self, layer_index: usize) -> bool {
        self.internal_layer_visible(layer_index, RECURSIVE_LIMIT)
    }

    fn internal_layer_visible(&self, layer_index: usize, deepness: u8) -> bool {
        let layer = &self.loaded_layers[layer_index];
        if layer.visible && layer.parent_index != NO_PARENT && deepness > 0 {
            layer.visible && self.internal_layer_visible(layer.parent_index, deepness.min(RECURSIVE_LIMIT) - 1)
        } else { layer.visible } // should be false unless it's really deep
    }

    /// Gets full name of a layer. Should **NOT** be repeatedly called as it is:
    /// - *Recursive* (to a maximum depth specified by `RECURSIVE_LIMIT`)
    /// - *Clones* all `String`s from a layer and its parent's, and so on
    /// 
    /// Internally this is called with `LoadedSprite::load()` for layers and is
    /// stored as its `full_name` within an option, and using the layer's
    /// `full_name` should be used instead of calling this.
    pub(crate) fn layer_name(&self, layer_index: usize) -> String {
        self.internal_layer_name(layer_index, RECURSIVE_LIMIT)
    }

    fn internal_layer_name(&self, layer_index: usize, deepness: u8) -> String {
        let layer = &self.loaded_layers[layer_index];
        let mut result = layer.name.clone();
        if layer.parent_index != NO_PARENT && deepness > 0 {
            result = format!("{}.{}", result, self.internal_layer_name(layer.parent_index, deepness.min(RECURSIVE_LIMIT) - 1))
        }
        result
    }

    pub fn load(fname: &str, rl: &mut RaylibHandle, thread: &RaylibThread) -> Result<Self, AsepriteError> {
        let mut f_in = match File::open(fname) {
            Ok(f) => f,
            Err(e) => return Err(AsepriteError::Other(Box::new(e))),
        };
    
        let mut main_data: Aseprite = match aseprite::read(&mut f_in) {
            Ok(d) => d,
            Err(err) => return Err(err),
        };

        let mut loaded_cels = vec![];
        let mut loaded_layers = vec![];
        let mut loaded_tags = vec![];

        let offset = Vector2{
            x: (main_data.header.width * main_data.header.pixel_width as u16 + GAP) as f32,
            y: (main_data.header.height * main_data.header.pixel_height as u16 + GAP) as f32
        };

        for (frame_idx, frame) in main_data.frames.iter_mut().enumerate() {
            for (chunk_idx, chunk) in frame.chunks.iter_mut().enumerate() {
                match chunk {
                    aseprite::Chunk::Layer(lchunk) => {
                        loaded_layers.push(PreparedLayer {
                            child_level:  lchunk.child_level,
                            blend_mode:   lchunk.blend_mode,
                            opacity:      lchunk.opacity,
                            layer_type:   lchunk.layer_type,
                            visible:      lchunk.flags & AsepriteLayerFlags::Visible > 0,
                            background:   lchunk.flags & AsepriteLayerFlags::Background > 0,
                            is_reference: lchunk.flags & AsepriteLayerFlags::IsReference > 0,
                            name:         lchunk.name.as_str().ok().unwrap_or(format!("frame{frame_idx} chunk{chunk_idx}").as_str()).to_owned(),
                            full_name:    None,

                            parent_index: NO_PARENT,
                        });
                    },
                    aseprite::Chunk::Cel(cel) => {
                        match cel.cel_type {
                            aseprite::AsepriteCelType::Raw | aseprite::AsepriteCelType::CompressedImage => {
                                if let Some(img_data) = &mut cel.raw_data {
                                    let mut img = raylib::texture::Image::gen_image_color(
                                        cel.width.unwrap_or(1).into(), cel.height.unwrap_or(1).into(),
                                        ERR_COLOR
                                    );
                                    
                                    img.set_format(match &main_data.header.colour_depth {
                                        32 => raylib::consts::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8,
                                        16 => raylib::consts::PixelFormat::PIXELFORMAT_UNCOMPRESSED_GRAY_ALPHA,
                                        _ => panic!("unsupported colour depth")
                                    });
        
                                    let mut txtr = rl.load_texture_from_image(thread, &img).unwrap();
                                    txtr.update_texture(&img_data);
        
                                    loaded_cels.push(PreparedCel{
                                        // image:           Some(img),
                                        layer_index:     cel.layer_index,
                                        frame_index:     frame_idx,
                                        texture:         Some(txtr),
                                        linked_to_frame: None,
                                        position:        Vector2 { x: cel.x_pos as f32, y: cel.y_pos as f32 },
                                        size:            Vector2 { x: cel.width.unwrap_or(0) as f32, y: cel.height.unwrap_or(0) as f32 },
                                        opacity:         cel.opacity,
                                        content_bounds:          Rectangle {
                                            x: frame_idx as f32 + cel.x_pos as f32,
                                            y: (cel.layer_index as f32 - cel.y_pos as f32) * -1.0,
                                            width: cel.width.unwrap_or(0) as f32,
                                            height: cel.height.unwrap_or(0) as f32
                                        },
                                        collision_bounds:       Rectangle{
                                            x: frame_idx as f32 * offset.x,
                                            y: cel.layer_index as f32 * offset.y * -1.0,
                                            width: main_data.header.width as f32 * main_data.header.pixel_width.max(1) as f32,
                                            height: main_data.header.height as f32 * main_data.header.pixel_height.max(1) as f32,
                                        },
                                        hover: false
                                    });
                                }
                            },
                            aseprite::AsepriteCelType::Linked => {
                                loaded_cels.push(PreparedCel{
                                    // image:           None,
                                    layer_index:     cel.layer_index,
                                    frame_index:     frame_idx,
                                    texture:         None,
                                    linked_to_frame: cel.linked_to,
                                    position:        Vector2 { x: 0.0, y: 0.0 },
                                    size:            Vector2 { x: main_data.header.width as f32, y: main_data.header.height as f32 },
                                    opacity:         255,
                                    content_bounds:          Rectangle {
                                        x: frame_idx as f32,
                                        y: cel.layer_index as f32 * -1.0,
                                        width: main_data.header.width as f32,
                                        height: main_data.header.height as f32
                                    },
                                    collision_bounds:       Rectangle{
                                        x: frame_idx as f32 * offset.x,
                                        y: cel.layer_index as f32 * offset.y * -1.0,
                                        width: main_data.header.width as f32 * main_data.header.pixel_width.max(1) as f32,
                                        height: main_data.header.height as f32 * main_data.header.pixel_height.max(1) as f32,
                                    },
                                    hover: false
                                });
                            },
                            _ => unimplemented!(),
                        };
                    },
                    aseprite::Chunk::Tag(tag) => {
                        let mut i = 0;
                        for tag in &tag.tags {
                            loaded_tags.push(PreparedTag {
                                from:      tag.from.into(),
                                to:        tag.to.into(),
                                direction: tag.direction,
                                name:      tag.name.as_str().unwrap_or(format!("Tag {i}").as_str()).to_owned(),
                            });
                            i += 1;
                        }
                    }
                    _ => ()
                }
            }
        }

        {
            let mut parent_map: HashMap<i32, usize> = HashMap::<i32, usize>::new();
            parent_map.insert(-1, NO_PARENT);

            for layer_idx in 0..loaded_layers.len() {
                let layer = &mut loaded_layers[layer_idx];
                parent_map.insert(layer.child_level as i32, layer_idx);

                layer.parent_index = *parent_map.get(&((layer.child_level as i32) - 1)).unwrap_or(&NO_PARENT);
            }
        }

        let frame_count = main_data.frames.len();
        let mut r = Self {
            main_data, loaded_cels, loaded_layers, loaded_tags, frame_count,

            offset,

            cached_list: None
        };

        for layer_index in 0..r.loaded_layers.len() {
            r.loaded_layers[layer_index].full_name = Some(r.layer_name(layer_index))
        }

        Ok(r)
    }

    pub fn draw(&mut self, d: &mut RaylibMode2D<'_, RaylibDrawHandle<'_>>, cam: &Camera2D) {
        let header = &self.main_data.header;

        let scale_x: i32 = header.pixel_width.max(1).into();
        let scale_y: i32 = header.pixel_height.max(1).into();

        let image_width = header.width;
        let image_height = header.height;

        for img in self.loaded_cels.iter() {
            let my_layer = &self.loaded_layers[img.layer_index as usize];

            if !self.is_layer_visible(img.layer_index as usize) {
                continue;
            }

            let rect_colour = Color{
                a: if img.hover { 96 } else { 32 },
                ..match img.linked_to_frame {
                    Some(_) => LINKED_COLOR,
                    None    => Color::WHITE,
                }
            };

            d.draw_rectangle_lines(
                (img.content_bounds.x + img.frame_index as f32 * (self.offset.x - 1.0)) as i32 - img.position.x as i32,
                (img.content_bounds.y - img.layer_index as f32 * (self.offset.y - 1.0)) as i32 - img.position.y as i32,
                image_width as i32 * scale_x,
                image_height as i32 * scale_y,
                rect_colour
            );

            if let Some(link) = img.linked_to_frame {
                if img.hover {
                    d.draw_line_ex(
                        Vector2{
                            x: (img.frame_index as f32 * (self.offset.x) + (image_width as f32 / 2.0)), 
                            y: (img.layer_index as f32 * (self.offset.y) - (image_height as f32 / 2.0)) * -1.0
                        },
                        Vector2{
                            x: (link as f32 * (self.offset.x) + (image_width as f32 / 2.0)),
                            y: (img.layer_index as f32 * (self.offset.y) - (image_height as f32 / 2.0)) * -1.0
                        },
                        3.0,
                        rect_colour
                    );

                    d.draw_circle(
                        (link as f32 * (self.offset.x) + (image_width as f32 / 2.0)) as i32, 
                        (img.layer_index as f32 * (self.offset.y) - (image_height as f32 / 2.0)) as i32 * -1, 
                        6.0 + f32::sin(d.get_time() as f32 * 1.7) * 2.4,
                        rect_colour
                    );

                    for i in 0..(img.frame_index as u16 - link) {
                        let cx = ((link + i + 1) as f32 - d.get_time().fract() as f32) * (self.offset.x) + (image_width as f32 / 2.0);
                        let cy = (img.layer_index as f32 * (self.offset.y) - (image_height as f32 / 2.0)) * -1.0;
                        let r = 3.5;

                        let v1 = Vector2{
                            x: cx + (r * f32::cos(1.0 * FRAC_PI_3)),
                            y: cy + (r * f32::sin(1.0 * FRAC_PI_3))
                        };

                        let v2 = Vector2{
                            x: cx + (r * f32::cos(3.0 * FRAC_PI_3)),
                            y: cy + (r * f32::sin(3.0 * FRAC_PI_3))
                        };

                        let v3 = Vector2{
                            x: cx + (r * f32::cos(5.0 * FRAC_PI_3)),
                            y: cy + (r * f32::sin(5.0 * FRAC_PI_3))
                        };

                        d.draw_triangle(
                            v1,
                            v2,
                            v3, 
                            ERR_COLOR//rect_colour
                        );

                        d.draw_circle_v(v1, r, rect_colour);
                        d.draw_circle_v(v2, r, rect_colour);
                        d.draw_circle_v(v3, r, rect_colour);
                    }
                }

                let tx = (img.frame_index as f32 * (self.offset.x) + (image_width as f32 / 2.0)) as i32;
                let ty = (img.layer_index as f32 * (self.offset.y) - (image_height as f32 / 2.0) + 16.) as i32 * -1;
                
                d.draw_text(
                    format!("{}", link).as_str(), 
                    tx,
                    ty,
                    FONT_SIZE_BIG,
                    rect_colour
                );
            } else if let Some(texture) = &img.texture {
                d.draw_texture_pro(texture,
                    Rectangle{
                        x:      0.0,
                        y:      0.0,
                        width:  img.size.x,
                        height: img.size.y,
                    }, 
                    Rectangle{
                        x: (img.content_bounds.x + (img.frame_index as f32 * (self.offset.x - 1.0))) * scale_x as f32,
                        y: (img.content_bounds.y - (img.layer_index as f32 * (self.offset.y - 1.0))) * scale_y as f32,
                        width: img.size.x * scale_x as f32,
                        height: img.size.y * scale_y as f32,
                    }, 
                    Vector2{ x: 0.0, y: 0.0 }, 
                    0.0, 
                    Color{a: {
                        let l = (my_layer.opacity as f64) / 255.0;
                        let r = (img.opacity as f64) / 255.0;
                        (l * r * 255.0).round().clamp(0.0, 255.0) as u8
                    }, ..Color::WHITE}
                );
            }

            if DEBUG_VISUALS { d.draw_rectangle_lines_ex(img.collision_bounds, 2.0, ERR_COLOR); }
        }

        let line_alpha = (24. * cam.zoom).clamp(0., 255.) as u8;

        for (i, l) in self.loaded_layers.iter().enumerate() {
            let m = d.measure_text(&l.full_name.as_ref().unwrap(), FONT_SIZE_REG);
            let my_alpha = line_alpha / if l.visible { 1 } else { 2 };

            d.draw_text(
                l.full_name.as_ref().unwrap(),
                (16 + m) * -1,
                (self.offset.y as i32 * i as i32 - (self.offset.x / 2.0) as i32) * -1,
                FONT_SIZE_REG, LABEL_COLOR
            );
            
            let line_y = ((self.offset.y) as i32 * i as i32) * -1;
            let line_y2 = line_y + image_height as i32 * scale_y;
            
            d.draw_line(
                (16 + m) * -1, line_y,
                (16 + m) * -1 + (self.offset.x * self.frame_count as f32) as i32, line_y,
                Color{a: my_alpha, ..SMALL_LINE_COLOR}
            );
            
            d.draw_line(
                (16 + m) * -1, line_y2,
                (16 + m) * -1 + (self.offset.x * self.frame_count as f32) as i32, line_y2,
                Color{a: my_alpha, ..SMALL_LINE_COLOR}
            );

            d.draw_line_ex(
                Vector2{
                    x: ((16 + m) * -1) as f32,
                    y: line_y2 as f32 + (GAP / 2) as f32 + 0.5,
                }, Vector2{
                    x: ((16 + m) * -1) as f32 + (self.offset.x * self.frame_count as f32),
                    y: line_y2 as f32 + (GAP / 2) as f32 + 0.5,
                }, 
                GAP as f32 + 1.,
                Color{a: my_alpha/4, ..BIG_LINE_COLOR}
            );
        }

        for i in 0..self.frame_count {
            let fstr = format!("{}", i);
            let fstr = fstr.as_str();

            let width = image_width as i32 - d.measure_text(fstr, FONT_SIZE_REG);
            let width = width / 2;

            d.draw_text(fstr,
                ((self.offset.x) * i as f32) as i32 + width,
                (self.offset.y + 16.0) as i32,
                FONT_SIZE_REG,
                LABEL_COLOR
            );

            d.draw_text(fstr,
                ((self.offset.x) * i as f32) as i32 + width,
                (self.offset.y * (self.loaded_layers.len() - 1) as f32 + 16.0) as i32 * -1,
                FONT_SIZE_REG,
                LABEL_COLOR
            );
            
            let line_x = ((self.offset.x) * (i as f32) as f32) as i32;
            let line_x2 = line_x + image_width as i32 * scale_x;

            d.draw_line(
                line_x, (self.offset.y + 4.0) as i32, 
                line_x, (self.offset.y * (self.loaded_layers.len() - 1) as f32 + 16.0) as i32 * -1,
                Color{a: line_alpha, ..SMALL_LINE_COLOR}
            );

            d.draw_line(
                line_x2, (self.offset.y + 4.0) as i32, 
                line_x2, (self.offset.y * (self.loaded_layers.len() - 1) as f32 + 16.0) as i32 * -1,
                Color{a: line_alpha, ..SMALL_LINE_COLOR}
            );

            d.draw_line_ex(
                Vector2{
                    x: (line_x2 + GAP as i32 / 2) as f32 + 0.5,
                    y: (self.offset.y + 4.0),
                }, Vector2{
                    x: (line_x2 + GAP as i32 / 2) as f32 + 0.5,
                    y: (self.offset.y * (self.loaded_layers.len() - 1) as f32 + 16.0) * -1.0,
                }, 
                GAP as f32 + 1.,
                Color{a: line_alpha/4, ..BIG_LINE_COLOR}
            );
        }

        if DEBUG_VISUALS {
            let mouse_pt = d.get_screen_to_world2D(d.get_mouse_position(), cam);
            d.draw_line(
                mouse_pt.x as i32, mouse_pt.y as i32 - 16,
                mouse_pt.x as i32, mouse_pt.y as i32 + 16,
                ERR_COLOR
            );
            d.draw_line(
                mouse_pt.x as i32 - 16, mouse_pt.y as i32,
                mouse_pt.x as i32 + 16, mouse_pt.y as i32,
                ERR_COLOR
            );

            if let Some(img) = self.loaded_cels.iter().find(|x| x.hover) {
                d.draw_text(
                    format!("l{}f{}\nctnt{:?}\ncoll{:?}", img.layer_index, img.frame_index, img.content_bounds, img.collision_bounds).as_str(),
                    img.collision_bounds.x as i32, img.collision_bounds.y as i32,
                    FONT_SIZE_REG, Color::LIME
                );
            }
        }
    }

    pub fn step(&mut self, rl: &mut RaylibHandle, cam: &Camera2D) {
        let mouse_pt = rl.get_screen_to_world2D(rl.get_mouse_position(), cam);

        for img in &mut self.loaded_cels {
            img.hover = img.collision_bounds.check_collision_point_rec(mouse_pt);
        }
    }

    pub fn invalidate_layer_list(&mut self) {
        self.cached_list = None
    }

    pub fn generate_layer_list(&mut self) -> &CString {
        if self.cached_list.is_none() {
            self.cached_list = Some(Box::new(CString::new(self.loaded_layers.iter().rev()
            .map(|i| {
                format!("{} {}",
                    match i.layer_type {
                        AsepriteLayerType::Normal  => if i.visible { if i.is_reference { "#15#" } else { "#44#" } } else { "#45#" },
                        AsepriteLayerType::Group   => if i.visible { "#217#" } else { "#45#" },
                        AsepriteLayerType::Tilemap => if i.visible { "#97#" } else { "#45#" },
                    },
                    i.full_name.as_ref().unwrap()
                )
            }).collect::<Vec<String>>().join(";").as_str()).ok().unwrap()));
        }

        self.cached_list.as_ref().unwrap()
    }
}