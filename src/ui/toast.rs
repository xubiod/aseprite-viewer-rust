use raylib::{color::Color, math::Rectangle, prelude::{RaylibDraw, RaylibDrawHandle}, RaylibHandle};

use super::{ui_main::FONT_SIZE_REG, ui_traits::ExpirableElement};

pub struct Toast {
    text: String,
    timer: i32,
    start_timer: i32,

    background: Color,

    bounds: Rectangle
}

impl ExpirableElement for Toast {
    fn is_alive(&self) -> bool {
        self.timer > 0
    }
}

impl Toast {
    pub fn new(text: &str, timer: i32) -> Self {
        Self {
            text: String::from(text),
            start_timer: timer,
            timer,
            bounds: Rectangle { ..Default::default() },
            background: Color{a: 192, ..Color::BLACK}
        }
    }

    pub fn draw(&mut self, y_offset: f32, d: &mut RaylibDrawHandle, window_w: i32) {
        let w = d.measure_text(&self.text, FONT_SIZE_REG) as f32;
        let padding = 6.;
        self.bounds = Rectangle{
            x: window_w as f32 - (padding * 4.) - w - 1.,
            y: y_offset + 1.,
            width: w + padding * 4.,
            height: 10. + padding * 2.
        };

        d.draw_rectangle_rec(self.bounds, self.background);
        d.draw_rectangle_rec(Rectangle{
            x: self.bounds.x + 1.,
            y: self.bounds.y + self.bounds.height - 3.,
            width: self.bounds.width * (self.timer as f32 / self.start_timer as f32) - 2.,
            height: 2.
        }, Color::WHITESMOKE);

        // d.draw_text(
        //     format!("{0:.1}s", self.timer as f32 / 60.).as_str(), 
        //     (self.bounds.x + self.bounds.width) as i32 - 16,
        //     (self.bounds.y + self.bounds.height) as i32 - 5,
        //     5, Color{a: 127, ..Color::WHITE}
        // );
        
        d.draw_text(&self.text, (self.bounds.x + padding * 2.) as i32, (self.bounds.y + padding) as i32, FONT_SIZE_REG, Color::WHITE);
    }

    pub fn step(&mut self, rl: &RaylibHandle) {
        self.timer = self.timer - 1;
        
        if self.bounds.check_collision_point_rec(rl.get_mouse_position()) {
            self.timer = self.start_timer
        }
    }

    pub fn height(&self) -> f32 {
        self.bounds.height
    }
}