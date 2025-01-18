use rgeometry_demo::{define_schema, render, setup_panic_hook};

define_schema!(
    r#"
[
  {
    "type": "time"
  }
]"#
);

#[no_mangle]
pub extern "C" fn request_animation_frame(time: f64) {
    setup_panic_hook();
    if time == 0.0 {
        panic!();
    }
    let circle = svg::node::element::Circle::new()
        .set("cx", "50")
        .set("cy", "50")
        .set("r", "40")
        .set("fill", "red");

    let text = svg::node::element::Text::new(format!("{:.2}", time))
        .set("x", "50")
        .set("y", "50")
        .set("text-anchor", "middle")
        .set("dominant-baseline", "middle")
        .set("fill", "white");

    let document = svg::Document::new()
        .set("width", "100")
        .set("height", "100")
        .set("viewBox", "0 0 100 100")
        .add(circle)
        .add(text);

    render(document.to_string())
}
