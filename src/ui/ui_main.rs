use std::ffi::CString;

use rand::Rng;
use raylib::prelude::*;
use raylib::{color::Color, math::Vector2};

use super::loaded_aseprite::{LoadedSprite, GAP};
use super::toast::Toast;
use super::ui_traits::ExpirableElement;

const MAX_ZOOM_OUT: f32 = 20.00;
const MAX_ZOOM_IN: f32 = 0.10;

pub(crate) const FONT_SIZE_REG: i32 = 10;
pub(crate) const FONT_SIZE_BIG: i32 = FONT_SIZE_REG * 2;

struct Part {
    pos: Vector2,
    spd: f32
}

#[derive(Default)]
pub struct UIState {
    loaded_sprite: Option<LoadedSprite>,
    toasts: Vec<Toast>,

    desired_zoom: f32,
    fit_zoom: f32,

    desired_position: Vector2,
    default_position: Vector2,

    show_zoom_reset: bool,

    pub window_w: i32,
    pub window_h: i32,

    particles: Vec<Part>
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
        ..Default::default()
    };

    let mut rng = rand::thread_rng();

    state.particles.reserve(10_000);
    for _ in 0..state.particles.capacity() {
        state.particles.push(
            Part{
                pos: Vector2{
                    x: rng.gen_range(-WINDOW_W as f32/MAX_ZOOM_IN..WINDOW_W as f32/MAX_ZOOM_IN),
                    y: rng.gen_range(-WINDOW_H as f32/MAX_ZOOM_IN..WINDOW_H as f32/MAX_ZOOM_IN)
                },
                spd: rng.gen_range(0.1..3.0)
            }
        );
    }

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

                for fname in list.paths() {
                    for ext in ACCEPTED_TYPES {
                        if rl.is_file_extension(fname, ext) {
                            let new = LoadedSprite::load(
                                fname,
                                &mut rl,
                                &thread
                            );

                            state.loaded_sprite = new.ok();

                            let img_ref = state.loaded_sprite.as_ref().unwrap();

                            state.default_position = Vector2{
                                x: (img_ref.frame_count + GAP as usize) as f32 * img_ref.main_data.header.pixel_width as f32 * img_ref.main_data.header.width as f32,
                                y: (img_ref.loaded_layers.len() + GAP as usize) as f32 * img_ref.main_data.header.pixel_height as f32 * img_ref.main_data.header.height as f32,
                            };

                            state.default_position *= 0.5;
                            state.default_position.y *= -1.0;

                            state.desired_position = state.default_position;

                            state.toasts.push(
                                Toast::new(
                                {
                                        format!(
                                            "file loaded successfully; {} cels, {} frames, {} layers",
                                            img_ref.loaded_cels.len(),
                                            img_ref.loaded_layers.len(),
                                            img_ref.frame_count,
                                        ).as_str()
                                    },
                                    180
                                )
                            );

                            break
                        }
                    }
                }
            }

            state.desired_zoom += rl.get_mouse_wheel_move() / 10.;
            state.desired_zoom = state.desired_zoom.clamp(MAX_ZOOM_IN, MAX_ZOOM_OUT);
            
            cam.zoom += (state.desired_zoom - cam.zoom) * 0.4;
            
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
            
            state.toasts.retain(|i: &Toast       | i.is_alive());
        }

        // draw
        {
            let mut d = rl.begin_drawing(&thread);
            // d.clear_background(Color::RAYWHITE);
            d.clear_background(Color{r: 8, g: 8, b: 8, a: 255});

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

                for part in &state.particles {
                    let px = wrap(part.pos.x - (cam.target.x * part.spd), left, left + width);
                    let py = wrap(part.pos.y - (cam.target.y * part.spd), top, top + height);

                    d.draw_circle(px as i32, py as i32, part.spd / cam.zoom, Color{ a: 40, ..Color::WHITE });
                }
                
                if let Some(ref mut spr) = state.loaded_sprite {
                    spr.draw(&mut d, &cam);
                }
            }

            // draw screenspace
            {
                match state.loaded_sprite {
                    Some(ref mut img) => img.draw_ui(&mut d),
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

fn wrap(x: f32, lo: f32, hi: f32) -> f32 {
    let mut x = x;
    let mut lo1 = lo;
    let mut hi1 = hi;
    let mut subby = 0.;

    if lo1 < 0. {
        lo1 += lo.abs();
        hi1 += lo.abs();
        subby = lo.abs();
    }

    let size = hi1 - lo1 + 1.;

    if x < lo1 {
        x += size * ((lo1 - x) / size + 1.);
    }

    (lo1 + (x - lo1) % size) - subby
}

fn label_wrapper(d: &mut RaylibDrawHandle, bounds: impl Into<ffi::Rectangle>, text: &str, is_btn: bool) -> bool {
    let lbl_str = CString::new(text).unwrap();
    let lbl_str = lbl_str.as_c_str();
    if is_btn {
        d.gui_label_button(bounds, Some(lbl_str))
    } else {
        d.gui_label(bounds, Some(lbl_str))
    }
}

fn bottom_bar(d: &mut RaylibDrawHandle, state: &mut UIState, cam: &Camera2D) {
    d.gui_panel(Rectangle{x: 0., y: (state.window_h - 24) as f32, width: state.window_w as f32, height: 24.}, None);

    match state.loaded_sprite {
        Some(ref mut img) => {
            if label_wrapper(d, Rectangle{x: 0., y: (state.window_h - 24) as f32, width: 24., height: 24.}, if img.layer_list_visible { "#197#" } else { "#196#" }, true) {
                img.layer_list_visible ^= true;
            }
        },
        None => { label_wrapper(d, Rectangle{x: 0., y: (state.window_h - 24) as f32, width: 24., height: 24.},  "#196#", false); },
    };

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