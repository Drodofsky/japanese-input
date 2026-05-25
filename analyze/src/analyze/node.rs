use crate::KanjiNode;
use crate::normalize::Normalize;
use crate::point::{OrientedPoint, ToOriented};

#[derive(Debug, Clone)]
pub enum AnalyzedKanjiNode {
    Group {
        element: Option<char>,
        children: Vec<AnalyzedKanjiNode>,
    },
    Stroke {
        index: u8,
        in_kanji_frame: Vec<OrientedPoint>,
        in_stroke_frame: Vec<OrientedPoint>,
    },
}

impl AnalyzedKanjiNode {
    #[must_use]
    pub fn from_node(node: &KanjiNode) -> AnalyzedKanjiNode {
        let mut raw_strokes: Vec<Vec<OrientedPoint>> = Vec::new();
        let shadow = walk(node, &mut raw_strokes);

        let kanji_frame = raw_strokes.clone().normalize();
        let stroke_frame: Vec<Vec<OrientedPoint>> =
            raw_strokes.into_iter().map(Normalize::normalize).collect();

        materialize(&shadow, &kanji_frame, &stroke_frame)
    }

    #[must_use]
    pub fn len(&self) -> u8 {
        match self {
            AnalyzedKanjiNode::Stroke { index, .. } => index + 1,
            AnalyzedKanjiNode::Group { children, .. } => {
                children.last().map_or(0, AnalyzedKanjiNode::len)
            }
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Pre-analyzes every character in `map`, returning a `(char, AnalyzedKanjiNode)` pair
    /// for each entry. Used by recognizers to build their candidate list once at startup.
    #[must_use]
    pub fn preprocess_map(map: &crate::KanjiMap) -> Vec<(char, AnalyzedKanjiNode)> {
        map.iter()
            .map(|(&c, node)| (c, AnalyzedKanjiNode::from_node(node)))
            .collect()
    }
}

/// Intermediate tree that records the flat slot index each stroke maps to,
/// built during the first pass over the `KanjiNode` tree.
enum Shadow {
    Group {
        element: Option<char>,
        children: Vec<Shadow>,
    },
    Stroke {
        index: u8,
        slot: u8,
    },
}

fn walk(node: &KanjiNode, raw: &mut Vec<Vec<OrientedPoint>>) -> Shadow {
    match node {
        KanjiNode::Stroke { index, path } => {
            let slot = raw.len();
            raw.push(path.to_oriented());
            Shadow::Stroke {
                index: *index,
                slot: slot.try_into().unwrap_or(u8::MAX),
            }
        }
        KanjiNode::Group { element, children } => {
            let children = children.iter().map(|c| walk(c, raw)).collect();
            Shadow::Group {
                element: *element,
                children,
            }
        }
    }
}

fn materialize(
    shadow: &Shadow,
    kanji_frame: &[Vec<OrientedPoint>],
    stroke_frame: &[Vec<OrientedPoint>],
) -> AnalyzedKanjiNode {
    match shadow {
        Shadow::Stroke { index, slot } => AnalyzedKanjiNode::Stroke {
            index: *index,
            in_kanji_frame: kanji_frame[usize::from(*slot)].clone(),
            in_stroke_frame: stroke_frame[usize::from(*slot)].clone(),
        },
        Shadow::Group { element, children } => AnalyzedKanjiNode::Group {
            element: *element,
            children: children
                .iter()
                .map(|s| materialize(s, kanji_frame, stroke_frame))
                .collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KanjiNode;
    use lyon_path::Path;
    use lyon_path::math::point;

    fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        let mut b = Path::builder();
        b.begin(point(x0, y0));
        b.line_to(point(x1, y1));
        b.end(false);
        b.build()
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn single_stroke_kanji_preserves_shape() {
        let node = KanjiNode::Group {
            element: Some('一'),
            children: vec![KanjiNode::Stroke {
                index: 0,
                path: line(10.0, 50.0, 90.0, 50.0),
            }],
        };

        let analyzed = AnalyzedKanjiNode::from_node(&node);

        match analyzed {
            AnalyzedKanjiNode::Group { element, children } => {
                assert_eq!(element, Some('一'));
                assert_eq!(children.len(), 1);
                match &children[0] {
                    AnalyzedKanjiNode::Stroke {
                        index,
                        in_kanji_frame,
                        in_stroke_frame,
                    } => {
                        assert_eq!(*index, 0);
                        assert!(!in_kanji_frame.is_empty());
                        assert!(!in_stroke_frame.is_empty());
                    }
                    AnalyzedKanjiNode::Group { .. } => panic!("expected Stroke leaf"),
                }
            }
            AnalyzedKanjiNode::Stroke { .. } => panic!("expected Group root"),
        }
    }

    #[test]
    fn three_strokes_normalize_to_kanji_bbox() {
        // Three horizontal strokes, top/middle/bottom — like 三.
        let node = KanjiNode::Group {
            element: Some('三'),
            children: vec![
                KanjiNode::Stroke {
                    index: 0,
                    path: line(20.0, 20.0, 80.0, 20.0),
                },
                KanjiNode::Stroke {
                    index: 1,
                    path: line(20.0, 50.0, 80.0, 50.0),
                },
                KanjiNode::Stroke {
                    index: 2,
                    path: line(20.0, 80.0, 80.0, 80.0),
                },
            ],
        };

        let analyzed = AnalyzedKanjiNode::from_node(&node);
        let AnalyzedKanjiNode::Group {
            children: strokes, ..
        } = analyzed
        else {
            panic!()
        };

        // Kanji frame: bbox is (20,20)..(80,80). Top stroke → y≈0, middle → y≈0.5, bottom → y≈1.
        let y_top = match &strokes[0] {
            AnalyzedKanjiNode::Stroke { in_kanji_frame, .. } => in_kanji_frame[0].position.y,
            AnalyzedKanjiNode::Group { .. } => panic!(),
        };
        let y_mid = match &strokes[1] {
            AnalyzedKanjiNode::Stroke { in_kanji_frame, .. } => in_kanji_frame[0].position.y,
            AnalyzedKanjiNode::Group { .. } => panic!(),
        };
        let y_bot = match &strokes[2] {
            AnalyzedKanjiNode::Stroke { in_kanji_frame, .. } => in_kanji_frame[0].position.y,
            AnalyzedKanjiNode::Group { .. } => panic!(),
        };

        assert!(approx(y_top, 0.0));
        assert!(approx(y_mid, 0.5));
        assert!(approx(y_bot, 1.0));
    }

    #[test]
    fn frame_c_centers_each_stroke_individually() {
        // Three horizontal strokes — each one alone is wide-flat.
        // Stroke frame should center each one vertically at y=0.5.
        let node = KanjiNode::Group {
            element: Some('三'),
            children: vec![
                KanjiNode::Stroke {
                    index: 0,
                    path: line(20.0, 20.0, 80.0, 20.0),
                },
                KanjiNode::Stroke {
                    index: 1,
                    path: line(20.0, 50.0, 80.0, 50.0),
                },
                KanjiNode::Stroke {
                    index: 2,
                    path: line(20.0, 80.0, 80.0, 80.0),
                },
            ],
        };

        let analyzed = AnalyzedKanjiNode::from_node(&node);
        let AnalyzedKanjiNode::Group {
            children: strokes, ..
        } = analyzed
        else {
            panic!()
        };

        for s in &strokes {
            match s {
                AnalyzedKanjiNode::Stroke {
                    in_stroke_frame, ..
                } => {
                    // every point's y should be 0.5 (horizontal stroke, vertically centered)
                    for op in in_stroke_frame {
                        assert!(
                            approx(op.position.y, 0.5),
                            "expected y=0.5, got {}",
                            op.position.y
                        );
                    }
                    // x should span [0, 1]
                    let xs: Vec<f32> = in_stroke_frame.iter().map(|p| p.position.x).collect();
                    let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min);
                    let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    assert!(approx(min_x, 0.0));
                    assert!(approx(max_x, 1.0));
                }
                AnalyzedKanjiNode::Group { .. } => panic!(),
            }
        }
    }

    #[test]
    fn frames_differ_for_corner_stroke() {
        // A small stroke at the corner of a big kanji should be in totally
        // different positions in kanji frame vs stroke frame.
        let node = KanjiNode::Group {
            element: None,
            children: vec![
                KanjiNode::Stroke {
                    index: 0,
                    path: line(0.0, 0.0, 10.0, 0.0),
                }, // top-left tiny
                KanjiNode::Stroke {
                    index: 1,
                    path: line(80.0, 80.0, 100.0, 100.0),
                }, // bottom-right
            ],
        };

        let analyzed = AnalyzedKanjiNode::from_node(&node);
        let AnalyzedKanjiNode::Group {
            children: strokes, ..
        } = analyzed
        else {
            panic!()
        };

        let (kanji, stroke) = match &strokes[0] {
            AnalyzedKanjiNode::Stroke {
                in_kanji_frame,
                in_stroke_frame,
                ..
            } => (in_kanji_frame.clone(), in_stroke_frame.clone()),
            AnalyzedKanjiNode::Group { .. } => panic!(),
        };

        // First stroke in kanji frame: lives near the top-left corner (y is small).
        assert!(kanji[0].position.y < 0.2);
        // First stroke in stroke frame: centered vertically at 0.5.
        assert!(approx(stroke[0].position.y, 0.5));
    }

    #[test]
    fn nested_groups_preserve_structure() {
        // Group { Group { Stroke }, Stroke }
        let node = KanjiNode::Group {
            element: Some('音'),
            children: vec![
                KanjiNode::Group {
                    element: Some('立'),
                    children: vec![KanjiNode::Stroke {
                        index: 0,
                        path: line(0.0, 0.0, 10.0, 0.0),
                    }],
                },
                KanjiNode::Stroke {
                    index: 1,
                    path: line(0.0, 50.0, 10.0, 50.0),
                },
            ],
        };

        let analyzed = AnalyzedKanjiNode::from_node(&node);

        match analyzed {
            AnalyzedKanjiNode::Group { element, children } => {
                assert_eq!(element, Some('音'));
                assert_eq!(children.len(), 2);
                // First child must be a nested group with the inner element.
                match &children[0] {
                    AnalyzedKanjiNode::Group { element, children } => {
                        assert_eq!(*element, Some('立'));
                        assert_eq!(children.len(), 1);
                        assert!(matches!(
                            children[0],
                            AnalyzedKanjiNode::Stroke { index: 0, .. }
                        ));
                    }
                    AnalyzedKanjiNode::Stroke { .. } => panic!("expected nested Group"),
                }
                // Second child is a direct stroke.
                assert!(matches!(
                    children[1],
                    AnalyzedKanjiNode::Stroke { index: 1, .. }
                ));
            }
            AnalyzedKanjiNode::Stroke { .. } => panic!(),
        }
    }

    #[test]
    fn stroke_indices_preserved() {
        let node = KanjiNode::Group {
            element: None,
            children: vec![
                KanjiNode::Stroke {
                    index: 5,
                    path: line(0.0, 0.0, 1.0, 0.0),
                },
                KanjiNode::Stroke {
                    index: 7,
                    path: line(0.0, 1.0, 1.0, 1.0),
                },
            ],
        };
        let analyzed = AnalyzedKanjiNode::from_node(&node);
        match analyzed {
            AnalyzedKanjiNode::Group { children, .. } => {
                let indices: Vec<_> = children
                    .iter()
                    .filter_map(|c| match c {
                        AnalyzedKanjiNode::Stroke { index, .. } => Some(*index),
                        AnalyzedKanjiNode::Group { .. } => None,
                    })
                    .collect();
                assert_eq!(indices, vec![5, 7]);
            }
            AnalyzedKanjiNode::Stroke { .. } => panic!(),
        }
    }
}
