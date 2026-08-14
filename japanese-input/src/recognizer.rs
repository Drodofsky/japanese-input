use core::cmp::Reverse;
use core::f64::consts::{LN_2, TAU};
use core::iter::{self, repeat_with};
use core::mem::swap;
use std::collections::{HashMap, HashSet};

use kurbo::{Point, Rect, Vec2};
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::convert_lossy::ConvertLossy as _;
use crate::rdp::rdp_slice;
use crate::stroke_point::{RDP_EPS, StrokePoint, to_stroke_points};
use crate::stroke_window;
use crate::to_index::ToIndex as _;

pub type RawStrokes = Vec<Vec<(f32, f32)>>;
pub type Dataset = HashMap<char, Vec<RawStrokes>>;

/// Terms, in order: 0 = position, 1 = displacement, 2 = curvature.
pub const N_TERMS: usize = 3;

const VAR_FLOOR: f64 = 1e-3;
const LN_THIRD: f64 = -1.098_612_288_668_109_7;
const N_ITER: usize = 5;
const EPS: f64 = 1e-9;
/// States below this many aligned samples borrow variance from the character's pooled spread.
const VAR_SHRINK: f64 = 4.0;
const EXTENT_FLOOR: f64 = 1e-3;
const SIZE_VAR_FLOOR: f64 = 1e-2;
/// Samples a small-kana variant needs before its size model is fitted rather than borrowed from its base.
const MIN_SIZE_SAMPLES: usize = 8;
/// Small kana are drawn at roughly half scale.
const SMALL_LN_OFFSET: f64 = -LN_2;
/// Share of a character's drawings a writing style needs to become its own cluster.
const MIN_SHARE: f64 = 0.05;
/// Drawings a cluster needs before its mean and variance are trustworthy.
const MIN_CLUSTER: usize = 3;
/// Deepest clustering trained per character; unused levels are dropped by [`Recognizer::trim`].
pub const MAX_LEVEL: usize = 8;
/// Minimum distortion drop a split must earn to be kept as its own level.
const DISTINCT_GAIN: f64 = 0.02;

/// Mean squared distance from each drawing to its own group's centre.
fn distortion(desc: &[Descriptor], groups: &[Vec<usize>]) -> f64 {
    let mut total = 0.0_f64;
    let mut n = 0.0_f64;
    for g in groups {
        let mut centre = [0.0_f64; DESC_POINTS * 4];
        let count: f64 = g.len().convert_lossy();
        if count <= 0.0_f64 {
            continue;
        }
        for d in g.iter().filter_map(|&i| desc.get(i)) {
            for (c, v) in centre.iter_mut().zip(d) {
                *c += v / count;
            }
        }
        for d in g.iter().filter_map(|&i| desc.get(i)) {
            total += dist2(d, &centre);
            n += 1.0_f64;
        }
    }
    if n > 0.0 { total / n } else { 0.0 }
}

pub const DEFAULT_SIZE_WEIGHT: f64 = 1.0;
pub const DEFAULT_STROKE_WEIGHT: f64 = 1.0;
pub const DEFAULT_PRIOR_WEIGHT: f64 = 0.0;

/// Stroke-count gaps beyond this are charged as if they were this large.
const MAX_STROKE_GAP: f64 = 4.0;
/// Smallest share a reading may be assigned, for labels absent from training.
const PRIOR_FLOOR: f64 = 1e-7;

/// A small kana's prior penalty against its base is capped in energy, immune to how large `prior` is tuned.
const MAX_SMALL_PRIOR_GAP: f64 = 0.3;

/// One sampled point as the recognizer sees it: position, displacement, curvature.
pub type Terms = [Vec2; N_TERMS];

/// Dimensions of a [`Placement`].
pub const N_PLACE: usize = 4;

/// How big a drawing is and where it sits: `[ln width, ln height, centre x, centre y]`.
pub type Placement = [f64; N_PLACE];

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RecognitionResult {
    pub character: char,
    pub score: f64,
}

/// The tuned parameters (paper's λ): three shape terms and a transition weight, plus size, stroke-gap, and prior weights.
#[expect(clippy::exhaustive_structs, reason = "tuning tools build this by literal")]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Weights {
    pub term: [f64; N_TERMS],
    pub transition: f64,
    /// Scales the placement term against the shape energy.
    pub size: f64,
    /// Charged per stroke of difference between the drawing and the model.
    pub stroke: f64,
    /// Scales how much a reading's own training frequency counts for.
    pub prior: f64,
}

impl Default for Weights {
    #[inline]
    fn default() -> Self {
        Self {
            term: [0.28, 0.48, 0.0],
            transition: 0.94,
            size: DEFAULT_SIZE_WEIGHT,
            stroke: DEFAULT_STROKE_WEIGHT,
            prior: DEFAULT_PRIOR_WEIGHT,
        }
    }
}

impl Weights {
    #[inline]
    #[must_use]
    pub fn normalized(self) -> Self {
        let sum = self.term.iter().sum::<f64>() + self.transition;
        if sum <= f64::EPSILON || !sum.is_finite() {
            return Self {
                size: self.size,
                stroke: self.stroke,
                prior: self.prior,
                ..Self::default()
            };
        }
        let inv = 1.0_f64 / sum;
        let mut term = self.term;
        for t in &mut term {
            *t *= inv;
        }
        Self {
            term,
            transition: self.transition * inv,
            size: self.size,
            stroke: self.stroke,
            prior: self.prior,
        }
    }
}

/// Bounding box of a drawing, in input units.
fn frame_rect(strokes: &[Vec<(f32, f32)>]) -> Option<Rect> {
    let mut out: Option<Rect> = None;
    for s in strokes {
        for &(x, y) in s {
            let p = Point::new(f64::from(x), f64::from(y));
            out = Some(out.map_or_else(|| Rect::from_points(p, p), |r| r.union_pt(p)));
        }
    }
    out
}

/// Longest side of the drawing's bounding box, in input units.
#[inline]
#[must_use]
pub fn frame_size(strokes: &[Vec<(f32, f32)>]) -> f32 {
    frame_rect(strokes).map_or(f32::INFINITY, |r| {
        r.width().max(r.height()).convert_lossy()
    })
}

/// Size and position of the drawing's box, both axes kept since small kana shrink unevenly.
#[inline]
#[must_use]
pub fn placement(strokes: &[Vec<(f32, f32)>]) -> Placement {
    let ln = |v: f64| v.max(EXTENT_FLOOR).ln();
    frame_rect(strokes).map_or([ln(0.0), ln(0.0), 0.5, 0.5], |r| {
        [ln(r.width()), ln(r.height()), r.center().x, r.center().y]
    })
}

/// Every stroke simplified on its own and appended into one unit-box point sequence.
fn unit_points(strokes: &[Vec<(f32, f32)>]) -> Vec<Point> {
    let Some(bb) = frame_rect(strokes) else {
        return Vec::new();
    };
    let span = bb.width().max(bb.height()).max(EPS);
    let mut out: Vec<Point> = Vec::new();
    for s in strokes {
        let pts: Vec<Point> = s
            .iter()
            .map(|&(x, y)| {
                Point::new(
                    (f64::from(x) - bb.x0) / span,
                    (f64::from(y) - bb.y0) / span,
                )
            })
            .collect();
        out.extend(rdp_slice(&pts, RDP_EPS));
    }
    out
}

/// Position, displacement and turn for every simplified point of a drawing.
#[inline]
#[must_use]
pub fn features(strokes: &[Vec<(f32, f32)>]) -> Vec<Terms> {
    to_stroke_points(unit_points(strokes).into_iter())
        .iter()
        .map(terms_of)
        .collect()
}

/// A drawing prepared once for scoring: its feature points, stroke count, and placement.
#[non_exhaustive]
#[derive(Clone)]
pub struct Drawing {
    pub feats: Vec<Terms>,
    pub strokes: usize,
    pub place: Placement,
}

impl Drawing {
    #[inline]
    #[must_use]
    pub fn new(raw: &[Vec<(f32, f32)>]) -> Self {
        Self {
            feats: features(raw),
            strokes: raw.len(),
            place: placement(raw),
        }
    }
}

/// The three terms a single point contributes.
#[inline]
fn terms_of(p: &StrokePoint) -> Terms {
    [p.position.to_vec2(), p.displacement, p.curvature]
}

fn log_gauss(a: Vec2, mean: Vec2, var: Vec2) -> f64 {
    let t = |d: f64, v: f64| {
        let v = v.max(VAR_FLOOR);
        d * d / v + (TAU * v).ln()
    };
    -0.5_f64 * (t(a.x - mean.x, var.x) + t(a.y - mean.y, var.y))
}

fn emit_cost(pf: &Terms, mean: &Terms, var: &Terms, w: &Weights) -> f64 {
    pf.iter()
        .zip(mean)
        .zip(var)
        .zip(&w.term)
        .map(|(((f, m), v), &wt)| -wt * log_gauss(*f, *m, *v))
        .sum()
}

/// A diagonal Gaussian over [`Placement`]: how big one reading is drawn, and where.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlaceModel {
    mean: Placement,
    var: Placement,
}

impl Default for PlaceModel {
    #[inline]
    fn default() -> Self {
        Self {
            mean: [0.0, 0.0, 0.5, 0.5],
            var: [1.0; N_PLACE],
        }
    }
}

impl PlaceModel {
    /// Cost of this reading given how the drawing was sized and placed.
    fn nll(self, p: Placement) -> f64 {
        0.5_f64
            * p.iter()
                .zip(&self.mean)
                .zip(&self.var)
                .map(|((&a, &m), &v)| {
                    let v = v.max(SIZE_VAR_FLOOR);
                    let d = a - m;
                    d * d / v + (TAU * v).ln()
                })
                .sum::<f64>()
    }

    fn fit(samples: &[Placement]) -> Self {
        let n: f64 = samples.len().convert_lossy();
        if n <= 0.0_f64 {
            return Self::default();
        }
        let (mut s1, mut s2) = ([0.0_f64; N_PLACE], [0.0_f64; N_PLACE]);
        for p in samples {
            for ((a, b), &x) in s1.iter_mut().zip(&mut s2).zip(p) {
                *a += x;
                *b = x.mul_add(x, *b);
            }
        }
        let mut mean = [0.0_f64; N_PLACE];
        let mut var = [0.0_f64; N_PLACE];
        for (((m, v), &a), &b) in mean.iter_mut().zip(&mut var).zip(&s1).zip(&s2) {
            *m = a / n;
            *v = (b / n - *m * *m).max(SIZE_VAR_FLOOR);
        }
        Self { mean, var }
    }

    /// The same spread drawn smaller, for a variant with too few drawings of its own to measure.
    fn shifted(self, by: f64) -> Self {
        let mut mean = self.mean;
        if let Some(w) = mean.first_mut() {
            *w += by;
        }
        if let Some(h) = mean.get_mut(1) {
            *h += by;
        }
        Self {
            mean,
            var: self.var,
        }
    }
}

/// Serializes [`Terms`] as plain `[x, y]` pairs, without depending on `kurbo`'s serde support.
#[expect(clippy::inline_modules, reason = "too small to be worth a second file")]
mod terms_serde {
    use super::{N_TERMS, Terms};
    use kurbo::Vec2;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    type Raw = [[f64; 2]; N_TERMS];

    pub fn serialize<S>(v: &[Terms], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        v.iter()
            .map(|t| t.map(|p| [p.x, p.y]))
            .collect::<Vec<Raw>>()
            .serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<Terms>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Vec::<Raw>::deserialize(d)?
            .into_iter()
            .map(|t| t.map(|[x, y]| Vec2::new(x, y)))
            .collect())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CharModel {
    #[serde(with = "terms_serde")]
    mean: Vec<Terms>,
    #[serde(with = "terms_serde")]
    var: Vec<Terms>,
    log_move: [f64; 3],
    stroke_count: usize,
    /// How this character is sized and placed when written full size.
    place_full: PlaceModel,
    /// The same for its small variant, where it has one.
    place_small: Option<PlaceModel>,
    /// Log frequency of the base reading and of the small one, from training.
    log_prior: [f64; 2],
    /// Which clustering level this prototype was cut at, counting from one.
    level: usize,
    /// The deepest level this character had enough drawings to reach.
    levels: usize,
    /// Log share of the character's drawings this cluster accounts for, a mixture weight.
    ln_share: f64,
}

impl CharModel {
    fn from_template(feats: &[Terms], stroke_count: usize) -> Self {
        Self {
            var: vec![[Vec2::new(1.0, 1.0); N_TERMS]; feats.len()],
            mean: feats.to_vec(),
            log_move: [LN_THIRD; 3],
            stroke_count,
            place_full: PlaceModel::default(),
            place_small: None,
            log_prior: [0.0, 0.0],
            level: 1,
            levels: 1,
            ln_share: 0.0,
        }
    }
}

fn viterbi(model: &CharModel, feats: &[Terms], w: &Weights) -> (f64, Vec<usize>) {
    let states = model.mean.len();
    if states == 0 || feats.is_empty() {
        return (f64::INFINITY, Vec::new());
    }
    let move_cost: [f64; 3] = model.log_move.map(|x| -w.transition * x);
    let cost: Vec<Vec<f64>> = feats
        .iter()
        .map(|pf| {
            model
                .mean
                .iter()
                .zip(&model.var)
                .map(|(m, v)| emit_cost(pf, m, v, w))
                .collect()
        })
        .collect();
    let mut prev: Vec<f64> = cost
        .first()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &c)| if j < 2 { c } else { f64::INFINITY })
                .collect()
        })
        .unwrap_or_default();
    let mut backs: Vec<Vec<usize>> = vec![(0..states).collect()];
    for row in cost.iter().skip(1) {
        let (np, bk): (Vec<f64>, Vec<usize>) = row
            .iter()
            .enumerate()
            .map(|(j, &emit)| {
                let (bp, bc) = (0..=2)
                    .filter_map(|m| {
                        let pj = j.checked_sub(m)?;
                        let mc = move_cost.get(m)?;
                        prev.get(pj).map(|&c| (pj, c + mc))
                    })
                    .min_by_key(|&(_, c)| OrderedFloat(c))
                    .unwrap_or((j, f64::INFINITY));
                (bc + emit, bp)
            })
            .unzip();
        backs.push(bk);
        prev = np;
    }
    let last = states.saturating_sub(1);
    let end = [last, last.saturating_sub(1)]
        .into_iter()
        .min_by_key(|&j| OrderedFloat(prev.get(j).copied().unwrap_or(f64::INFINITY)))
        .unwrap_or(0);
    let energy = prev.get(end).copied().unwrap_or(f64::INFINITY);
    let mut path = vec![0_usize; feats.len()];
    let mut j = end;
    for i in (0..feats.len()).rev() {
        if let Some(s) = path.get_mut(i) {
            *s = j;
        }
        j = backs.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0);
    }
    (energy, path)
}

fn viterbi_energy(model: &CharModel, feats: &[Terms], w: &Weights) -> f64 {
    let states = model.mean.len();
    if states == 0 || feats.is_empty() {
        return f64::INFINITY;
    }
    let move_cost: [f64; 3] = model.log_move.map(|x| -w.transition * x);
    let Some(first) = feats.first() else {
        return f64::INFINITY;
    };
    let mut prev: Vec<f64> = model
        .mean
        .iter()
        .zip(&model.var)
        .enumerate()
        .map(|(j, (m, v))| {
            if j < 2 {
                emit_cost(first, m, v, w)
            } else {
                f64::INFINITY
            }
        })
        .collect();
    let mut next: Vec<f64> = vec![0.0_f64; states];
    for pf in feats.iter().skip(1) {
        for (j, ((m, v), slot)) in model
            .mean
            .iter()
            .zip(&model.var)
            .zip(next.iter_mut())
            .enumerate()
        {
            let best = (0..=2)
                .filter_map(|mv| {
                    let pj = j.checked_sub(mv)?;
                    let mc = move_cost.get(mv)?;
                    prev.get(pj).map(|&c| c + mc)
                })
                .fold(f64::INFINITY, f64::min);
            *slot = best + emit_cost(pf, m, v, w);
        }
        swap(&mut prev, &mut next);
    }
    let last = states.saturating_sub(1);
    [last, last.saturating_sub(1)]
        .into_iter()
        .map(|j| prev.get(j).copied().unwrap_or(f64::INFINITY))
        .fold(f64::INFINITY, f64::min)
}

#[derive(Clone)]
struct Acc {
    n: f64,
    s: Terms,
    s2: Terms,
}

impl Default for Acc {
    fn default() -> Self {
        Self {
            n: 0.0,
            s: [Vec2::ZERO; N_TERMS],
            s2: [Vec2::ZERO; N_TERMS],
        }
    }
}

impl Acc {
    fn add(&mut self, pf: &Terms) {
        self.n += 1.0_f64;
        for ((s, s2), f) in self.s.iter_mut().zip(&mut self.s2).zip(pf) {
            *s = Vec2::new(s.x + f.x, s.y + f.y);
            *s2 = Vec2::new(f.x.mul_add(f.x, s2.x), f.y.mul_add(f.y, s2.y));
        }
    }

    fn finish(&self) -> (Terms, Terms) {
        let n = self.n.max(EPS);
        let mut mean = [Vec2::ZERO; N_TERMS];
        let mut var = [Vec2::ZERO; N_TERMS];
        for (((mo, vo), s), s2) in mean.iter_mut().zip(&mut var).zip(&self.s).zip(&self.s2) {
            let mu = Vec2::new(s.x / n, s.y / n);
            *mo = mu;
            *vo = Vec2::new(
                (s2.x / n - mu.x * mu.x).max(VAR_FLOOR),
                (s2.y / n - mu.y * mu.y).max(VAR_FLOOR),
            );
        }
        (mean, var)
    }
}

fn reestimate(model: &mut CharModel, samples: &[Vec<Terms>], w: &Weights) {
    let states = model.mean.len();
    let mut accs: Vec<Acc> = repeat_with(Acc::default).take(states).collect();
    let mut pooled = Acc::default();
    let mut moves = [1.0_f64; 3];
    for feats in samples {
        let (_, path) = viterbi(model, feats, w);
        let mut prev: Option<usize> = None;
        for (i, &st) in path.iter().enumerate() {
            if let Some(pf) = feats.get(i) {
                pooled.add(pf);
                if let Some(acc) = accs.get_mut(st) {
                    acc.add(pf);
                }
            }
            if let Some(m) = prev
                .and_then(|p| st.checked_sub(p))
                .and_then(|d| moves.get_mut(d))
            {
                *m += 1.0_f64;
            }
            prev = Some(st);
        }
    }
    // Every state is pulled toward the character's own spread, by an amount that
    // fades as it gathers samples of its own. A state nothing aligned to takes
    // that spread outright, so it stays a plausible state of this character
    // rather than the box-wide wildcard its initial variance would leave it as.
    let (_, gvar) = pooled.finish();
    for (acc, (m, v)) in accs
        .iter()
        .zip(model.mean.iter_mut().zip(model.var.iter_mut()))
    {
        if acc.n > 0.0_f64 {
            let (nm, nv) = acc.finish();
            let k = acc.n / (acc.n + VAR_SHRINK);
            *m = nm;
            for ((vo, sv), gv) in v.iter_mut().zip(&nv).zip(&gvar) {
                *vo = Vec2::new(
                    k.mul_add(sv.x, (1.0 - k) * gv.x).max(VAR_FLOOR),
                    k.mul_add(sv.y, (1.0 - k) * gv.y).max(VAR_FLOOR),
                );
            }
        } else {
            *v = gvar;
        }
    }
    let total: f64 = moves.iter().sum();
    model.log_move = moves.map(|x| (x / total).ln());
}

fn train_one(samples: &[Vec<Terms>], t: usize, stroke_count: usize, w: &Weights) -> CharModel {
    let tmpl = samples.get(t).map_or([].as_slice(), Vec::as_slice);
    let mut m = CharModel::from_template(tmpl, stroke_count);
    for _ in 0..N_ITER {
        reestimate(&mut m, samples, w);
    }
    m
}

/// The stroke count seen most often, ties going to the smaller one.
fn common_stroke_count(lens: impl Iterator<Item = usize>) -> usize {
    let mut tally: Vec<(usize, usize)> = Vec::new();
    for len in lens {
        match tally.iter_mut().find(|(c, _)| *c == len) {
            Some(slot) => slot.1 = slot.1.saturating_add(1),
            None => tally.push((len, 1)),
        }
    }
    tally
        .iter()
        .copied()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map_or(1, |(c, _)| c.max(1))
}

/// Log share of the training drawings a label accounted for, floored so an unseen label is merely unlikely.
fn ln_share(n: usize, total: usize) -> f64 {
    let share: f64 = n.convert_lossy();
    let total: f64 = total.convert_lossy();
    (share / total.max(1.0)).max(PRIOR_FLOOR).ln()
}

/// Points a resampled half-descriptor (shape or order) is built from before clustering.
const DESC_POINTS: usize = 12;

/// Lloyd iterations for k-means clustering.
const KMEANS_ITERS: usize = 8;

/// A drawing's style-and-order fingerprint: shape resampled by point, then stroke centroids resampled by stroke.
type Descriptor = [f64; DESC_POINTS * 4];

/// Resamples a sequence of unit-box points to `DESC_POINTS` steps along it.
fn resample(points: &[Vec2], out: &mut [f64]) {
    let last = points.len().saturating_sub(1);
    if points.is_empty() {
        return;
    }
    let span: f64 = last.convert_lossy();
    let steps: f64 = DESC_POINTS.saturating_sub(1).max(1).convert_lossy();
    for (index, slot) in out.chunks_exact_mut(2).enumerate() {
        let at = span * index.convert_lossy() / steps;
        let floor = at.floor();
        let frac = at - floor;
        let before = points.get(floor.to_index()).copied().unwrap_or(Vec2::ZERO);
        let after = points
            .get(floor.to_index().saturating_add(1))
            .copied()
            .unwrap_or(before);
        let point = before.lerp(after, frac);
        if let [px, py] = slot {
            *px = point.x;
            *py = point.y;
        }
    }
}

/// A stroke's centroid, normalized into the drawing's own unit box.
fn stroke_centroid(stroke: &[(f32, f32)], frame: Rect, span: f64) -> Vec2 {
    let n = stroke.len().convert_lossy().max(1.0_f64);
    let sum = stroke.iter().fold(Vec2::ZERO, |acc, &(x, y)| {
        Vec2::new(
            acc.x + (f64::from(x) - frame.x0) / span,
            acc.y + (f64::from(y) - frame.y0) / span,
        )
    });
    Vec2::new(sum.x / n, sum.y / n)
}

/// A single prototype's Viterbi alignment is strictly forward over one state sequence, so a stroke-order difference has to be told apart here or it never can be.
fn descriptor(feats: &[Terms], raw: &RawStrokes) -> Descriptor {
    let mut out = [0.0_f64; DESC_POINTS * 4];
    let (shape, order) = out.split_at_mut(DESC_POINTS * 2);
    resample(&feats.iter().map(|f| f[0]).collect::<Vec<Vec2>>(), shape);
    if let Some(frame) = frame_rect(raw) {
        let span = frame.width().max(frame.height()).max(EPS);
        let centroids: Vec<Vec2> = raw.iter().map(|s| stroke_centroid(s, frame, span)).collect();
        resample(&centroids, order);
    }
    out
}

fn dist2(a: &Descriptor, b: &Descriptor) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum()
}

/// K-means over the style-and-order fingerprint into writing styles, largest group first, thin ones folded into it.
fn clusters(desc: &[Descriptor], k: usize, floor: usize) -> Vec<Vec<usize>> {
    let n = desc.len();
    if n == 0 {
        return Vec::new();
    }
    let floor = floor.max(MIN_CLUSTER);
    #[expect(clippy::arithmetic_side_effects, reason = "floor is at least MIN_CLUSTER, never zero")]
    let k = k.max(1).min(n.saturating_div(floor).max(1));
    if k == 1 {
        return vec![(0..n).collect()];
    }
    // Farthest-first seeding: start from the first drawing, then repeatedly take
    // whichever is least like anything chosen so far. Cheap, deterministic, and
    // it puts the seeds in different writing styles rather than in one crowd.
    let mut seeds: Vec<Descriptor> = Vec::with_capacity(k);
    if let Some(d0) = desc.first() {
        seeds.push(*d0);
    }
    while seeds.len() < k {
        let pick = desc
            .iter()
            .map(|d| seeds.iter().map(|s| dist2(d, s)).fold(f64::INFINITY, f64::min))
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        match pick.and_then(|i| desc.get(i)) {
            Some(d) => seeds.push(*d),
            None => break,
        }
    }
    let mut owner: Vec<usize> = vec![0; n];
    for _ in 0..KMEANS_ITERS {
        let mut moved = false;
        for (o, d) in owner.iter_mut().zip(desc) {
            let best = seeds
                .iter()
                .enumerate()
                .min_by(|a, b| dist2(d, a.1).total_cmp(&dist2(d, b.1)))
                .map_or(0, |(i, _)| i);
            if *o != best {
                *o = best;
                moved = true;
            }
        }
        if !moved {
            break;
        }
        for (c, seed) in seeds.iter_mut().enumerate() {
            let mut sum = [0.0_f64; DESC_POINTS * 2];
            let mut count = 0.0_f64;
            for (d, _) in desc.iter().zip(&owner).filter(|(_, o)| **o == c) {
                for (acc, v) in sum.iter_mut().zip(d) {
                    *acc += v;
                }
                count += 1.0_f64;
            }
            if count > 0.0_f64 {
                for (s, acc) in seed.iter_mut().zip(&sum) {
                    *s = acc / count;
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); seeds.len()];
    for (i, &o) in owner.iter().enumerate() {
        if let Some(g) = groups.get_mut(o) {
            g.push(i);
        }
    }
    groups.sort_by_key(|g| Reverse(g.len()));
    let mut out: Vec<Vec<usize>> = Vec::new();
    for g in groups {
        if g.len() >= floor {
            out.push(g);
            continue;
        }
        match out.first_mut() {
            Some(first) => first.extend(g),
            None if !g.is_empty() => out.push(g),
            None => {}
        }
    }
    out
}

fn default_window() -> Vec<(u8, u8)> {
    stroke_window::WINDOWS
        .iter()
        .map(|w| (w.lo, w.hi))
        .collect()
}

/// Whether a `template`-stroke model is worth scoring against a `user`-stroke drawing.
fn admits(window: &[(u8, u8)], template: usize, user: usize) -> bool {
    let Some(index) = user.checked_sub(1) else {
        return false;
    };
    let index = index.min(window.len().saturating_sub(1));
    match (window.get(index), u8::try_from(template)) {
        (Some(&(lo, hi)), Ok(t)) => lo <= t && t <= hi,
        _ => false,
    }
}

/// A tuned configuration for one candidate budget, built by [`Recognizer::upsert_preset`].
#[non_exhaustive]
#[derive(Clone, Serialize, Deserialize)]
pub struct Preset {
    pub cap: usize,
    pub depths: HashMap<char, usize>,
    pub weights: Weights,
    pub window: Vec<(u8, u8)>,
    pub stats: PresetStats,
}

#[expect(clippy::exhaustive_structs, reason = "tuning tools build this by literal")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PresetStats {
    pub hira: f64,
    pub kanji: f64,
    pub samples: usize,
    pub candidates: usize,
}

impl Preset {
    #[inline]
    #[must_use]
    pub fn key(&self) -> usize {
        self.cap.saturating_div(1000)
    }

    #[inline]
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "key {:>3}  cand {:>6}/{:>6}  deep {:>4}  hira {:5.2}%  kanji {:5.2}%  (n={})",
            self.key(),
            self.stats.candidates,
            self.cap,
            self.depths.values().filter(|&&d| d > 1).count(),
            self.stats.hira * 100.0,
            self.stats.kanji * 100.0,
            self.stats.samples
        )
    }
}

/// What one stored prototype costs and when it is used, for budget arithmetic.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub ch: char,
    pub strokes: usize,
    pub level: usize,
    pub levels: usize,
}

impl Slot {
    /// Whether this prototype is the one its character answers with at `depth`, clamped to its own range.
    #[inline]
    #[must_use]
    pub fn active(self, depth: usize) -> bool {
        self.level == depth.min(self.levels).max(1)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Recognizer {
    weights: Weights,
    /// Clustering depth per character, as the cluster phase settled it; absent characters use one.
    depths: HashMap<char, usize>,
    /// `depths` resolved against each prototype, rebuilt whenever either moves.
    #[serde(skip)]
    live: Vec<bool>,
    window: Vec<(u8, u8)>,
    models: Vec<(char, CharModel)>,
    /// Tuned configurations by candidate budget; the active one is mirrored into `weights` and `window`.
    presets: Vec<Preset>,
}

/// Every prototype trained for one character: its shape models at each clustering level it earns, sharing one placement model.
fn train_char(
    c: char,
    raws: &[&RawStrokes],
    extent: &HashMap<char, Vec<Placement>>,
    drawings: usize,
    weights: &Weights,
) -> Vec<(char, CharModel)> {
    let full = PlaceModel::fit(extent.get(&c).map_or(&[][..], Vec::as_slice));
    let small = to_small(c).map(|sc| {
        extent
            .get(&sc)
            .filter(|v| v.len() >= MIN_SIZE_SAMPLES)
            .map_or_else(|| full.shifted(SMALL_LN_OFFSET), |v| PlaceModel::fit(v))
    });
    let all: Vec<Vec<Terms>> = raws.iter().map(|s| features(s)).collect();
    let desc: Vec<Descriptor> = all
        .iter()
        .zip(raws.iter())
        .map(|(f, &r)| descriptor(f, r))
        .collect();
    let total: f64 = all.len().convert_lossy();
    let floor = (total * MIN_SHARE).to_index().max(MIN_CLUSTER);
    // Splits until a level leaves a group too thin to train, or fails to earn DISTINCT_GAIN.
    let mut levels: Vec<Vec<Vec<usize>>> = Vec::new();
    let mut tightest = f64::INFINITY;
    for k in 1..=MAX_LEVEL {
        let g = clusters(&desc, k, floor);
        if g.len() != k || g.iter().any(|x| x.len() < floor) {
            break;
        }
        let spread = distortion(&desc, &g);
        if k > 1 && spread > tightest * (1.0_f64 - DISTINCT_GAIN) {
            break;
        }
        tightest = spread;
        levels.push(g);
    }
    if levels.is_empty() {
        levels.push(vec![(0..all.len()).collect()]);
    }
    let deepest = levels.len();
    let prior = [
        ln_share(extent.get(&c).map_or(0, Vec::len), drawings),
        ln_share(
            to_small(c).map_or(0, |sc| extent.get(&sc).map_or(0, Vec::len)),
            drawings,
        ),
    ];
    let mut out: Vec<(char, CharModel)> = Vec::new();
    for (li, groups) in levels.iter().enumerate() {
        for group in groups {
            let feats: Vec<Vec<Terms>> = group.iter().filter_map(|&i| all.get(i).cloned()).collect();
            let picked: Vec<&RawStrokes> = group.iter().filter_map(|&i| raws.get(i).copied()).collect();
            let mut sorted: Vec<(usize, usize)> =
                feats.iter().enumerate().map(|(i, f)| (f.len(), i)).collect();
            sorted.sort_unstable();
            let t = sorted.get(sorted.len().saturating_div(2)).map_or(0, |&(_, i)| i);
            let count = common_stroke_count(picked.iter().map(|s| s.len()));
            let mut m = train_one(&feats, t, count, weights);
            m.place_full = full;
            m.place_small = small;
            m.log_prior = prior;
            m.level = li.saturating_add(1);
            m.levels = deepest;
            let share: f64 = group.len().convert_lossy();
            m.ln_share = (share / total.max(1.0)).max(PRIOR_FLOOR).ln();
            out.push((c, m));
        }
    }
    out
}

/// Training-time report on cluster depth, mixture weights, and small-kana separability.
#[expect(clippy::print_stderr, reason = "diagnostics for the fit-recognizer tool, not library behavior")]
fn log_fit_diagnostics(models: &[(char, CharModel)], extent: &HashMap<char, Vec<Placement>>) {
    let measured = extent
        .iter()
        .filter(|(c, v)| to_base(**c) != **c && v.len() >= MIN_SIZE_SAMPLES)
        .count();
    eprintln!(
        "[fit] {} prototypes over {} labels | {measured} small variants measured from their own drawings",
        models.len(),
        extent.len()
    );

    let mut deepest: HashMap<char, usize> = HashMap::new();
    for (c, m) in models {
        let e = deepest.entry(*c).or_insert(1);
        *e = (*e).max(m.levels);
    }
    let mut hist = [0_usize; MAX_LEVEL + 1];
    #[expect(clippy::iter_over_hash_type, reason = "counting into a histogram is order-independent")]
    for &l in deepest.values() {
        if let Some(h) = hist.get_mut(l.min(MAX_LEVEL)) {
            *h = h.saturating_add(1);
        }
    }
    let chars: f64 = deepest.len().convert_lossy();
    eprint!("[fit] levels over {} characters:", deepest.len());
    for (l, &n) in hist.iter().enumerate().skip(1) {
        if n > 0 {
            let pct = 100.0_f64 * n.convert_lossy() / chars.max(1.0_f64);
            eprint!("  {l}:{n} ({pct:.0}%)");
        }
    }
    eprintln!();

    let mut thinnest: Vec<(f64, char)> = models
        .iter()
        .filter(|(_, m)| m.levels > 1 && m.level == m.levels)
        .map(|(c, m)| (m.ln_share.exp(), *c))
        .collect();
    thinnest.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut listed: Vec<char> = Vec::new();
    thinnest.retain(|&(_, c)| {
        let fresh = !listed.contains(&c);
        if fresh {
            listed.push(c);
        }
        fresh
    });
    eprint!("[fit] thinnest clusters at full depth:");
    for &(share, c) in thinnest.iter().take(6) {
        eprint!("  {c} {:.0}%", share * 100.0_f64);
    }
    eprintln!();

    let mut sep_rows: Vec<(f64, char, usize, [f64; N_PLACE])> = Vec::new();
    let mut seen: Vec<char> = Vec::new();
    for (c, m) in models {
        if seen.contains(c) {
            continue;
        }
        let Some(sm) = m.place_small else { continue };
        seen.push(*c);
        let f = m.place_full;
        let mut sd = [0.0_f64; N_PLACE];
        for (i, out) in sd.iter_mut().enumerate() {
            let gap = match (f.mean.get(i), sm.mean.get(i)) {
                (Some(a), Some(b)) => (b - a).abs(),
                _ => 0.0_f64,
            };
            let pooled = match (f.var.get(i), sm.var.get(i)) {
                (Some(a), Some(b)) => ((a + b) / 2.0).max(SIZE_VAR_FLOOR).sqrt(),
                _ => 1.0_f64,
            };
            *out = gap / pooled;
        }
        let best = sd.iter().copied().fold(0.0_f64, f64::max);
        sep_rows.push((best, *c, m.levels, sd));
    }
    sep_rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (best, c, levels, sd) in sep_rows {
        eprintln!(
            "[fit] {c} levels {levels} | small-vs-base in sd: ln_w {:.2} ln_h {:.2} cx {:.2} cy {:.2}  (best {best:.2}){}",
            sd.first().copied().unwrap_or(0.0_f64),
            sd.get(1).copied().unwrap_or(0.0_f64),
            sd.get(2).copied().unwrap_or(0.0_f64),
            sd.get(3).copied().unwrap_or(0.0_f64),
            if best < 1.0_f64 { "  <- not separable" } else { "" }
        );
    }
}

impl Recognizer {
    #[inline]
    #[must_use]
    pub fn fit(data: &Dataset) -> Self {
        Self::fit_with(data, Weights::default())
    }

    /// Trains with the given weights instead of the default, so a targeted re-fit (see
    /// [`Self::splice`]) can align against the same weights its patch will be scored under.
    #[inline]
    #[must_use]
    pub fn fit_with(data: &Dataset, weights: Weights) -> Self {
        let corpus_drawings = data.values().map(Vec::len).sum();
        Self::fit_scoped(data, weights, corpus_drawings)
    }

    /// Like [`Self::fit_with`], but every reading's prior is judged against `corpus_drawings`
    /// rather than `data`'s own total — for training a handful of characters in isolation, whose
    /// true rarity has to be judged against the full corpus they will be spliced back into, not
    /// against a pool containing only themselves.
    #[inline]
    #[must_use]
    pub fn fit_scoped(data: &Dataset, weights: Weights, corpus_drawings: usize) -> Self {
        let mut shape: HashMap<char, Vec<&RawStrokes>> = HashMap::new();
        let mut extent: HashMap<char, Vec<Placement>> = HashMap::new();
        // Shape trains on the base character (a small kana is its base drawn smaller);
        // placement stays keyed on the literal one, so size can tell them apart.
        #[expect(clippy::iter_over_hash_type, reason = "every entry is folded in independent of order")]
        for (&c, raws) in data {
            shape.entry(to_base(c)).or_default().extend(raws.iter());
            extent
                .entry(c)
                .or_default()
                .extend(raws.iter().map(|s| placement(s)));
        }
        let models: Vec<(char, CharModel)> = shape
            .par_iter()
            .flat_map_iter(|(&c, raws)| train_char(c, raws, &extent, corpus_drawings, &weights))
            .collect();
        log_fit_diagnostics(&models, &extent);
        let mut rec = Self {
            weights,
            depths: HashMap::new(),
            live: Vec::new(),
            window: default_window(),
            models,
            presets: Vec::new(),
        };
        rec.refresh();
        rec
    }

    /// The readings one prototype proposes: the character, and its small variant where it has one.
    fn emit<'model>(
        c: char,
        m: &'model CharModel,
        d: &'model Drawing,
        w: &'model Weights,
    ) -> impl Iterator<Item = (char, f64)> + 'model {
        let gap = m.stroke_count.abs_diff(d.strokes).convert_lossy();
        let base = w.stroke.mul_add(f64::min(gap, MAX_STROKE_GAP), viterbi_energy(m, &d.feats, w));
        let share = m.ln_share;
        let lp = m.log_prior;
        let score = move |p: PlaceModel, prior: f64| {
            w.size.mul_add(p.nll(d.place), base) - w.prior * (prior + share)
        };
        let full = (c, score(m.place_full, lp[0]));
        // Refunds whatever the small reading's prior term charges beyond MAX_SMALL_PRIOR_GAP.
        let excess = (w.prior * (lp[0] - lp[1]) - MAX_SMALL_PRIOR_GAP).max(0.0);
        let small = m
            .place_small
            .map(|p| (to_small(c).unwrap_or(c), score(p, lp[1]) - excess));
        iter::once(full).chain(small)
    }

    /// Every reading the live prototypes admitted by `window` propose, as `(character, energy)`.
    fn readings<'query>(
        &'query self,
        d: &'query Drawing,
        w: &'query Weights,
        window: &'query [(u8, u8)],
        live: &'query [bool],
    ) -> impl ParallelIterator<Item = (char, f64)> + 'query {
        self.models
            .par_iter()
            .enumerate()
            .filter(move |&(i, (_, m))| {
                live.get(i).copied().unwrap_or(false) && admits(window, m.stroke_count, d.strokes)
            })
            .flat_map_iter(move |(_, (c, m))| Self::emit(*c, m, d, w))
            .filter(|&(_, e)| e.is_finite())
    }

    /// The best reading from any character except `skip`, independent of `skip`'s own depth.
    #[inline]
    #[must_use]
    pub fn best_other(&self, d: &Drawing, w: &Weights, window: &[(u8, u8)], live: &[bool], skip: char) -> Option<(char, f64)> {
        self.models
            .par_iter()
            .enumerate()
            .filter(|&(i, (c, m))| {
                *c != skip
                    && live.get(i).copied().unwrap_or(false)
                    && admits(window, m.stroke_count, d.strokes)
            })
            .flat_map_iter(|(_, (c, m))| Self::emit(*c, m, d, w))
            .filter(|&(_, e)| e.is_finite())
            .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
    }

    /// The best reading from one character, answering at one clustering level.
    #[inline]
    #[must_use]
    pub fn best_at(&self, d: &Drawing, w: &Weights, window: &[(u8, u8)], ch: char, level: usize) -> Option<(char, f64)> {
        self.models
            .iter()
            .filter(|(c, m)| {
                *c == ch && m.level == level.min(m.levels).max(1) && admits(window, m.stroke_count, d.strokes)
            })
            .flat_map(|(c, m)| Self::emit(*c, m, d, w))
            .filter(|&(_, e)| e.is_finite())
            .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
    }

    #[inline]
    #[must_use]
    pub fn classify_with(&self, d: &Drawing, w: &Weights, window: &[(u8, u8)], live: &[bool]) -> Option<char> {
        self.readings(d, w, window, live)
            .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            .map(|(c, _)| c)
    }

    #[inline]
    #[must_use]
    pub fn recognize(&self, strokes: &[Vec<(f32, f32)>]) -> Vec<RecognitionResult> {
        let d = Drawing::new(strokes);
        if d.feats.is_empty() {
            return Vec::new();
        }
        let mut r: Vec<RecognitionResult> = self
            .readings(&d, &self.weights, &self.window, &self.live)
            .map(|(character, score)| RecognitionResult { character, score })
            .collect();
        r.sort_by(|a, b| a.score.total_cmp(&b.score).then_with(|| a.character.cmp(&b.character)));
        // A prototype's small-kana reading is a second candidate for the same character;
        // keep only the best-scoring proposal per character.
        let mut seen: HashSet<char> = HashSet::new();
        r.retain(|res| seen.insert(res.character));
        r
    }

    #[inline]
    pub fn set_weights(&mut self, w: Weights) {
        self.weights = w;
    }

    #[inline]
    #[must_use]
    pub fn weights(&self) -> Weights {
        self.weights
    }

    #[inline]
    pub fn set_window(&mut self, w: Vec<(u8, u8)>) {
        self.window = w;
    }

    #[inline]
    #[must_use]
    pub fn window(&self) -> &[(u8, u8)] {
        &self.window
    }

    #[inline]
    #[must_use]
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Reference stroke count of every trained model, for candidate-count costs.
    #[inline]
    #[must_use]
    pub fn model_slots(&self) -> Vec<Slot> {
        self.models
            .iter()
            .map(|(c, m)| Slot {
                ch: *c,
                strokes: m.stroke_count,
                level: m.level,
                levels: m.levels,
            })
            .collect()
    }

    /// Resolves `depths` into the per-prototype mask the scorer reads.
    fn refresh(&mut self) {
        let depths = &self.depths;
        self.live = self
            .models
            .iter()
            .map(|(c, m)| {
                let want = depths.get(c).copied().unwrap_or(1);
                m.level == want.min(m.levels).max(1)
            })
            .collect();
    }

    /// The mask a trial set of depths would produce, without applying it.
    #[inline]
    #[must_use]
    pub fn mask_for(&self, depths: &HashMap<char, usize>) -> Vec<bool> {
        self.models
            .iter()
            .map(|(c, m)| {
                let want = depths.get(c).copied().unwrap_or(1);
                m.level == want.min(m.levels).max(1)
            })
            .collect()
    }

    #[inline]
    pub fn set_depths(&mut self, depths: HashMap<char, usize>) {
        self.depths = depths;
        self.refresh();
    }

    #[inline]
    #[must_use]
    pub fn depths(&self) -> &HashMap<char, usize> {
        &self.depths
    }

    #[inline]
    #[must_use]
    pub fn live(&self) -> &[bool] {
        &self.live
    }

    /// Drops every prototype that neither the live depths nor any stored preset uses.
    #[inline]
    pub fn trim(&mut self) -> usize {
        let mut used = self.mask_for(&self.depths);
        for p in &self.presets {
            for (u, m) in used.iter_mut().zip(self.mask_for(&p.depths)) {
                *u |= m;
            }
        }
        let before = self.models.len();
        let mut keep = used.iter();
        self.models
            .retain(|_| keep.next().copied().unwrap_or(false));
        let mut deepest: HashMap<char, usize> = HashMap::new();
        for (c, m) in &self.models {
            let e = deepest.entry(*c).or_insert(1);
            *e = (*e).max(m.level);
        }
        for (c, m) in &mut self.models {
            m.levels = deepest.get(c).copied().unwrap_or(1);
        }
        self.refresh();
        before.saturating_sub(self.models.len())
    }

    /// Replaces every prototype for the given base characters with `patch`'s versions of them, leaving everyone else untouched.
    #[inline]
    pub fn splice(&mut self, patch: &Self, targets: &[char]) {
        self.models.retain(|(c, _)| !targets.contains(&to_base(*c)));
        self.models
            .extend(patch.models.iter().filter(|(c, _)| targets.contains(&to_base(*c))).cloned());
        self.refresh();
    }

    #[inline]
    pub fn upsert_preset(
        &mut self,
        cap: usize,
        depths: HashMap<char, usize>,
        weights: Weights,
        window: Vec<(u8, u8)>,
        stats: PresetStats,
    ) {
        let p = Preset {
            cap,
            depths,
            weights,
            window,
            stats,
        };
        match self.presets.iter_mut().find(|e| e.cap == cap) {
            Some(slot) => *slot = p,
            None => self.presets.push(p),
        }
        self.presets.sort_by_key(|e| e.cap);
    }

    #[inline]
    #[must_use]
    pub fn list_presets(&self) -> Vec<usize> {
        self.presets.iter().map(Preset::key).collect()
    }

    #[inline]
    #[must_use]
    pub fn describe_presets(&self) -> Vec<String> {
        self.presets.iter().map(Preset::describe).collect()
    }

    #[inline]
    #[must_use]
    pub fn preset(&self, key: usize) -> Option<&Preset> {
        self.presets.iter().find(|p| p.key() == key)
    }

    #[inline]
    #[must_use]
    pub fn nearest_preset(&self, key: usize) -> Option<&Preset> {
        self.presets.get(self.nearest_index(key)?)
    }

    /// Index of the preset closest to `key`, ties going to the smaller budget.
    fn nearest_index(&self, key: usize) -> Option<usize> {
        self.presets
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.key()
                    .abs_diff(key)
                    .cmp(&b.key().abs_diff(key))
                    .then_with(|| a.key().cmp(&b.key()))
            })
            .map(|(i, _)| i)
    }

    #[inline]
    pub fn select_preset(&mut self, key: usize) -> Option<usize> {
        let chosen = self.presets.get(self.nearest_index(key)?)?;
        let chosen_key = chosen.key();
        self.weights = chosen.weights;
        self.depths = chosen.depths.clone();
        self.window = chosen.window.clone();
        self.refresh();
        Some(chosen_key)
    }

    /// # Errors
    /// [`postcard::Error`] on serialization failure.
    #[inline]
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    /// # Errors
    /// [`postcard::Error`] if `bytes` do not match the stored layout.
    #[inline]
    pub fn load(bytes: &[u8]) -> Result<Self, postcard::Error> {
        let mut rec = postcard::from_bytes::<Self>(bytes)?;
        rec.refresh();
        Ok(rec)
    }

    #[inline]
    pub fn set_size_weight(&mut self, weight: f64) {
        self.weights.size = weight;
    }
}

/// Full-size kana to its small variant.
#[inline]
#[must_use]
pub fn to_small(c: char) -> Option<char> {
    Some(match c {
        'あ' => 'ぁ',
        'い' => 'ぃ',
        'う' => 'ぅ',
        'え' => 'ぇ',
        'お' => 'ぉ',
        'つ' => 'っ',
        'や' => 'ゃ',
        'ゆ' => 'ゅ',
        'よ' => 'ょ',
        'わ' => 'ゎ',
        'ア' => 'ァ',
        'イ' => 'ィ',
        'ウ' => 'ゥ',
        'エ' => 'ェ',
        'オ' => 'ォ',
        'ツ' => 'ッ',
        'ヤ' => 'ャ',
        'ユ' => 'ュ',
        'ヨ' => 'ョ',
        'ワ' => 'ヮ',
        _ => return None,
    })
}

/// A small kana back to the full-size form the models were trained on.
#[inline]
#[must_use]
pub fn to_base(c: char) -> char {
    match c {
        'ぁ' => 'あ',
        'ぃ' => 'い',
        'ぅ' => 'う',
        'ぇ' => 'え',
        'ぉ' => 'お',
        'っ' => 'つ',
        'ゃ' => 'や',
        'ゅ' => 'ゆ',
        'ょ' => 'よ',
        'ゎ' => 'わ',
        'ァ' => 'ア',
        'ィ' => 'イ',
        'ゥ' => 'ウ',
        'ェ' => 'エ',
        'ォ' => 'オ',
        'ッ' => 'ツ',
        'ャ' => 'ヤ',
        'ュ' => 'ユ',
        'ョ' => 'ヨ',
        'ヮ' => 'ワ',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_stroke() -> Vec<Vec<(f32, f32)>> {
        vec![vec![(0.0, 0.0), (1.0, 0.0)], vec![(1.0, 0.0), (1.0, 1.0)]]
    }

    #[test]
    fn an_empty_drawing_has_no_features() {
        assert!(features(&[]).is_empty());
        assert!(features(&[Vec::new()]).is_empty());
    }

    #[test]
    fn features_are_scaled_into_the_unit_box() {
        for scale in [1.0_f32, 25.0, 400.0] {
            let strokes: Vec<Vec<(f32, f32)>> = box_stroke()
                .iter()
                .map(|s| s.iter().map(|&(x, y)| (x * scale, y * scale)).collect())
                .collect();
            for term in features(&strokes) {
                let position = term[0];
                assert!(
                    (-EPS..=1.0 + EPS).contains(&position.x),
                    "x {} left the unit box at scale {scale}",
                    position.x
                );
                assert!((-EPS..=1.0 + EPS).contains(&position.y));
            }
        }
    }

    #[test]
    fn the_first_point_has_no_displacement() {
        let feats = features(&box_stroke());
        let first = feats.first().expect("a first point");
        assert!(first[1].hypot() < EPS);
    }

    #[test]
    fn frame_size_is_the_longer_side() {
        let wide = vec![vec![(0.0_f32, 0.0), (4.0, 1.0)]];
        assert!((frame_size(&wide) - 4.0).abs() < 1e-6);
        assert!(!frame_size(&[]).is_finite());
    }

    #[test]
    fn placement_keeps_size_and_position_apart() {
        let wide = vec![vec![(1.0_f32, 2.0), (5.0, 3.0)]];
        let p = placement(&wide);
        assert!((p[0] - 4.0_f64.ln()).abs() < 1e-9, "width");
        assert!(p[1].abs() < 1e-9, "height");
        assert!((p[2] - 3.0).abs() < 1e-9, "centre x");
        assert!((p[3] - 2.5).abs() < 1e-9, "centre y");
        assert!(placement(&[]).iter().all(|v| v.is_finite()));
    }

    #[test]
    fn position_separates_two_drawings_of_the_same_size() {
        // The cue that size alone cannot give: a small kana sits low in the cell.
        let high = PlaceModel::fit(&[[-0.7, -0.7, 0.5, 0.3]]);
        let low = PlaceModel::fit(&[[-0.7, -0.7, 0.5, 0.8]]);
        let drawn_low = [-0.7, -0.7, 0.5, 0.8];
        assert!(low.nll(drawn_low) < high.nll(drawn_low));
    }

    #[test]
    fn normalizing_makes_the_weights_sum_to_one() {
        let w = Weights {
            term: [2.0, 3.0, 5.0],
            transition: 10.0,
            size: 0.3,
            stroke: 1.5,
            prior: 0.25,
        }
        .normalized();
        let sum = w.term.iter().sum::<f64>() + w.transition;
        assert!((sum - 1.0).abs() < 1e-12, "{sum}");
        assert!((w.size - 0.3).abs() < 1e-12);
        assert!((w.stroke - 1.5).abs() < 1e-12, "stroke stays out of the budget");
        assert!((w.prior - 0.25).abs() < 1e-12, "prior stays out of the budget");
    }

    #[test]
    fn normalizing_nothing_falls_back_to_the_default() {
        let w = Weights {
            term: [0.0; N_TERMS],
            transition: 0.0,
            size: 0.42,
            stroke: 2.0,
            prior: 0.5,
        }
        .normalized();
        assert_eq!(w.term, Weights::default().term);
        assert!((w.size - 0.42).abs() < 1e-12);
        assert!((w.stroke - 2.0).abs() < 1e-12);
    }

    #[test]
    fn a_window_admits_only_its_own_range() {
        let window = vec![(2_u8, 4_u8)];
        assert!(admits(&window, 2, 1) && admits(&window, 4, 1));
        assert!(!admits(&window, 1, 1) && !admits(&window, 5, 1));
        assert!(!admits(&window, 3, 0));
    }

    #[test]
    fn a_stroke_count_past_the_window_uses_the_last_row() {
        let window = vec![(1_u8, 2_u8), (5, 9)];
        assert!(admits(&window, 7, 40));
    }

    #[test]
    fn a_small_drawing_costs_less_as_the_small_variant() {
        let big = PlaceModel {
            mean: [0.0, 0.0, 0.5, 0.5],
            var: [0.05, 0.05, 0.05, 0.05],
        };
        let small = big.shifted(SMALL_LN_OFFSET);
        let drawn_small = [SMALL_LN_OFFSET, SMALL_LN_OFFSET, 0.5, 0.5];
        assert!(small.nll(drawn_small) < big.nll(drawn_small));
        let drawn_big = [0.0, 0.0, 0.5, 0.5];
        assert!(big.nll(drawn_big) < small.nll(drawn_big));
    }

    #[test]
    fn a_size_model_recovers_the_mean_it_was_fitted_on() {
        let s = PlaceModel::fit(&[[1.0, 2.0, 0.2, 0.4], [3.0, 4.0, 0.4, 0.6]]);
        assert!((s.mean[0] - 2.0).abs() < 1e-12 && (s.mean[1] - 3.0).abs() < 1e-12);
        assert!((s.mean[2] - 0.3).abs() < 1e-12 && (s.mean[3] - 0.5).abs() < 1e-12);
        assert!(PlaceModel::fit(&[]).var[0] > 0.0);
    }

    #[test]
    fn every_small_kana_maps_back_to_its_base() {
        for base in "あいうえおつやゆよわアイウエオツヤユヨワ".chars() {
            let small = to_small(base).expect("a small variant");
            assert_eq!(to_base(small), base);
        }
    }

    #[test]
    fn the_most_common_stroke_count_wins() {
        assert_eq!(common_stroke_count([3, 3, 5].into_iter()), 3);
        assert_eq!(common_stroke_count(core::iter::empty()), 1);
    }

    fn descriptors(at: f64, n: usize) -> Vec<Descriptor> {
        vec![[at; DESC_POINTS * 4]; n]
    }

    /// The floor `fit` would use for a character with this many drawings.
    fn floor_for(n: usize) -> usize {
        ((n as f64 * MIN_SHARE) as usize).max(MIN_CLUSTER)
    }

    #[test]
    fn a_sliver_folds_into_the_biggest_group() {
        // Two drawings out of a hundred is a stray, not a style.
        let mut d = descriptors(0.0, 100);
        d.extend(descriptors(1.0, 2));
        let c = clusters(&d, 2, floor_for(d.len()));
        assert_eq!(c.len(), 1, "two of a hundred cannot carry a prototype");
        assert_eq!(c.first().map(Vec::len), Some(d.len()));
    }

    #[test]
    fn a_rare_character_may_still_divide() {
        // The same split on a character with forty drawings rather than fifteen
        // hundred. An absolute floor would refuse it; a share does not.
        let mut d = descriptors(0.0, 20);
        d.extend(descriptors(1.0, 20));
        let c = clusters(&d, 2, floor_for(d.len()));
        assert_eq!(c.len(), 2, "rare characters get written two ways too");
    }

    #[test]
    fn two_well_stocked_styles_each_get_a_prototype() {
        let mut d = descriptors(0.0, 40);
        d.extend(descriptors(1.0, 40));
        let c = clusters(&d, 2, floor_for(d.len()));
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|g| g.len() == 40));
    }

    #[test]
    fn one_uniform_style_stays_one_prototype() {
        let d = descriptors(0.5, 90);
        let c = clusters(&d, 3, floor_for(d.len()));
        assert_eq!(c.len(), 1, "identical drawings have nothing to split on");
    }

    #[test]
    fn groups_come_back_commonest_first() {
        let mut d = descriptors(0.0, 30);
        d.extend(descriptors(1.0, 60));
        let c = clusters(&d, 2, floor_for(d.len()));
        assert!(
            c.first().map(Vec::len) >= c.get(1).map(Vec::len),
            "the first group must be the way the character is most often written"
        );
    }

    #[test]
    fn a_resampled_descriptor_keeps_the_endpoints() {
        let strokes = box_stroke();
        let feats = features(&strokes);
        let d = descriptor(&feats, &strokes);
        let first = feats.first().map(|f| f[0]).expect("a first point");
        assert!((d[0] - first.x).abs() < 1e-9 && (d[1] - first.y).abs() < 1e-9);
    }

    #[test]
    fn the_order_half_tells_apart_two_drawings_with_the_same_strokes_in_a_different_order() {
        let forward = box_stroke();
        let reversed: Vec<Vec<(f32, f32)>> = forward.iter().rev().cloned().collect();
        let order_half = |strokes: &Vec<Vec<(f32, f32)>>| {
            let d = descriptor(&features(strokes), strokes);
            d[DESC_POINTS * 2..].to_vec()
        };
        let (a, b) = (order_half(&forward), order_half(&reversed));
        let gap: f64 = a.iter().zip(&b).map(|(x, y)| (x - y).powi(2)).sum();
        assert!(gap > 1e-6, "reversing stroke order left the order half unchanged");
    }


    #[test]
    fn a_model_survives_a_round_trip_through_bytes() {
        let mut rec = Recognizer {
            weights: Weights::default(),
            depths: HashMap::new(),
            live: Vec::new(),
            window: default_window(),
            models: vec![('あ', CharModel::from_template(&features(&box_stroke()), 2))],
            presets: Vec::new(),
        };
        rec.refresh();
        rec.upsert_preset(
            20_000,
            HashMap::from([('あ', 2)]),
            Weights::default(),
            default_window(),
            PresetStats::default(),
        );
        let bytes = rec.to_bytes().expect("serialize");
        let back = Recognizer::load(&bytes).expect("deserialize");
        assert_eq!(back.model_count(), 1);
        let slots = back.model_slots();
        assert_eq!(slots.len(), 1);
        assert!(slots.first().is_some_and(|s| s.strokes == 2 && s.active(3)));
        assert_eq!(back.list_presets(), vec![20]);
    }

    #[test]
    fn a_character_answers_at_the_deepest_level_it_has() {
        let shallow = Slot { ch: 'し', strokes: 2, level: 1, levels: 1 };
        assert!(shallow.active(1) && shallow.active(3), "asking deeper costs it nothing");
        let deep = [
            Slot { ch: '右', strokes: 2, level: 1, levels: 3 },
            Slot { ch: '右', strokes: 2, level: 2, levels: 3 },
            Slot { ch: '右', strokes: 2, level: 3, levels: 3 },
        ];
        for depth in 1..=3 {
            let live: Vec<usize> = deep.iter().filter(|s| s.active(depth)).map(|s| s.level).collect();
            assert_eq!(live, vec![depth], "exactly one level answers at a time");
        }
    }

    #[test]
    fn a_split_must_earn_its_level() {
        // One blob: any cut through it is arbitrary, so no second level.
        let one = descriptors(0.5, 90);
        assert!(distortion(&one, &clusters(&one, 1, floor_for(one.len()))) < 1e-12);
        // Two blobs far apart: cutting between them collapses the spread.
        let mut two = descriptors(0.0, 40);
        two.extend(descriptors(1.0, 40));
        let f = floor_for(two.len());
        let flat = distortion(&two, &clusters(&two, 1, f));
        let split = distortion(&two, &clusters(&two, 2, f));
        assert!(split < flat * (1.0 - DISTINCT_GAIN), "a real division");
    }

    #[test]
    fn trimming_spares_every_preset_not_just_the_live_one() {
        let feats = features(&box_stroke());
        let at = |level: usize| {
            let mut m = CharModel::from_template(&feats, 2);
            m.level = level;
            m.levels = 3;
            m
        };
        let mut rec = Recognizer {
            weights: Weights::default(),
            depths: HashMap::from([('あ', 1)]),
            live: Vec::new(),
            window: default_window(),
            // levels 1, 2 and 3 for one character: 1 + 2 + 3 prototypes.
            models: vec![
                ('あ', at(1)),
                ('あ', at(2)),
                ('あ', at(2)),
                ('あ', at(3)),
                ('あ', at(3)),
                ('あ', at(3)),
            ],
            presets: Vec::new(),
        };
        rec.refresh();
        rec.upsert_preset(
            50_000,
            HashMap::from([('あ', 3)]),
            Weights::default(),
            default_window(),
            PresetStats::default(),
        );
        // Live is depth 1, the preset is depth 3, so level 2 is what goes.
        assert_eq!(rec.trim(), 2);
        assert_eq!(rec.model_count(), 4);
        assert_eq!(rec.select_preset(50), Some(50));
        assert_eq!(
            rec.live().iter().filter(|&&b| b).count(),
            3,
            "the preset still finds all three of its prototypes"
        );
    }

    #[test]
    fn trimming_keeps_only_what_the_depths_use() {
        let feats = features(&box_stroke());
        let deep = |level: usize, levels: usize| {
            let mut m = CharModel::from_template(&feats, 2);
            m.level = level;
            m.levels = levels;
            m
        };
        let mut rec = Recognizer {
            weights: Weights::default(),
            depths: HashMap::from([('あ', 2)]),
            live: Vec::new(),
            window: default_window(),
            models: vec![
                ('あ', deep(1, 2)),
                ('あ', deep(2, 2)),
                ('あ', deep(2, 2)),
                ('い', deep(1, 1)),
            ],
            presets: Vec::new(),
        };
        rec.refresh();
        assert_eq!(rec.trim(), 1, "あ's unused level-1 prototype goes");
        assert_eq!(rec.model_count(), 3);
        assert!(rec.live().iter().all(|&b| b), "everything left is in use");
        assert_eq!(
            rec.depths().get(&'あ').copied(),
            Some(2),
            "the depths still name levels that exist, so they stay"
        );
    }

    #[test]
    fn splicing_replaces_only_the_targeted_characters() {
        let feats = features(&box_stroke());
        let mut rec = Recognizer {
            weights: Weights::default(),
            depths: HashMap::new(),
            live: Vec::new(),
            window: default_window(),
            models: vec![
                ('あ', CharModel::from_template(&feats, 2)),
                ('い', CharModel::from_template(&feats, 2)),
            ],
            presets: Vec::new(),
        };
        rec.refresh();
        let mut patch = rec.clone();
        patch.models = vec![
            ('あ', CharModel::from_template(&feats, 2)),
            ('あ', CharModel::from_template(&feats, 2)),
        ];
        rec.splice(&patch, &['あ']);
        assert_eq!(rec.model_count(), 3, "あ's two fresh prototypes replace its one, い is untouched");
        assert_eq!(rec.models.iter().filter(|(c, _)| *c == 'い').count(), 1);
        assert_eq!(rec.models.iter().filter(|(c, _)| *c == 'あ').count(), 2);
    }

    #[test]
    fn fit_scoped_judges_prior_against_the_given_corpus_not_the_datasets_own_total() {
        let strokes = box_stroke();
        let mut dataset: Dataset = HashMap::new();
        dataset.insert('あ', vec![strokes; 10]);
        let natural = Recognizer::fit_with(&dataset, Weights::default());
        let scoped = Recognizer::fit_scoped(&dataset, Weights::default(), 1_000_000);
        let prior_of = |rec: &Recognizer| {
            rec.models
                .iter()
                .find(|(c, _)| *c == 'あ')
                .map(|(_, m)| m.log_prior[0])
                .expect("あ")
        };
        assert!(
            prior_of(&scoped) < prior_of(&natural),
            "judged against a corpus a hundred thousand times bigger, あ's own ten drawings should look far rarer"
        );
    }

    #[test]
    fn a_rarer_reading_starts_from_a_worse_prior() {
        let common = ln_share(1044, 20_292);
        let rare = ln_share(6, 20_292);
        assert!(rare < common);
        assert!((common - rare - (1044.0_f64 / 6.0).ln()).abs() < 1e-9);
        assert!(ln_share(0, 20_292).is_finite(), "an unseen label is not impossible");
    }

    #[test]
    fn a_small_kanas_prior_penalty_stays_capped_however_large_prior_is_tuned() {
        let feats = features(&box_stroke());
        let mut m = CharModel::from_template(&feats, 2);
        m.place_full = PlaceModel::default();
        m.place_small = Some(PlaceModel::default());
        m.log_prior = [0.0, -100.0];
        let d = Drawing::new(&box_stroke());
        let w = Weights {
            prior: 50.0,
            ..Weights::default()
        };
        let readings: Vec<(char, f64)> = Recognizer::emit('あ', &m, &d, &w).collect();
        let full = readings.iter().find(|&&(c, _)| c == 'あ').expect("full").1;
        let small = readings.iter().find(|&&(c, _)| c == 'ぁ').expect("small").1;
        assert!(
            small <= full + MAX_SMALL_PRIOR_GAP + 1e-9,
            "small={small} should never clear full={full} by more than the cap"
        );
    }

    #[test]
    fn a_preset_key_is_its_budget_in_thousands() {
        let p = Preset {
            cap: 20_000,
            depths: HashMap::new(),
            weights: Weights::default(),
            window: default_window(),
            stats: PresetStats::default(),
        };
        assert_eq!(p.key(), 20);
        assert!(p.describe().contains("key  20"));
    }
}