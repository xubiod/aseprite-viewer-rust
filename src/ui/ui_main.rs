use std::ffi::CString;
use std::io::{stderr, Write};

use raylib::prelude::*;
use raylib::{color::Color, math::Vector2};

use crate::ase::aseprite::AsepriteError;

use super::loaded_aseprite::{LoadedSprite, GAP};
use super::toast::Toast;
use super::ui_traits::ExpirableElement;

const MAX_ZOOM_OUT:    f32 = 20.00;
const MAX_ZOOM_IN:     f32 =  0.10;
const ZOOM_LERP_SPEED: f32 =  0.4;

const SCROLL_SENSITIVITY: f32 = 10.0;

pub(crate) const FONT_SIZE_REG: i32 = 10;
pub(crate) const FONT_SIZE_BIG: i32 = FONT_SIZE_REG * 2;

const WORKSPACE_CLEAR_COLOUR: Color = Color{r: 8, g: 8, b: 8, a: 255};

/// The distance the GUI icons for signifying resizablity on the layer list should
/// be from center.
const LAYER_RESIZE_ICON_SPREAD: f32   = 8.0;
/// The colour of the resizing indicator and arrows for the layer list.
const LAYER_RESIZE_COLOUR:      Color = Color::ORANGERED;

const TOAST_COLOR_ERROR: Color = Color::MAROON;

// struct Part {
//     pos: Vector2,
//     spd: f32
// }

#[derive(Default)]
pub struct UIState {
    loaded_sprite: Option<LoadedSprite>,
    toasts:        Vec<Toast>,

    desired_zoom: f32,
    fit_zoom:     f32,

    desired_position: Vector2,
    default_position: Vector2,

    show_zoom_reset: bool,

    pub window_w: i32,
    pub window_h: i32,

    // particles: Vec<Part>,

    pub layer_list_visible: bool,
    layer_list_width:       f32,
    layer_list_resizing:    bool,
    layer_list_scroll:      i32,
    layer_list_active:      i32,
}

const ACCEPTED_TYPES: [&str; 2] = [".ase", ".aseprite"];

pub(crate) const WINDOW_W: i32 = 1200;
pub(crate) const WINDOW_H: i32 = 800;

pub fn ui() {
    let (mut rl, thread) = raylib::init()
        .size(WINDOW_W, WINDOW_H)
        .title("ui")
        // .vsync()
        .build();

    let mut state = UIState{
        desired_zoom: 1.,
        desired_position: Vector2{x: 0., y: 0.},
        window_w: WINDOW_W,
        window_h: WINDOW_H,

        layer_list_active: -1,
        layer_list_width: 120.0,
        ..Default::default()
    };

    // let mut rng = rand::thread_rng();

    // state.particles.reserve(10_000);
    // for _ in 0..state.particles.capacity() {
    //     state.particles.push(
    //         Part{
    //             pos: Vector2{
    //                 x: rng.gen_range(-WINDOW_W as f32/MAX_ZOOM_IN..WINDOW_W as f32/MAX_ZOOM_IN),
    //                 y: rng.gen_range(-WINDOW_H as f32/MAX_ZOOM_IN..WINDOW_H as f32/MAX_ZOOM_IN)
    //             },
    //             spd: rng.gen_range(0.1..3.0)
    //         }
    //     );
    // }

    rl.set_target_fps(60);

    let mut cam = Camera2D {
        zoom: 1.0,
        offset: Vector2{x: (state.window_w/2) as f32, y: (state.window_h/2) as f32},
        ..Default::default()
    };

    while !rl.window_should_close() {
        // update
        {
            if rl.is_file_dropped() {
                let list = rl.load_dropped_files();

                'path: for fname in list.paths() {
                    for ext in ACCEPTED_TYPES {
                        if rl.is_file_extension(fname, ext) {

                            match LoadedSprite::load(fname, &mut rl, &thread) {
                                Ok(new) => {
                                    state.layer_list_visible = state.loaded_sprite.is_none() || state.layer_list_visible;
                                    
                                    state.default_position = Vector2{
                                        x: (new.frame_count + GAP as usize) as f32 * new.pixel_width as f32 * new.image_width as f32,
                                        y: (new.loaded_layers.len() + GAP as usize) as f32 * new.pixel_height as f32 * new.image_height as f32,
                                    };
                                    
                                    state.default_position *= 0.5;
                                    state.default_position.y *= -1.0;
                                    
                                    state.desired_position = state.default_position;
                                    
                                    state.toasts.push(
                                        Toast::new(
                                            {
                                                format!(
                                                    "file loaded successfully; {} cels, {} frames, {} layers",
                                                    new.loaded_cels.len(),
                                                    new.loaded_layers.len(),
                                                    new.frame_count,
                                                ).as_str()
                                            },
                                            180
                                        )
                                    );

                                    state.loaded_sprite = Some(new);
        
                                    break 'path
                                },
                                Err(e) => {
                                    match e {
                                        AsepriteError::RanOutAtHeader => {
                                            state.toasts.push(Toast::new_ex(
                                                "file error! too small to have header",
                                                210,
                                                TOAST_COLOR_ERROR
                                            ));
                                        },
                                        AsepriteError::HeaderMagicMismatch | AsepriteError::FrameMagicMismatch => {
                                            state.toasts.push(Toast::new_ex(
                                                "file error! corrupted data!",
                                                210,
                                                TOAST_COLOR_ERROR
                                            ));
                                        },
                                        AsepriteError::Other(error) => {
                                            state.toasts.push(Toast::new_ex(
                                                "unknown error, check error output for details",
                                                240,
                                                TOAST_COLOR_ERROR
                                            ));

                                            let _ = stderr().write_all(error.to_string().as_bytes());
                                        },
                                    }
                                },
                            };
                        }
                    }
                }
            }

            state.desired_zoom += rl.get_mouse_wheel_move() / SCROLL_SENSITIVITY;
            state.desired_zoom = state.desired_zoom.clamp(MAX_ZOOM_IN, MAX_ZOOM_OUT);
            
            cam.zoom += (state.desired_zoom - cam.zoom) * ZOOM_LERP_SPEED;
            
            if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
                state.desired_position -= rl.get_mouse_delta() / cam.zoom;
                
                // for part in &mut state.particles {
                //     part.pos += rl.get_mouse_delta() / (cam.zoom * part.spd * 2.);
                // }
            }
                
            cam.target += (state.desired_position - cam.target) * 0.8;
            
            for toast in &mut state.toasts {
                toast.step(&rl);
            }

            if let Some(loaded) = &mut state.loaded_sprite {
                loaded.step(&mut rl, &cam);
            }
            
            state.toasts.retain(|i| i.is_alive());
        }

        // draw
        {
            let mut d = rl.begin_drawing(&thread);
            // d.clear_background(Color::RAYWHITE);
            d.clear_background(WORKSPACE_CLEAR_COLOUR);

            // draw cameraspace
            {
                let mut d = d.begin_mode2D(cam);

                let left = cam.target.x - (cam.offset.x) / MAX_ZOOM_IN;
                let width = (state.window_w as f32) / MAX_ZOOM_IN;

                let top = cam.target.y - (cam.offset.y) / MAX_ZOOM_IN;
                let height = (state.window_h as f32) / MAX_ZOOM_IN;

                d.draw_rectangle_lines_ex(Rectangle{
                    x: left, y: top, width, height
                }, MAX_ZOOM_OUT, Color::RED);

                // for part in &state.particles {
                //     let px = wrap(part.pos.x - (cam.target.x * part.spd), left, left + width);
                //     let py = wrap(part.pos.y - (cam.target.y * part.spd), top, top + height);

                //     d.draw_circle(px as i32, py as i32, part.spd / cam.zoom, Color{ a: 40, ..Color::WHITE });
                // }
                
                if let Some(ref mut spr) = state.loaded_sprite {
                    spr.draw(&mut d, &cam);
                }
            }

            // draw screenspace
            {
                match state.loaded_sprite {
                    Some(_) => { layer_list(&mut d, &mut state); },
                    None => {
                        let tx = "drag and drop an aseprite file..";
                        let tx_w = d.measure_text(tx,FONT_SIZE_BIG);
                        d.draw_text(&tx, (state.window_w - tx_w)/2, (state.window_h/2)-12, FONT_SIZE_BIG, Color::RAYWHITE);
                    },
                };

                let mut toast_y = 0.0;
                for toast in &mut state.toasts {
                    toast.draw( toast_y, &mut d, state.window_w);
                    toast_y += toast.height() + 4.
                }

                bottom_bar(&mut d, &mut state, &cam);
            }
        }
    }
}

// fn wrap(x: f32, lo: f32, hi: f32) -> f32 {
//     let mut x = x;
//     let mut lo1 = lo;
//     let mut hi1 = hi;
//     let mut subby = 0.;

//     if lo1 < 0. {
//         lo1 += lo.abs();
//         hi1 += lo.abs();
//         subby = lo.abs();
//     }

//     let size = hi1 - lo1 + 1.;

//     if x < lo1 {
//         x += size * ((lo1 - x) / size + 1.);
//     }

//     (lo1 + (x - lo1) % size) - subby
// }

fn label_wrapper(d: &mut RaylibDrawHandle, bounds: impl Into<ffi::Rectangle>, text: &str, is_btn: bool) -> bool {
    let lbl_str = CString::new(text).unwrap();
    let lbl_str = lbl_str.as_c_str();
    if is_btn {
        d.gui_label_button(bounds, Some(lbl_str))
    } else {
        d.gui_label(bounds, Some(lbl_str))
    }
}

fn layer_list(d: &mut RaylibDrawHandle, state: &mut UIState) {
    if let Some(ref mut loaded) = state.loaded_sprite {
        if state.layer_list_visible {
            let dd_str = loaded.generate_layer_list().as_c_str();

            let layer_list_rec = Rectangle{
                x: 0.0,
                y: 0.0,
                width: state.layer_list_width,
                height: WINDOW_H as f32,
            };

            let _ = d.gui_list_view(
                layer_list_rec, Some(dd_str), &mut state.layer_list_scroll, &mut state.layer_list_active
            );

            let resize_area = Rectangle{
                x: layer_list_rec.width - 8.0,
                width: 16.0,
                ..layer_list_rec
            };

            let lo_resize_bound: f32 = 90.0;
            let hi_resize_bound: f32 = d.get_screen_width() as f32 - 128.0;

            let m = d.get_mouse_position();
            if resize_area.check_collision_point_rec(m) || state.layer_list_resizing {
                d.draw_line_ex(Vector2{
                    x: resize_area.x + resize_area.width / 2.,
                    y: resize_area.y
                }, Vector2{
                    x: resize_area.x + resize_area.width / 2.,
                    y: resize_area.y + resize_area.height
                }, resize_area.width / 4., LAYER_RESIZE_COLOUR);

                unsafe {
                    let arrows_height = (resize_area.height / 2.) as i32;

                    if state.layer_list_width > lo_resize_bound {
                        ffi::GuiDrawIcon(118, (resize_area.x - LAYER_RESIZE_ICON_SPREAD) as i32, arrows_height, 1, LAYER_RESIZE_COLOUR.into());
                    }
                    if state.layer_list_width < hi_resize_bound {
                        ffi::GuiDrawIcon(119, (resize_area.x + LAYER_RESIZE_ICON_SPREAD) as i32, arrows_height, 1, LAYER_RESIZE_COLOUR.into());
                    }
                };

                state.layer_list_resizing = d.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);

                if state.layer_list_resizing {
                    state.layer_list_width = m.x.clamp(lo_resize_bound, hi_resize_bound)
                }
            }

            if state.layer_list_active >= 0 && (state.layer_list_active as usize) < loaded.loaded_layers.len() {
                let effective_layer_active = (loaded.loaded_layers.len() - 1) - (state.layer_list_active as usize);
                let prop_bounds = Rectangle{
                    x: state.layer_list_width + 8.,
                    y: 8.0,
                    width: 120.0,
                    height: 130.0,
                };

                let layer_name = CString::new(loaded.loaded_layers[effective_layer_active].name.as_str()).unwrap();
                let layer_name = layer_name.as_c_str();

                if d.gui_window_box(prop_bounds, Some(layer_name)) {
                    state.layer_list_active = -1;
                }
                
                let layer = &loaded.loaded_layers[effective_layer_active];
                let properties_contents = rstr!(
                    "Blend mode: {}\nOpacity: {}{}{}",
                    layer.blend_mode.to_string(), 
                    layer.opacity, 
                    if layer.background {"\nIs a background"} else {"\n"},
                    if layer.is_reference {"\nIs a reference"} else {"\n"},
                );
                
                d.gui_label(Rectangle{
                    x: prop_bounds.x + 4.0,
                    y: prop_bounds.y + 24.0,
                    width: prop_bounds.width,
                    height: 72.0
                }, Some(properties_contents.as_c_str()));

                if d.gui_check_box(Rectangle{
                    x: prop_bounds.x + 8.0,
                    y: prop_bounds.y + prop_bounds.height - 28.0,
                    width: 24.0,
                    height: 24.0,
                }, Some(rstr!("Visible")), &mut loaded.loaded_layers[effective_layer_active].visible) {
                    loaded.invalidate_layer_list();
                }
            }
        }
    }
}

fn bottom_bar(d: &mut RaylibDrawHandle, state: &mut UIState, cam: &Camera2D) {
    d.gui_panel(Rectangle{x: 0., y: (state.window_h - 24) as f32, width: state.window_w as f32, height: 24.}, None);

    {
        let bounds = Rectangle{x: 0., y: (state.window_h - 24) as f32, width: 24., height: 24.};
        match state.loaded_sprite {
            Some(_) => {
                if label_wrapper(d, bounds, if state.layer_list_visible { "#197#" } else { "#196#" }, true) {
                    state.layer_list_visible ^= true;
                }
            },
            None => { label_wrapper(d, bounds,  "#196#", false); },
        };
    }

    if label_wrapper(d, Rectangle{x: 28., y: (state.window_h - 24) as f32, width: 90., height: 24.},
                     format!("#43# {0:.2}%", cam.zoom * 100.).as_str(), true) {
        state.show_zoom_reset ^= true;
    }

    if state.show_zoom_reset {
        let rect = Rectangle{x: 28., y: (state.window_h - 72) as f32, width: 65., height: 24.};

        if d.gui_button(rect, Some(rstr!("#43# fit"))) {
            state.desired_zoom = state.fit_zoom;
            state.show_zoom_reset = false
        }
        if d.gui_button(Rectangle{y: rect.y + rect.height, ..rect}, Some(rstr!("#42# 100%"))) {
            state.desired_zoom = 1.;
            state.show_zoom_reset = false
        }
    }

    {
        let recenter = Rectangle{x: 112., y: (state.window_h - 24) as f32, width: 90., height: 24.};
        let t = format!("#48# {0:.0}, {1:.0}", cam.target.x, cam.target.y);
        let recenter_tx = if recenter.check_collision_point_rec(d.get_mouse_position()) {
            "#48# recenter?"
        } else {
            t.as_str() 
        };
        
        if label_wrapper(d, recenter, recenter_tx, true) {
            state.desired_position = state.default_position;
        }
    }
}