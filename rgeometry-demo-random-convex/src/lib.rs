use rand::rngs::SmallRng;
use rand::SeedableRng;
use rgeometry::data::polygon::PolygonConvex;
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
    // Generate a new polygon every second
    let seed = (time as i32) / 1;
    let mut rng = SmallRng::seed_from_u64(seed as u64);

    // Generate a random convex polygon with 5-10 vertices
    let n_vertices = 5 + (seed % 6);
    let polygon = PolygonConvex::<i8>::random(n_vertices as usize, &mut rng);

    // Convert polygon points to SVG path
    let points = polygon.iter_boundary().map(|p| {
        (*p.x_coord() as f64 + 500.0, *p.y_coord() as f64 + 50.0) // Center the polygon in the 100x100 viewport
    });

    // Create SVG path
    let mut path_data = String::new();
    for (i, (x, y)) in points.enumerate() {
        if i == 0 {
            path_data.push_str(&format!("M {:.1} {:.1}", x, y));
        } else {
            path_data.push_str(&format!(" L {:.1} {:.1}", x, y));
        }
    }
    path_data.push('Z');

    let polygon_path = svg::node::element::Path::new()
        .set("d", path_data)
        .set("fill", "red")
        .set("stroke", "black")
        .set("stroke-width", "1");

    let document = svg::Document::new()
        .set("width", "1000")
        .set("height", "100")
        .set("viewBox", "0 0 1000 100")
        .add(polygon_path);

    render(document.to_string())
}
