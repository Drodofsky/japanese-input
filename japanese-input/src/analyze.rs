use core::ops::Range;

use kurbo::{Affine, BezPath, Rect};

use crate::{
    KanjiMap,
    analyzed_kanji_node::AnalyzedKanjiNode,
    bbox::BBox as _,
    convert_stroke_index::ConvertStrokeIndex as _,
    gen_svg::{
        gen_batsu_remove_strokes, gen_batsu_stroke_order, gen_maru_add_strokes,
        gen_maru_stroke_order,
    },
    map_space_to::MapSpaceTo as _,
    match_strokes::{Weights, match_strokes},
    stroke_geometry::StrokeGeometry,
    stroke_point::StrokePoint,
    to_bez_path::ToBezPathVec as _,
    transform::Transform as _,
};

const MOVE_THRESHOLD: f64 = 0.3;

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum AnalyzeResult {
    StrokeOrder { correct: String, wrong: String },
    ExtraOrMissingStrokes { correct: String, wrong: String },
    StrokePositions { correct: String, wrong: String },
    NoError,
    WrongDrawn { correct: String, wrong: String },
}

impl AnalyzeResult {
    #[must_use]
    #[inline]
    pub fn correct(&self) -> Option<&str> {
        match &self {
            AnalyzeResult::ExtraOrMissingStrokes { correct, .. }
            | AnalyzeResult::StrokeOrder { correct, .. }
            | AnalyzeResult::StrokePositions { correct, .. }
            | AnalyzeResult::WrongDrawn { correct, .. } => Some(correct),
            AnalyzeResult::NoError => None,
        }
    }
    #[must_use]
    #[inline]
    pub fn wrong(&self) -> Option<&str> {
        match &self {
            AnalyzeResult::ExtraOrMissingStrokes { wrong, .. }
            | AnalyzeResult::StrokeOrder { wrong, .. }
            | AnalyzeResult::StrokePositions { wrong, .. }
            | AnalyzeResult::WrongDrawn { wrong, .. } => Some(wrong),
            AnalyzeResult::NoError => None,
        }
    }
}

pub struct Analyzer {
    kanji_map: KanjiMap,
}

impl Analyzer {
    #[inline]
    #[must_use]
    pub fn new(kanji_map: KanjiMap) -> Self {
        Self { kanji_map }
    }
    #[must_use]
    #[inline]
    pub fn analyze_kanji(
        &self,
        kanji: char,
        mut user_strokes: Vec<Vec<StrokePoint>>,
        grid_color: &str,
        corner_radius: f32,
        stroke_color: &str,
    ) -> Option<AnalyzeResult> {
        let kanji_tree_raw = self.kanji_map.get(&kanji)?;
        let kanji_tree = kanji_tree_raw.clone().to_analyzed();

        let mut mapping = match_strokes(
            kanji_tree.clone(),
            user_strokes.clone(),
            Weights::default(),
            100,
        )
        .first()?
        .user_stroke_order
        .to_vec();

        let order_wrong = mapping
            .iter()
            .enumerate()
            .any(|(i, &u)| usize::from(u) != i);
        let user_stroke_count = user_strokes.len();
        let has_missing = mapping.contains(&u8::MAX);

        let mut keep = get_stroke_keep_list(&user_strokes, &mapping);
        let has_extra = keep.contains(&false);
        let mut highlight_added = vec![false; keep.len()];

        if has_missing {
            insert_missing_strokes(&kanji_tree, &mut user_strokes, &mut mapping);
            for _ in 0..user_strokes.len().saturating_sub(keep.len()) {
                keep.push(true);
                highlight_added.push(true);
            }
        }

        let pre_moved = user_strokes.clone();
        let markers = reposition_groups(&kanji_tree, &mut user_strokes, &mapping);
        let has_moved = !markers.is_empty();

        let batsu_maru = |wrong: &[BezPath],
                          w_keep: &[bool],
                          w_marks: &[Rect],
                          correct: &[BezPath],
                          c_keep: &[bool],
                          c_high: &[bool]| {
            (
                gen_batsu_remove_strokes(
                    grid_color,
                    corner_radius,
                    wrong,
                    stroke_color,
                    w_keep,
                    w_marks,
                ),
                gen_maru_add_strokes(
                    grid_color,
                    corner_radius,
                    correct,
                    stroke_color,
                    c_keep,
                    c_high,
                ),
            )
        };

        let res = if (has_extra || has_missing) && has_moved {
            let wrong = pre_moved.iter().take(user_stroke_count).to_bez_path_vec();
            let correct = kanji_tree_raw.collect_paths();
            let (wrong, correct) = batsu_maru(
                &wrong,
                &vec![true; wrong.len()],
                &[],
                &correct,
                &vec![true; correct.len()],
                &vec![false; correct.len()],
            );
            AnalyzeResult::WrongDrawn { correct, wrong }
        } else if has_moved {
            let wrong = pre_moved.iter().to_bez_path_vec();
            let correct = user_strokes.iter().to_bez_path_vec();
            let (wrong, correct) = batsu_maru(
                &wrong,
                &vec![true; wrong.len()],
                &markers,
                &correct,
                &vec![true; correct.len()],
                &vec![false; correct.len()],
            );
            AnalyzeResult::StrokePositions { correct, wrong }
        } else if has_extra || has_missing {
            let wrong = pre_moved.iter().take(user_stroke_count).to_bez_path_vec();
            let correct = pre_moved.iter().to_bez_path_vec();
            let (wrong, correct) =
                batsu_maru(&wrong, &keep, &[], &correct, &keep, &highlight_added);
            AnalyzeResult::ExtraOrMissingStrokes { correct, wrong }
        } else if order_wrong {
            let strokes = pre_moved.iter().to_bez_path_vec();
            AnalyzeResult::StrokeOrder {
                wrong: gen_batsu_stroke_order(grid_color, corner_radius, &strokes),
                correct: gen_maru_stroke_order(grid_color, corner_radius, &strokes, &mapping),
            }
        } else {
            AnalyzeResult::NoError
        };

        Some(res)
    }
}

fn get_stroke_keep_list(user_strokes: &[Vec<StrokePoint>], user_stroke_order: &[u8]) -> Vec<bool> {
    user_strokes
        .iter()
        .enumerate()
        .map(|(i, _)| user_stroke_order.contains(&i.convert_stroke_index()))
        .collect()
}

fn insert_missing_strokes(
    tree: &AnalyzedKanjiNode,
    user_strokes: &mut Vec<Vec<StrokePoint>>,
    mapping: &mut [u8],
) {
    let ref_strokes = tree.collect_strokes();
    let ref_geometry = tree.collect_geometry();
    let Some(seed) = ref_geometry
        .as_slice()
        .bbox()
        .zip(user_strokes.bbox())
        .and_then(|(r, d)| r.map_space_to(d))
    else {
        return;
    };
    insert_missing_rec(
        tree,
        0,
        seed,
        &ref_strokes,
        &ref_geometry,
        mapping,
        user_strokes,
    );
}

fn reposition_groups(
    tree: &AnalyzedKanjiNode,
    user_strokes: &mut [Vec<StrokePoint>],
    mapping: &[u8],
) -> Vec<Rect> {
    let ref_geometry = tree.collect_geometry();
    let mut marker_rects: Vec<Rect> = Vec::new();
    reposition_rec(
        tree,
        0,
        &ref_geometry,
        mapping,
        user_strokes,
        &mut marker_rects,
    );
    marker_rects
}

fn insert_missing_rec(
    node: &AnalyzedKanjiNode,
    start: usize,
    inherited: Affine,
    ref_strokes: &[Vec<StrokePoint>],
    ref_geometry: &[StrokeGeometry],
    mapping: &mut [u8],
    user_strokes: &mut Vec<Vec<StrokePoint>>,
) -> usize {
    match node {
        AnalyzedKanjiNode::Stroke { .. } => {
            if let Some(m) = mapping.get_mut(start)
                && *m == u8::MAX
                && let Some(r) = ref_strokes.get(start)
            {
                let placed = r.iter().map(|s| s.transform(inherited)).collect();
                let new_idx = user_strokes.len();
                user_strokes.push(placed);
                *m = new_idx.convert_stroke_index();
            }
            start.saturating_add(1)
        }
        AnalyzedKanjiNode::Group { children, .. } => {
            let end = start.saturating_add(node.leaf_count());
            let transform = range_bboxes(start..end, mapping, ref_geometry, user_strokes)
                .and_then(|(r, d)| r.map_space_to(d))
                .unwrap_or(inherited);
            let mut cursor = start;
            for child in children {
                cursor = insert_missing_rec(
                    child,
                    cursor,
                    transform,
                    ref_strokes,
                    ref_geometry,
                    mapping,
                    user_strokes,
                );
            }
            end
        }
    }
}

fn reposition_rec(
    node: &AnalyzedKanjiNode,
    start: usize,
    ref_geometry: &[StrokeGeometry],
    mapping: &[u8],
    user_strokes: &mut [Vec<StrokePoint>],
    marker_rects: &mut Vec<Rect>,
) -> usize {
    let AnalyzedKanjiNode::Group { children, .. } = node else {
        return start.saturating_add(1);
    };
    let end = start.saturating_add(node.leaf_count());

    if let Some((p_ref, p_drawn)) = range_bboxes(start..end, mapping, ref_geometry, user_strokes)
        && let Some(frame) = p_ref.map_space_to(p_drawn)
    {
        let p_extent = p_drawn.width().max(p_drawn.height());

        let mut cursor = start;
        for child in children {
            let child_end = cursor.saturating_add(child.leaf_count());
            if let Some((c_ref, c_drawn)) =
                range_bboxes(cursor..child_end, mapping, ref_geometry, user_strokes)
            {
                let target = frame.transform_rect_bbox(c_ref);
                let score = correction_score(c_drawn, target, p_extent);
                if let Some(mv) = c_drawn.map_space_to(target) {
                    move_range(cursor..child_end, mapping, user_strokes, mv);
                    if score > MOVE_THRESHOLD {
                        marker_rects.push(c_drawn);
                    }
                }
            }
            cursor = child_end;
        }
    }

    let mut cursor = start;
    for child in children {
        cursor = reposition_rec(
            child,
            cursor,
            ref_geometry,
            mapping,
            user_strokes,
            marker_rects,
        );
    }
    end
}

fn correction_score(current: Rect, target: Rect, parent_extent: f64) -> f64 {
    const MIN_PARENT_EXT: f64 = 0.25;
    let denom = parent_extent.max(MIN_PARENT_EXT);
    let offset = current.center().distance(target.center()) / denom;
    let scale = (current.width() - target.width())
        .abs()
        .max((current.height() - target.height()).abs())
        / denom;
    offset.max(scale)
}

fn matched_pairs(range: Range<usize>, mapping: &[u8]) -> impl Iterator<Item = (usize, usize)> {
    range.filter_map(move |r| {
        let m = *mapping.get(r)?;
        (m != u8::MAX).then_some((r, usize::from(m)))
    })
}
fn range_bboxes(
    range: Range<usize>,
    mapping: &[u8],
    ref_geometry: &[StrokeGeometry],
    user_strokes: &[Vec<StrokePoint>],
) -> Option<(Rect, Rect)> {
    let mut refs = Vec::new();
    let mut drawn = Vec::new();
    for (r, m) in matched_pairs(range, mapping) {
        if let Some(g) = ref_geometry.get(r) {
            refs.push(*g);
        }
        if let Some(s) = user_strokes.get(m) {
            drawn.push(s);
        }
    }
    Some((refs.as_slice().bbox()?, drawn.bbox()?))
}

fn move_range(
    range: Range<usize>,
    mapping: &[u8],
    user_strokes: &mut [Vec<StrokePoint>],
    t: Affine,
) {
    for (_, m) in matched_pairs(range, mapping) {
        if let Some(stroke) = user_strokes.get_mut(m) {
            for p in stroke.iter_mut() {
                *p = p.transform(t);
            }
        }
    }
}
