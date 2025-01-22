use std::{f32::consts::FRAC_PI_3, ffi::CString, fs::File};

use raylib::prelude::*;
use raylib::{camera::Camera2D, color::Color, math::{Rectangle, Vector2}, rgui::RaylibDrawGui, texture::{RaylibTexture2D, Texture2D}, RaylibHandle, RaylibThread};

use crate::ase::aseprite::{self, Aseprite, AsepriteBlendMode, AsepriteLayerFlags, AsepriteTagDirection};

use super::ui_main::{FONT_SIZE_BIG, FONT_SIZE_REG, WINDOW_H};

const GAP: u16 = 4;

const SMALL_LINE_COLOR: Color = Color::WHITESMOKE;
const BIG_LINE_COLOR:   Color = Color::GRAY;

const LABEL_COLOR:      Color = SMALL_LINE_COLOR;

const LINKED_COLOR:     Color = Color::ORANGERED;
const ERR_COLOR:        Color = Color::FUCHSIA;

pub struct PreparedCel {
    // image:       Option<Image>,
    texture:     Option<Texture2D>,
    frame_index: usize,
    layer_index: u16,
    position:    Vector2,
    size:        Vector2,
    opacity:     u8,

    linked_to_frame: Option<u16>,

    bounds: Rectangle,

    hover: bool
}

pub struct PreparedLayer {
    child_level:  u16,
    blend_mode:   AsepriteBlendMode,
    opacity:      u8,
    name:         String,

    visible:      bool,
    background:   bool,
    is_reference: bool
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

    layer_scroll: i32,
    layer_active: i32,
}

impl LoadedSprite {
    pub fn load(fname: &str, rl: &mut RaylibHandle, thread: &RaylibThread) -> Result<Self, ()> {
        let mut f_in = File::open(fname).unwrap();
    
        let mut data = aseprite::read(&mut f_in).unwrap();

        let mut loaded_cels = vec![];
        let mut loaded_layers = vec![];
        let mut loaded_tags = vec![];

        for frame_idx in 0..data.frames.len() {
            let frame = &mut data.frames[frame_idx];
            for chunk in &mut frame.chunks {
                match chunk {
                    aseprite::Chunk::Layer(lchunk) => {
                        loaded_layers.push(PreparedLayer {
                            child_level:    lchunk.child_level,
                            blend_mode:     lchunk.blend_mode,
                            opacity:        lchunk.opacity,
                            visible:        lchunk.flags & AsepriteLayerFlags::Visible > 0,
                            background:     lchunk.flags & AsepriteLayerFlags::Background > 0,
                            is_reference:   lchunk.flags & AsepriteLayerFlags::IsReference > 0,
                            name:           lchunk.name.as_str().unwrap().to_owned()
                        });
                    },
                    aseprite::Chunk::Cel(cel) => {
                        match cel.cel_type {
                            aseprite::AsepriteCelType::Raw | aseprite::AsepriteCelType::CompressedImage => {
                                if let Some(img_data) = &mut cel.raw_data {
                                    let mut img = raylib::texture::Image::gen_image_color(
                                        cel.width.unwrap().into(), cel.height.unwrap().into(),
                                        ERR_COLOR
                                    );
                                    
                                    img.set_format(match &data.header.colour_depth {
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
                                        bounds:          Rectangle {
                                            x: frame_idx as f32 + cel.x_pos as f32,
                                            y: (cel.layer_index as f32 - cel.y_pos as f32) * -1.0,
                                            width: cel.width.unwrap_or(0) as f32,
                                            height: cel.height.unwrap_or(0) as f32
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
                                    size:            Vector2 { x: data.header.width as f32, y: data.header.height as f32 },
                                    opacity:         255,
                                    bounds:          Rectangle {
                                        x: frame_idx as f32,
                                        y: cel.layer_index as f32 * -1.0,
                                        width: data.header.width as f32,
                                        height: data.header.height as f32
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

        let frame_count = data.frames.len();
        Ok(Self { main_data: data, loaded_cels, loaded_layers, loaded_tags, frame_count, layer_scroll: 0, layer_active: 0 })
    }

    pub fn draw(&mut self, d: &mut RaylibMode2D<'_, RaylibDrawHandle<'_>>, cam: &Camera2D) {
        let header = &self.main_data.header;

        let scale_x: i32 = header.pixel_width.max(1).into();
        let scale_y: i32 = header.pixel_height.max(1).into();

        let image_width = header.width;
        let image_height = header.height;

        let off = Vector2{
            x: (image_width * scale_x as u16 + GAP) as f32,
            y: (image_height * scale_y as u16 + GAP) as f32
        };

        for i in 0..self.loaded_cels.len() {
            let img = &self.loaded_cels[i];
            let my_layer = &self.loaded_layers[img.layer_index as usize];

            if !my_layer.visible || my_layer.is_reference {
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
                img.bounds.x as i32 + (img.frame_index as f32 * off.x as f32) as i32 - img.position.x as i32,
                img.bounds.y as i32 - (img.layer_index as f32 * off.y as f32) as i32 - img.position.y as i32,
                image_width as i32 * scale_x,
                image_height as i32 * scale_y,
                rect_colour
            );

            if let Some(link) = img.linked_to_frame {
                if img.hover {
                    d.draw_line_ex(
                        Vector2{
                            x: (img.frame_index as f32 * (off.x as f32 + 1.0) + (image_width as f32 / 2.0)), 
                            y: (img.layer_index as f32 * (off.y as f32 + 1.0) - (image_height as f32 / 2.0)) * -1.0
                        },
                        Vector2{
                            x: (link as f32 * (off.x as f32 + 1.0) + (image_width as f32 / 2.0)),
                            y: (img.layer_index as f32 * (off.y as f32 + 1.0) - (image_height as f32 / 2.0)) * -1.0
                        },
                        3.0,
                        rect_colour
                    );

                    d.draw_circle(
                        (link as f32 * (off.x as f32 + 1.0) + (image_width as f32 / 2.0)) as i32, 
                        (img.layer_index as f32 * (off.y as f32 + 1.0) - (image_height as f32 / 2.0)) as i32 * -1, 
                        6.0 + f32::sin(d.get_time() as f32 * 1.7) * 2.4,
                        rect_colour
                    );

                    for i in 0..(img.frame_index as u16 - link) {
                        let cx = (((link + i + 1) as f32 - d.get_time().fract() as f32) * (off.x as f32 + 1.0) + (image_width as f32 / 2.0)) as i32;
                        let cy = (img.layer_index as f32 * (off.y as f32 + 1.0) - (image_height as f32 / 2.0)) as i32 * -1;
                        let r = 3.5;

                        let v1 = Vector2{
                            x: cx as f32 + (r * f32::cos(1.0 * FRAC_PI_3)),
                            y: cy as f32 + (r * f32::sin(1.0 * FRAC_PI_3))
                        };

                        let v2 = Vector2{
                            x: cx as f32 + (r * f32::cos(3.0 * FRAC_PI_3)),
                            y: cy as f32 + (r * f32::sin(3.0 * FRAC_PI_3))
                        };

                        let v3 = Vector2{
                            x: cx as f32 + (r * f32::cos(5.0 * FRAC_PI_3)),
                            y: cy as f32 + (r * f32::sin(5.0 * FRAC_PI_3))
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

                let tx = (img.frame_index as f32 * (off.x as f32 + 1.0) + (image_width as f32 / 2.0)) as i32;
                let ty = (img.layer_index as f32 * (off.y as f32 + 1.0) - (image_height as f32 / 2.0) + 16.) as i32 * -1;
                
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
                        x: (img.bounds.x + (img.frame_index as f32 * off.x as f32)) * scale_x as f32,
                        y: (img.bounds.y - (img.layer_index as f32 * off.y as f32)) * scale_y as f32,
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
        }

        let line_alpha = (24. * cam.zoom).clamp(0., 255.) as u8;

        for i in 0..self.loaded_layers.len() {
            let l = &self.loaded_layers[i];
            let m = d.measure_text(&l.name, FONT_SIZE_REG);

            d.draw_text(
                &l.name,
                (16 + m) * -1,
                (off.y as i32 * i as i32 - (off.x / 2.0) as i32) * -1,
                FONT_SIZE_REG, LABEL_COLOR
            );
            
            let line_y = ((off.y + 1.0) as i32 * i as i32) * -1;
            let line_y2 = line_y + image_height as i32 * scale_y;
            
            d.draw_line(
                (16 + m) * -1, line_y,
                (16 + m) * -1 + (off.x * self.frame_count as f32) as i32, line_y,
                Color{a: line_alpha, ..SMALL_LINE_COLOR}
            );
            
            d.draw_line(
                (16 + m) * -1, line_y2,
                (16 + m) * -1 + (off.x * self.frame_count as f32) as i32, line_y2,
                Color{a: line_alpha, ..SMALL_LINE_COLOR}
            );

            d.draw_line_ex(
                Vector2{
                    x: ((16 + m) * -1) as f32,
                    y: line_y2 as f32 + (GAP / 2) as f32 + 0.5,
                }, Vector2{
                    x: ((16 + m) * -1) as f32 + (off.x * self.frame_count as f32),
                    y: line_y2 as f32 + (GAP / 2) as f32 + 0.5,
                }, 
                GAP as f32 + 1.,
                Color{a: line_alpha/4, ..BIG_LINE_COLOR}
            );
        }

        for i in 0..self.frame_count {
            let fstr = format!("{}", i);
            let fstr = fstr.as_str();

            let width = image_width as i32 - d.measure_text(fstr, FONT_SIZE_REG);
            let width = width / 2;

            d.draw_text(fstr,
                ((off.x + 1.0) * i as f32) as i32 + width,
                (off.y + 16.0) as i32,
                FONT_SIZE_REG,
                LABEL_COLOR
            );

            d.draw_text(fstr,
                ((off.x + 1.0) * i as f32) as i32 + width,
                (off.y * (self.loaded_layers.len() - 1) as f32 + 16.0) as i32 * -1,
                FONT_SIZE_REG,
                LABEL_COLOR
            );
            
            let line_x = ((off.x + 1.0) * (i as f32) as f32) as i32;
            let line_x2 = line_x + image_width as i32 * scale_x;

            d.draw_line(
                line_x, (off.y + 4.0) as i32, 
                line_x, (off.y * (self.loaded_layers.len() - 1) as f32 + 16.0) as i32 * -1,
                Color{a: line_alpha, ..SMALL_LINE_COLOR}
            );

            d.draw_line(
                line_x2, (off.y + 4.0) as i32, 
                line_x2, (off.y * (self.loaded_layers.len() - 1) as f32 + 16.0) as i32 * -1,
                Color{a: line_alpha, ..SMALL_LINE_COLOR}
            );

            d.draw_line_ex(
                Vector2{
                    x: (line_x2 + GAP as i32 / 2) as f32 + 0.5,
                    y: (off.y + 4.0),
                }, Vector2{
                    x: (line_x2 + GAP as i32 / 2) as f32 + 0.5,
                    y: (off.y * (self.loaded_layers.len() - 1) as f32 + 16.0) * -1.0,
                }, 
                GAP as f32 + 1.,
                Color{a: line_alpha/4, ..BIG_LINE_COLOR}
            );
        }
    }

    pub fn draw_ui(&mut self, d: &mut RaylibDrawHandle) {
        let dd_str = CString::new(
            <Vec<PreparedLayer> as AsRef<Vec<PreparedLayer>>>::as_ref(&self.loaded_layers)
            .into_iter().rev()
            .map(|i| {
                format!("{} {}",
                    if i.visible { "#44#" } else { "#45#" }, 
                    &i.name
                )
            }).collect::<Vec<String>>().join(";").as_str()).unwrap();
        
        let dd_str = dd_str.as_c_str();
        
        let _ = d.gui_list_view(
            Rectangle{
                x: 0.0,
                y: 0.0,
                width: 120.0,
                height: WINDOW_H as f32,
            },
            Some(dd_str), &mut self.layer_scroll, &mut self.layer_active
        );

        let effective_layer_active = (self.loaded_layers.len() - 1) - self.layer_active as usize;

        {
            let prop_bounds = Rectangle{
                x: 128.0,
                y: 0.0,
                width: 120.0,
                height: 110.0,
            };

            let layer_name = CString::new(self.loaded_layers[effective_layer_active].name.as_str()).unwrap();
            let layer_name = layer_name.as_c_str();

            let _ = d.gui_window_box(prop_bounds, Some(layer_name));
            
            let properties_contents = CString::new({
                let layer = &self.loaded_layers[effective_layer_active];

                format!("Blend mode: {}\nOpacity: {}{}{}",
                    layer.blend_mode.to_string(), 
                    layer.opacity, 
                    if layer.background {"\nIs a background"} else {""},
                    if layer.is_reference {"\nIs a reference"} else {""},
                )
            }).unwrap();
            let properties_contents = properties_contents.as_c_str();
            
            d.gui_label(Rectangle{
                x: prop_bounds.x + 4.0,
                y: prop_bounds.y + 24.0,
                width: prop_bounds.width,
                height: 72.0
            }, Some(properties_contents));

            d.gui_check_box(Rectangle{
                x: prop_bounds.x + 8.0,
                y: prop_bounds.y + prop_bounds.height - 28.0,
                width: 24.0,
                height: 24.0,
            }, Some(rstr!("Visible")), &mut self.loaded_layers[effective_layer_active].visible);
        }

    }

    pub fn step(&mut self, rl: &mut RaylibHandle, cam: &Camera2D) {
        let header = &self.main_data.header;
        let mouse_pt = rl.get_screen_to_world2D(rl.get_mouse_position(), cam);

        let scale_x: i32 = header.pixel_width.max(1).into();
        let scale_y: i32 = header.pixel_height.max(1).into();

        let off = Vector2{
            x: (header.width * scale_x as u16 + GAP) as f32,
            y: (header.height * scale_y as u16 + GAP) as f32
        };

        for img in &mut self.loaded_cels {
            let range = Rectangle{
                x: (img.frame_index as f32 * off.x as f32) * scale_x as f32,
                y: (img.layer_index as f32 * off.y as f32) * scale_y as f32 * -1.0,
                width: header.width as f32 * scale_x as f32,
                height: header.height as f32 * scale_y as f32,
            };

            img.hover = range.check_collision_point_rec(mouse_pt);
        }
    }
}