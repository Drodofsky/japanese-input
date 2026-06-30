use colorous::{CATEGORY10, Color};
use kurbo::{Affine, BezPath, Rect};
use svg::{
    Document,
    node::element::{Circle, Line, Path, Rectangle, Text},
};

use crate::convert_stroke_index::ConvertStrokeIndex;

#[must_use]
#[inline]
pub fn gen_kanji_grid(grid_color: &str, corner_radius: f32) -> String {
    let mut doc = Document::new();
    doc = init_doc(doc);
    doc = draw_grid(doc, grid_color, corner_radius);
    doc.to_string()
}

#[must_use]
#[inline]
pub fn gen_kanji_grid_with_hint(
    grid_color: &str,
    corner_radius: f32,
    hint: &[BezPath],
    hint_color: &str,
) -> String {
    let mut doc = Document::new();
    doc = init_doc(doc);
    doc = draw_grid(doc, grid_color, corner_radius);
    doc = draw_hint(doc, hint, hint_color);
    doc.to_string()
}
#[must_use]
#[inline]
pub fn gen_batsu_remove_strokes(
    grid_color: &str,
    corner_radius: f32,
    user_strokes: &[BezPath],
    stroke_color: &str,
    draw_strokes: &[bool],
    markers: &[Rect],
) -> String {
    let mut doc = Document::new();
    doc = init_doc(doc);
    doc = draw_grid(doc, grid_color, corner_radius);
    doc = draw_batsu(doc);
    doc = draw_moved_markers(doc, markers);
    let scale = Affine::scale(109.0);
    for (path, color) in user_strokes.iter().zip(
        draw_strokes
            .iter()
            .map(|d| if *d { stroke_color } else { "#DC4A38" }),
    ) {
        let svg_path = scale_path(scale, path).to_svg();

        let element = Path::new()
            .set("d", svg_path)
            .set("fill", "none")
            .set("stroke", color)
            .set("stroke-width", 3.0_f64)
            .set("stroke-linecap", "round")
            .set("stroke-linejoin", "round");

        doc = doc.add(element);
    }

    doc.to_string()
}

#[must_use]
#[inline]
pub fn gen_maru_add_strokes(
    grid_color: &str,
    corner_radius: f32,
    user_strokes: &[BezPath],
    stroke_color: &str,
    draw_strokes: &[bool],
    highlight_strokes: &[bool],
) -> String {
    let mut doc = Document::new();
    doc = init_doc(doc);
    doc = draw_grid(doc, grid_color, corner_radius);
    doc = draw_maru(doc);
    let scale = Affine::scale(109.0);
    for (path, color) in user_strokes
        .iter()
        .zip(
            highlight_strokes
                .iter()
                .map(|d| if *d { "#2A8DC9" } else { stroke_color }),
        )
        .zip(draw_strokes.iter())
        .filter_map(|(data, drawn)| drawn.then_some(data))
    {
        let svg_path = scale_path(scale, path).to_svg();

        let element = Path::new()
            .set("d", svg_path)
            .set("fill", "none")
            .set("stroke", color)
            .set("stroke-width", 3.0_f64)
            .set("stroke-linecap", "round")
            .set("stroke-linejoin", "round");

        doc = doc.add(element);
    }

    doc.to_string()
}

#[must_use]
#[inline]
pub fn gen_maru_stroke_order(
    grid_color: &str,
    corner_radius: f32,
    user_strokes: &[BezPath],
    reference_order: &[u8],
) -> String {
    let mut doc = init_doc(Document::new());
    doc = draw_grid(doc, grid_color, corner_radius);
    doc = draw_maru(doc);
    doc = draw_stroke_order(doc, user_strokes, reference_order);
    doc.to_string()
}

#[must_use]
#[inline]
pub fn gen_batsu_stroke_order(
    grid_color: &str,
    corner_radius: f32,
    user_strokes: &[BezPath],
) -> String {
    let mut doc = init_doc(Document::new());
    doc = draw_grid(doc, grid_color, corner_radius);
    doc = draw_batsu(doc);
    let drawn_order: Vec<u8> = (0..user_strokes.len())
        .map(ConvertStrokeIndex::convert_stroke_index)
        .collect();
    doc = draw_stroke_order(doc, user_strokes, &drawn_order);
    doc.to_string()
}

fn draw_maru(mut doc: Document) -> Document {
    let side = 109.0_f64;
    let stroke_width = 20.0_f64;
    let margin = 25.0_f64;

    let center = side / 2.0_f64;
    let radius = center - margin;

    let circle = Circle::new()
        .set("cx", center)
        .set("cy", center)
        .set("r", radius)
        .set("fill", "none")
        .set("stroke", "#D0021B")
        .set("stroke-width", stroke_width)
        .set("opacity", 0.15_f64);

    doc = doc.add(circle);

    doc
}

fn draw_batsu(doc: Document) -> Document {
    let side = 109.0_f64;
    let stroke_width = 20.0_f64;
    let margin = 25.0_f64;

    let lo = margin;
    let hi = side - margin;
    let d = format!("M{lo},{lo} L{hi},{hi} M{hi},{lo} L{lo},{hi}");

    let cross = Path::new()
        .set("d", d)
        .set("fill", "none")
        .set("stroke", "#D0021B")
        .set("stroke-width", stroke_width)
        .set("stroke-linecap", "round")
        .set("opacity", 0.15_f64);

    doc.add(cross)
}

fn draw_hint(mut doc: Document, paths: &[BezPath], hint_color: &str) -> Document {
    let scale = Affine::scale(109.0);
    for path in paths {
        let svg_path = scale_path(scale, path).to_svg();

        let element = Path::new()
            .set("d", svg_path)
            .set("fill", "none")
            .set("stroke", hint_color)
            .set("stroke-width", 5.0_f64);

        doc = doc.add(element);
    }

    doc
}

fn draw_grid(mut doc: Document, grid_color: &str, corner_radius: f32) -> Document {
    let side = 109.0_f64;
    let stroke_width = 3.0_f64;
    let half = stroke_width / 2.0_f64;

    let origin_x = 0.0_f64;
    let origin_y = 0.0_f64;

    let border = Rectangle::new()
        .set("x", origin_x + half)
        .set("y", origin_y + half)
        .set("width", side - stroke_width)
        .set("height", side - stroke_width)
        .set("fill", "none")
        .set("stroke", grid_color)
        .set("stroke-width", stroke_width)
        .set("stroke-linejoin", "miter")
        .set("rx", corner_radius)
        .set("ry", corner_radius);

    doc = doc.add(border);

    let mid_x = origin_x + side / 2.0_f64;
    let mid_y = origin_y + side / 2.0_f64;

    let dot_size = 1.0_f64;
    let segment = dot_size * 2.0_f64;
    let dasharray = format!("{segment},{segment}");

    let center = (mid_x, mid_y);
    let endpoints = [
        (center, (origin_x + side, mid_y)), // center -> right
        (center, (origin_x, mid_y)),        // center -> left
        (center, (mid_x, origin_y)),        // center -> top
        (center, (mid_x, origin_y + side)), // center -> bottom
    ];

    for (a, b) in endpoints {
        let line = Line::new()
            .set("x1", a.0)
            .set("y1", a.1)
            .set("x2", b.0)
            .set("y2", b.1)
            .set("stroke", grid_color)
            .set("stroke-width", dot_size)
            .set("stroke-dasharray", dasharray.clone())
            .set("stroke-linecap", "round");

        doc = doc.add(line);
    }

    doc
}

fn init_doc(mut doc: Document) -> Document {
    doc = doc.set("width", 109_u32);
    doc = doc.set("height", 109_u32);
    doc = doc.set("viewBox", (0_u32, 0_u32, 109_u32, 109_u32));
    doc
}

fn draw_stroke_order(mut doc: Document, user_strokes: &[BezPath], order: &[u8]) -> Document {
    let scale = Affine::scale(109.0);
    for (seq, &idx) in order.iter().enumerate() {
        let Some(path) = user_strokes.get(usize::from(idx)) else {
            continue;
        };
        let c = get_color(seq);
        let color = format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b);

        let scaled = scale_path(scale, path);
        let element = Path::new()
            .set("d", scaled.to_svg())
            .set("fill", "none")
            .set("stroke", color.clone())
            .set("stroke-width", 3.0_f64)
            .set("stroke-linecap", "round")
            .set("stroke-linejoin", "round");
        doc = doc.add(element);

        if let Some(start) = scaled.elements().first().and_then(|el| {
            if let kurbo::PathEl::MoveTo(p) = el {
                Some(*p)
            } else {
                None
            }
        }) {
            let label = Text::new((seq.saturating_add(1)).to_string())
                .set("x", start.x - 2.0_f64)
                .set("y", start.y - 2.0_f64)
                .set("fill", color)
                .set("font-size", 8.0_f64)
                .set("text-anchor", "end");
            doc = doc.add(label);
        }
    }
    doc
}

fn draw_moved_markers(mut doc: Document, rects: &[Rect]) -> Document {
    for r in rects {
        let x = f64::max(r.x0 * 109.0 - 2.0, 0.0);
        let y = f64::max(r.y0 * 109.0 - 2.0, 0.0);
        let w = f64::min((r.x1 - r.x0) * 109.0 + 4.0, 109.0);
        let h = f64::min((r.y1 - r.y0) * 109.0 + 4.0, 109.0);
        let marker = Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", w)
            .set("height", h)
            .set("fill", "#D98A00")
            .set("opacity", 0.3_f64)
            .set("rx", 4.0_f64)
            .set("ry", 4.0_f64);
        doc = doc.add(marker);
    }
    doc
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "The Affine transform is applied to the path, not fallible integer arithmetic"
)]
fn scale_path(scale: Affine, path: &BezPath) -> BezPath {
    scale * path
}

#[expect(
    clippy::integer_division_remainder_used,
    reason = "not security/constant-time sensitive"
)]
#[expect(
    clippy::indexing_slicing,
    reason = "index is `% len`, always in bounds"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "remainder by non-zero const len can't panic"
)]
fn get_color(index: usize) -> Color {
    CATEGORY10[index % CATEGORY10.len()]
}

#[cfg(test)]
mod tests {
    #[derive(Deserialize, Serialize)]
    pub struct StrokeFile {
        pub character: char,
        pub strokes: Vec<Vec<(f32, f32)>>,
    }

    use kurbo::BezPath;
    use serde::{Deserialize, Serialize};

    use crate::{
        KanjiMap,
        gen_svg::{gen_batsu_remove_strokes, gen_kanji_grid, gen_kanji_grid_with_hint},
        to_bez_path::ToBezPath,
    };

    #[test]
    fn draw_grid() {
        let grid = gen_kanji_grid("#808080ff", 0.0);
        assert_eq!(
            grid.as_str(),
            include_str!("../../data/test/kanji_grid.svg")
        )
    }
    #[test]
    fn draw_grid_round_corners() {
        let grid = gen_kanji_grid("#808080ff", 8.0);
        assert_eq!(
            grid.as_str(),
            include_str!("../../data/test/kanji_grid_round_corners.svg")
        )
    }
    #[test]
    fn draw_grid_with_hint() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/generated/reference_data.bin");
        let bytes = std::fs::read(path).expect("failed to read reference_data.bin");
        let map: KanjiMap = postcard::from_bytes(&bytes).expect("failed to deserialize kanji map");
        let hint_path = map.get(&'食').unwrap().collect_paths();
        let grid = gen_kanji_grid_with_hint("#808080ff", 8.0, &hint_path, "darkgrey");
        println!("{}", grid);
        assert_eq!(
            grid.as_str(),
            include_str!("../../data/test/kanji_grid_with_hint.svg")
        )
    }
    #[test]
    fn draw_kanji_with_too_many_strokes() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/test/三_p1_wo.bin");
        let bytes = std::fs::read(path).expect("failed to read reference_data.bin");
        let file: StrokeFile =
            postcard::from_bytes(&bytes).expect("failed to deserialize stroke file");
        let user_strokes: Vec<BezPath> = file.strokes.iter().map(|s| s.to_bez_path()).collect();
        let drawn_strokes = [true, false, true, true];
        let markers = Vec::new();
        let svg = gen_batsu_remove_strokes(
            "#808080ff",
            8.0,
            &user_strokes,
            "darkgrey",
            drawn_strokes.as_slice(),
            &markers,
        );

        assert_eq!(
            svg,
            include_str!("../../data/test/kanji_batsu_to_many_strokes.svg")
        );
    }
}
