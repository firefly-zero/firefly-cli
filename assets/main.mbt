///|
using @firefly {type Point, type Size, type Style}

///|
fn main {

}

///|
pub fn boot() -> Unit {
  // ...
}

///|
pub fn update() -> Unit {
  // ...
}

///|
pub fn render() -> Unit {
  @firefly.draw_rect(
    Point::new(10, 10),
    Size::new(40, 15),
    Style::new_solid(LightGray),
  )
}
