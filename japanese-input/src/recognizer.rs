use core::f64::consts::TAU;
use core::iter::repeat_with;
use core::mem::swap;
use std::collections::HashMap;

use kurbo::{Point, Vec2};
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::rdp::rdp_slice;
use crate::stroke_point::{RDP_EPS, StrokePoint, to_stroke_points};
use crate::stroke_window;

pub type RawStrokes = Vec<Vec<(f32, f32)>>;
pub type Dataset = HashMap<char, Vec<RawStrokes>>;

/// Terms, in order: 0 = position, 1 = displacement, 2 = curvature.
pub const N_TERMS: usize = 3;

const VAR_FLOOR: f64 = 1e-3;
const LN_THIRD: f64 = -1.098_612_288_668_109_7;
const N_ITER: usize = 5;
const EPS: f64 = 1e-9;

pub const DEFAULT_SMALL_THRESHOLD: f64 = 0.5;

/// One sampled point as the recognizer sees it: where it sits, where it came
/// from, and how the path turns there.
pub type Terms = [Vec2; N_TERMS];

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RecognitionResult {
    pub character: char,
    pub score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Weights {
    pub term: [f64; N_TERMS],
    pub transition: f64,
    pub small_threshold: f64,
}

impl Default for Weights {
    #[inline]
    fn default() -> Self {
        Self {
            term: [0.28, 0.48, 0.0],
            transition: 0.94,
            small_threshold: DEFAULT_SMALL_THRESHOLD,
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
                small_threshold: self.small_threshold,
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
            small_threshold: self.small_threshold,
        }
    }
}

/// Longest side of the drawing's bounding box, in input units.
#[must_use]
pub fn frame_size(strokes: &[Vec<(f32, f32)>]) -> f32 {
    let (mut lo, mut hi) = ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]);
    for s in strokes {
        for &(x, y) in s {
            lo[0] = lo[0].min(x);
            lo[1] = lo[1].min(y);
            hi[0] = hi[0].max(x);
            hi[1] = hi[1].max(y);
        }
    }
    if !lo[0].is_finite() {
        return f32::INFINITY;
    }
    (hi[0] - lo[0]).max(hi[1] - lo[1])
}

/// The whole drawing as one simplified point sequence, scaled into a unit box.
///
/// Every stroke is simplified on its own and then appended, so a stroke boundary
/// shows up as one long displacement rather than being smoothed away.
fn unit_points(strokes: &[Vec<(f32, f32)>]) -> Vec<Point> {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for s in strokes {
        for &(x, y) in s {
            let (x, y) = (f64::from(x), f64::from(y));
            lo[0] = lo[0].min(x);
            lo[1] = lo[1].min(y);
            hi[0] = hi[0].max(x);
            hi[1] = hi[1].max(y);
        }
    }
    if !lo[0].is_finite() {
        return Vec::new();
    }
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(EPS);
    let mut out: Vec<Point> = Vec::new();
    for s in strokes {
        let pts: Vec<Point> = s
            .iter()
            .map(|&(x, y)| Point::new((f64::from(x) - lo[0]) / span, (f64::from(y) - lo[1]) / span))
            .collect();
        out.extend(rdp_slice(&pts, RDP_EPS));
    }
    out
}

/// Position, displacement and turn for every simplified point of a drawing.
#[must_use]
pub fn features(strokes: &[Vec<(f32, f32)>]) -> Vec<Terms> {
    to_stroke_points(unit_points(strokes).into_iter())
        .iter()
        .map(terms_of)
        .collect()
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

/// Stores the terms as plain pairs, so the geometry types need no serde support
/// and the layout stays what it was before they were introduced.
mod terms_serde {
    use super::{N_TERMS, Terms};
    use kurbo::Vec2;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    type Raw = [[f64; 2]; N_TERMS];

    pub fn serialize<S: Serializer>(v: &[Terms], s: S) -> Result<S::Ok, S::Error> {
        v.iter()
            .map(|t| t.map(|p| [p.x, p.y]))
            .collect::<Vec<Raw>>()
            .serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Terms>, D::Error> {
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
}

impl CharModel {
    fn from_template(feats: &[Terms], stroke_count: usize) -> Self {
        Self {
            var: vec![[Vec2::new(1.0, 1.0); N_TERMS]; feats.len()],
            mean: feats.to_vec(),
            log_move: [LN_THIRD; 3],
            stroke_count,
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
        self.n += 1.0;
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
    let mut moves = [1.0_f64; 3];
    for feats in samples {
        let (_, path) = viterbi(model, feats, w);
        let mut prev: Option<usize> = None;
        for (i, &st) in path.iter().enumerate() {
            if let (Some(pf), Some(acc)) = (feats.get(i), accs.get_mut(st)) {
                acc.add(pf);
            }
            if let Some(p) = prev {
                if let Some(m) = st.checked_sub(p).and_then(|d| moves.get_mut(d)) {
                    *m += 1.0;
                }
            }
            prev = Some(st);
        }
    }
    for (acc, (m, v)) in accs
        .iter()
        .zip(model.mean.iter_mut().zip(model.var.iter_mut()))
    {
        if acc.n > 0.0 {
            let (nm, nv) = acc.finish();
            *m = nm;
            *v = nv;
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
fn common_stroke_count(raws: &[RawStrokes]) -> usize {
    let mut tally: Vec<(usize, usize)> = Vec::new();
    for s in raws {
        match tally.iter_mut().find(|(c, _)| *c == s.len()) {
            Some(slot) => slot.1 = slot.1.saturating_add(1),
            None => tally.push((s.len(), 1)),
        }
    }
    tally
        .iter()
        .copied()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map_or(1, |(c, _)| c.max(1))
}

fn default_window() -> Vec<(u8, u8)> {
    stroke_window::WINDOWS
        .iter()
        .map(|w| (w.lo, w.hi))
        .collect()
}

/// Whether a model with `template` strokes is worth scoring for a `user` stroke
/// drawing, under a window the presets may have changed.
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Preset {
    pub cap: usize,
    pub weights: Weights,
    pub window: Vec<(u8, u8)>,
    pub stats: PresetStats,
}

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

    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "key {:>3}  cand {:>6}/{:>6}  hira {:5.2}%  kanji {:5.2}%  (n={})",
            self.key(),
            self.stats.candidates,
            self.cap,
            self.stats.hira * 100.0,
            self.stats.kanji * 100.0,
            self.stats.samples
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Recognizer {
    weights: Weights,
    window: Vec<(u8, u8)>,
    models: Vec<(char, CharModel)>,
    /// Tuned configurations by candidate budget; the active one is mirrored
    /// into `weights` and `window`.
    presets: Vec<Preset>,
}

impl Recognizer {
    #[inline]
    #[must_use]
    pub fn fit(data: &Dataset) -> Self {
        let weights = Weights::default();
        let models = data
            .par_iter()
            .map(|(&c, raws)| {
                let feats: Vec<Vec<Terms>> = raws.iter().map(|s| features(s)).collect();
                let mut lens: Vec<(usize, usize)> = feats
                    .iter()
                    .enumerate()
                    .map(|(i, f)| (f.len(), i))
                    .collect();
                lens.sort_unstable();
                let t = lens
                    .get(lens.len().saturating_div(2))
                    .map_or(0, |&(_, i)| i);
                (c, train_one(&feats, t, common_stroke_count(raws), &weights))
            })
            .collect();
        Self {
            weights,
            window: default_window(),
            models,
            presets: Vec::new(),
        }
    }

    #[inline]
    #[must_use]
    pub fn prepare(strokes: &[Vec<(f32, f32)>]) -> (Vec<Terms>, usize, f32) {
        (features(strokes), strokes.len(), frame_size(strokes))
    }

    #[inline]
    #[must_use]
    pub fn classify_with(
        &self,
        feats: &[Terms],
        user_count: usize,
        size: f32,
        w: &Weights,
        window: &[(u8, u8)],
    ) -> Option<char> {
        let winner = self
            .models
            .par_iter()
            .filter(|(_, m)| admits(window, m.stroke_count, user_count))
            .map(|(c, m)| (*c, viterbi_energy(m, feats, w)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(c, _)| c)?;
        Some(resize_kana(winner, size, w))
    }

    #[inline]
    #[must_use]
    pub fn recognize(&self, strokes: &[Vec<(f32, f32)>]) -> Vec<RecognitionResult> {
        let feats = features(strokes);
        if feats.is_empty() {
            return Vec::new();
        }
        let uc = strokes.len();
        let (w, win) = (self.weights, &self.window);
        let mut r: Vec<RecognitionResult> = self
            .models
            .par_iter()
            .filter(|(_, m)| admits(win, m.stroke_count, uc))
            .map(|(c, m)| RecognitionResult {
                character: *c,
                score: viterbi_energy(m, &feats, &w),
            })
            .collect();
        r.sort_by(|a, b| a.score.total_cmp(&b.score));
        let size = frame_size(strokes);
        for res in &mut r {
            res.character = resize_kana(res.character, size, &w);
        }
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
    pub fn model_stroke_counts(&self) -> Vec<usize> {
        self.models.iter().map(|(_, m)| m.stroke_count).collect()
    }

    #[inline]
    pub fn upsert_preset(
        &mut self,
        cap: usize,
        weights: Weights,
        window: Vec<(u8, u8)>,
        stats: PresetStats,
    ) {
        let p = Preset {
            cap,
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
        self.weights = chosen.weights;
        self.window.clone_from(&chosen.window);
        Some(chosen.key())
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
        postcard::from_bytes::<Self>(bytes)
    }

    #[inline]
    pub fn set_small_threshold(&mut self, threshold: f64) {
        self.weights.small_threshold = threshold;
    }
}

/// Models are trained on full-size kana, so a drawing small enough to be a small
/// variant is remapped after scoring rather than being a separate model.
#[inline]
fn resize_kana(c: char, size: f32, w: &Weights) -> char {
    if f64::from(size) <= w.small_threshold {
        to_small(c).unwrap_or(c)
    } else {
        c
    }
}

/// Full-size kana to its small variant.
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
    fn normalizing_makes_the_weights_sum_to_one() {
        let w = Weights {
            term: [2.0, 3.0, 5.0],
            transition: 10.0,
            small_threshold: 0.3,
        }
        .normalized();
        let sum = w.term.iter().sum::<f64>() + w.transition;
        assert!((sum - 1.0).abs() < 1e-12, "{sum}");
        assert!((w.small_threshold - 0.3).abs() < 1e-12);
    }

    #[test]
    fn normalizing_nothing_falls_back_to_the_default() {
        let w = Weights {
            term: [0.0; N_TERMS],
            transition: 0.0,
            small_threshold: 0.42,
        }
        .normalized();
        assert_eq!(w.term, Weights::default().term);
        assert!((w.small_threshold - 0.42).abs() < 1e-12);
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
    fn a_small_drawing_is_remapped_and_a_large_one_is_not() {
        let w = Weights::default();
        assert_eq!(resize_kana('つ', 0.2, &w), 'っ');
        assert_eq!(resize_kana('つ', 0.9, &w), 'つ');
        assert_eq!(resize_kana('食', 0.2, &w), '食');
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
        let raws: Vec<RawStrokes> = vec![
            vec![Vec::new(); 3],
            vec![Vec::new(); 3],
            vec![Vec::new(); 5],
        ];
        assert_eq!(common_stroke_count(&raws), 3);
        assert_eq!(common_stroke_count(&[]), 1);
    }

    #[test]
    fn a_model_survives_a_round_trip_through_bytes() {
        let mut rec = Recognizer {
            weights: Weights::default(),
            window: default_window(),
            models: vec![('あ', CharModel::from_template(&features(&box_stroke()), 2))],
            presets: Vec::new(),
        };
        rec.upsert_preset(
            20_000,
            Weights::default(),
            default_window(),
            PresetStats::default(),
        );
        let bytes = rec.to_bytes().expect("serialize");
        let back = Recognizer::load(&bytes).expect("deserialize");
        assert_eq!(back.model_count(), 1);
        assert_eq!(back.model_stroke_counts(), vec![2]);
        assert_eq!(back.list_presets(), vec![20]);
    }

    #[test]
    fn a_preset_key_is_its_budget_in_thousands() {
        let p = Preset {
            cap: 20_000,
            weights: Weights::default(),
            window: default_window(),
            stats: PresetStats::default(),
        };
        assert_eq!(p.key(), 20);
        assert!(p.describe().contains("key  20"));
    }
}
