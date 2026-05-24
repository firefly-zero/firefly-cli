#![no_std]
#![no_main]
extern crate alloc;
use firefly_rust::*;

#[unsafe(no_mangle)]
extern "C" fn boot() {
    // ...
}

#[unsafe(no_mangle)]
extern "C" fn update() {
    // ...
}

#[unsafe(no_mangle)]
extern "C" fn render() {
    clear_screen(Color::White);
    draw_triangle(
        Point::new(60, 10),
        Point::new(40, 40),
        Point::new(80, 40),
        Style {
            fill_color: Color::LightGray,
            stroke_color: Color::DarkBlue,
            stroke_width: 1,
        },
    );
}
