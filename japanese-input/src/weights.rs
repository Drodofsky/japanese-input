use crate::{leaf_score::LEAF_FEATURE_COUNT, shape::HARMONICS};

/// Leaf weights followed by missing, extra, then the five group weights.
pub const WEIGHT_COUNT: usize = LEAF_FEATURE_COUNT + 8;

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Weights {
    pub direction_weight: f64,
    pub sideways_weights: [f64; HARMONICS],
    pub along_weights: [f64; HARMONICS],
    pub length_weight: f64,
    pub missing_penalty: f64,
    pub extra_penalty: f64,
    pub order_weight: f64,
    pub group_weight: f64,
    pub contiguity_weight: f64,
    pub rel_length_weight: f64,
    pub abs_position_weight: f64,
    pub merge_penalty: f64,
}

impl Default for Weights {
    #[inline]
    fn default() -> Self {
        Weights::v2()
    }
}

impl Weights {
    #[must_use]
    #[inline]
    pub fn ones() -> Self {
        Self {
            direction_weight: 1.0,
            sideways_weights: [1.0; HARMONICS],
            along_weights: [1.0; HARMONICS],
            length_weight: 1.0,
            missing_penalty: 1.0,
            extra_penalty: 1.0,
            order_weight: 1.0,
            group_weight: 1.0,
            contiguity_weight: 1.0,
            rel_length_weight: 1.0,
            abs_position_weight: 1.0,
            merge_penalty: 1.0,
        }
    }

    #[must_use]
    #[inline]
    pub fn v1() -> Self {
        Self {
            direction_weight: 0.0196078431372549,
            sideways_weights: [
                0.049019607843137254,
                0.029411764705882353,
                0.00980392156862745,
            ],
            along_weights: [0.0196078431372549, 0.00980392156862745, 0.00980392156862745],
            length_weight: 0.0196078431372549,
            missing_penalty: 1.6,
            extra_penalty: 0.4,
            order_weight: 0.08823529411764706,
            group_weight: 0.21568627450980393,
            contiguity_weight: 0.0784313725490196,
            rel_length_weight: 0.22549019607843138,
            abs_position_weight: 0.22549019607843138,
            merge_penalty: 0.5,
        }
    }

    #[must_use]
    #[inline]
    pub fn v2() -> Self {
        Self {
            direction_weight: 0.2196870744641536,
            sideways_weights: [
                0.029411899941490408,
                0.0037412724027624687,
                0.004546855492679897,
            ],
            along_weights: [
                0.014516485285438164,
                0.0027165433646208414,
                0.005768337404960056,
            ],
            length_weight: 0.3377468482731969,
            missing_penalty: 0.47234405360112686,
            extra_penalty: 0.6757718453935307,
            order_weight: 0.007965979093542764,
            group_weight: 0.26267909647654614,
            contiguity_weight: 0.0016887342413659837,
            rel_length_weight: 0.03648309204359259,
            abs_position_weight: 0.07304778151565021,
            merge_penalty: 0.3382224250229905,
        }
    }

    #[must_use]
    #[inline]
    pub fn leaf(&self) -> [f64; LEAF_FEATURE_COUNT] {
        let mut weights = [0.0_f64; LEAF_FEATURE_COUNT];
        let mut slot = weights.iter_mut();
        write_next(&mut slot, self.direction_weight);
        for weight in self.sideways_weights {
            write_next(&mut slot, weight);
        }
        for weight in self.along_weights {
            write_next(&mut slot, weight);
        }
        write_next(&mut slot, self.length_weight);
        weights
    }

    #[must_use]
    #[inline]
    pub fn to_array(&self) -> [f64; WEIGHT_COUNT] {
        let mut values = [0.0_f64; WEIGHT_COUNT];
        let mut slot = values.iter_mut();
        for weight in self.leaf() {
            write_next(&mut slot, weight);
        }
        write_next(&mut slot, self.missing_penalty);
        write_next(&mut slot, self.extra_penalty);
        write_next(&mut slot, self.order_weight);
        write_next(&mut slot, self.group_weight);
        write_next(&mut slot, self.contiguity_weight);
        write_next(&mut slot, self.rel_length_weight);
        write_next(&mut slot, self.abs_position_weight);
        write_next(&mut slot, self.merge_penalty);
        values
    }

    #[must_use]
    #[inline]
    pub fn to_vec(&self) -> Vec<f64> {
        self.to_array().to_vec()
    }

    #[must_use]
    #[inline]
    pub fn normalized(&self) -> Option<Self> {
        let mut values = self.to_array();
        let total: f64 = feature_slots()
            .filter_map(|index| values.get(index).copied())
            .sum();
        if total <= 1e-12_f64 || !total.is_finite() {
            return None;
        }
        for index in feature_slots() {
            if let Some(slot) = values.get_mut(index) {
                *slot /= total;
            }
        }
        Self::try_from(values.as_slice()).ok()
    }
}

impl TryFrom<&[f64]> for Weights {
    type Error = String;

    #[inline]
    fn try_from(value: &[f64]) -> Result<Self, Self::Error> {
        Ok(Weights {
            direction_weight: at(value, 0)?,
            sideways_weights: triple(value, 1)?,
            along_weights: triple(value, HARMONICS.saturating_add(1))?,
            length_weight: at(value, LEAF_FEATURE_COUNT.saturating_sub(1))?,
            missing_penalty: at(value, LEAF_FEATURE_COUNT)?,
            extra_penalty: at(value, LEAF_FEATURE_COUNT.saturating_add(1))?,
            order_weight: at(value, LEAF_FEATURE_COUNT.saturating_add(2))?,
            group_weight: at(value, LEAF_FEATURE_COUNT.saturating_add(3))?,
            contiguity_weight: at(value, LEAF_FEATURE_COUNT.saturating_add(4))?,
            rel_length_weight: at(value, LEAF_FEATURE_COUNT.saturating_add(5))?,
            abs_position_weight: at(value, LEAF_FEATURE_COUNT.saturating_add(6))?,
            merge_penalty: at(value, LEAF_FEATURE_COUNT.saturating_add(7))?,
        })
    }
}

#[inline]
pub fn feature_slots() -> impl Iterator<Item = usize> {
    (0..LEAF_FEATURE_COUNT)
        .chain(LEAF_FEATURE_COUNT.saturating_add(2)..WEIGHT_COUNT.saturating_sub(1))
}

#[inline]
pub fn penalty_slots() -> impl Iterator<Item = usize> {
    (LEAF_FEATURE_COUNT..LEAF_FEATURE_COUNT.saturating_add(2))
        .chain(WEIGHT_COUNT.saturating_sub(1)..WEIGHT_COUNT)
}

#[inline]
fn at(value: &[f64], index: usize) -> Result<f64, String> {
    let weight = *value
        .get(index)
        .ok_or_else(|| format!("expected {WEIGHT_COUNT} weights, found {}", value.len()))?;
    if !weight.is_finite() || weight < 0.0_f64 {
        return Err(format!("weight {index} is negative or not finite"));
    }
    Ok(weight)
}

#[inline]
fn triple(value: &[f64], start: usize) -> Result<[f64; HARMONICS], String> {
    let mut weights = [0.0_f64; HARMONICS];
    for (offset, slot) in weights.iter_mut().enumerate() {
        *slot = at(value, start.saturating_add(offset))?;
    }
    Ok(weights)
}

#[inline]
fn write_next<'slot>(slots: &mut impl Iterator<Item = &'slot mut f64>, value: f64) {
    if let Some(slot) = slots.next() {
        *slot = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_eight_leaf_weights_and_five_group_weights() {
        assert_eq!(LEAF_FEATURE_COUNT, 8);
        assert_eq!(WEIGHT_COUNT, 16);
        assert_eq!(Weights::v1().to_vec().len(), WEIGHT_COUNT);
    }

    #[test]
    fn a_round_trip_through_a_slice_keeps_every_weight() {
        for weights in [Weights::ones(), Weights::v1(), Weights::default()] {
            let restored = Weights::try_from(weights.to_vec().as_slice()).expect("weights");
            assert_eq!(weights, restored);
        }
    }

    #[test]
    fn the_flat_order_puts_sideways_before_along_and_length_last() {
        let weights = Weights::v1();
        let flat = weights.to_vec();
        assert_eq!(flat.first().copied(), Some(weights.direction_weight));
        assert_eq!(flat.get(1..4), Some(weights.sideways_weights.as_slice()));
        assert_eq!(flat.get(4..7), Some(weights.along_weights.as_slice()));
        assert_eq!(flat.get(7).copied(), Some(weights.length_weight));
        assert_eq!(flat.get(8).copied(), Some(weights.missing_penalty));
        assert_eq!(flat.get(9).copied(), Some(weights.extra_penalty));
        assert_eq!(flat.get(10).copied(), Some(weights.order_weight));
        assert_eq!(flat.get(11).copied(), Some(weights.group_weight));
        assert_eq!(flat.get(12).copied(), Some(weights.contiguity_weight));
        assert_eq!(flat.get(13).copied(), Some(weights.rel_length_weight));
        assert_eq!(flat.get(14).copied(), Some(weights.abs_position_weight));
        assert_eq!(flat.get(15).copied(), Some(weights.merge_penalty));
    }

    #[test]
    fn the_leaf_slice_matches_the_leading_flat_weights() {
        let weights = Weights::v1();
        assert_eq!(
            weights.to_vec().get(..LEAF_FEATURE_COUNT),
            Some(weights.leaf().as_slice())
        );
    }

    #[test]
    fn a_short_slice_is_refused() {
        assert!(Weights::try_from([1.0, 2.0].as_slice()).is_err());
        let one_short = vec![1.0_f64; WEIGHT_COUNT.saturating_sub(1)];
        assert!(Weights::try_from(one_short.as_slice()).is_err());
    }

    #[test]
    fn a_longer_slice_keeps_only_the_leading_weights() {
        let values = vec![1.0_f64; WEIGHT_COUNT.saturating_add(5)];
        let weights = Weights::try_from(values.as_slice()).expect("weights");
        assert_eq!(weights, Weights::ones());
    }

    #[test]
    fn a_negative_weight_is_refused() {
        for index in 0..WEIGHT_COUNT {
            let mut values = Weights::v1().to_vec();
            if let Some(slot) = values.get_mut(index) {
                *slot = -1.0;
            }
            assert!(
                Weights::try_from(values.as_slice()).is_err(),
                "index {index} slipped through"
            );
        }
    }

    #[test]
    fn a_non_finite_weight_is_refused() {
        for bad in [f64::NAN, f64::INFINITY] {
            let mut values = Weights::v1().to_vec();
            if let Some(slot) = values.get_mut(3) {
                *slot = bad;
            }
            assert!(Weights::try_from(values.as_slice()).is_err());
        }
    }

    #[test]
    fn normalizing_makes_the_feature_weights_sum_to_one() {
        let weights = Weights::v1().normalized().expect("normalized");
        let values = weights.to_array();
        let total: f64 = feature_slots()
            .filter_map(|index| values.get(index).copied())
            .sum();
        assert!((total - 1.0).abs() < 1e-12, "{total}");
    }

    #[test]
    fn normalizing_leaves_the_two_penalties_untouched() {
        let weights = Weights::v1().normalized().expect("normalized");
        assert_eq!(weights.missing_penalty, Weights::v1().missing_penalty);
        assert_eq!(weights.extra_penalty, Weights::v1().extra_penalty);
    }

    #[test]
    fn normalizing_keeps_every_feature_weight_in_proportion() {
        let raw = Weights::v1();
        let normalized = raw.normalized().expect("normalized");
        let ratio = normalized.direction_weight / raw.direction_weight;
        for (left, right) in normalized
            .to_array()
            .iter()
            .zip(raw.to_array().iter())
            .enumerate()
            .filter(|(index, _)| feature_slots().any(|slot| slot == *index))
            .map(|(_, pair)| pair)
        {
            assert!((left / right - ratio).abs() < 1e-12);
        }
    }

    #[test]
    fn the_two_slot_groups_together_cover_every_weight_exactly_once() {
        let mut seen: Vec<usize> = feature_slots().chain(penalty_slots()).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..WEIGHT_COUNT).collect::<Vec<usize>>());
        assert_eq!(feature_slots().count(), 13);
        assert_eq!(penalty_slots().collect::<Vec<usize>>(), vec![8, 9, 15]);
    }

    #[test]
    fn a_penalty_far_above_one_survives_normalization_unchanged() {
        let mut values = Weights::v1().to_vec();
        if let Some(slot) = values.get_mut(8) {
            *slot = 250.0;
        }
        if let Some(slot) = values.get_mut(9) {
            *slot = 90.0;
        }
        let weights = Weights::try_from(values.as_slice())
            .expect("weights")
            .normalized()
            .expect("normalized");
        assert_eq!(weights.missing_penalty, 250.0);
        assert_eq!(weights.extra_penalty, 90.0);
        let flat = weights.to_array();
        let total: f64 = feature_slots()
            .filter_map(|index| flat.get(index).copied())
            .sum();
        assert!((total - 1.0).abs() < 1e-12, "{total}");
    }

    #[test]
    fn scaling_only_the_feature_weights_changes_nothing_after_normalization() {
        let base = Weights::v1().normalized().expect("base");
        let mut values = Weights::v1().to_vec();
        for slot in feature_slots() {
            if let Some(value) = values.get_mut(slot) {
                *value *= 37.0;
            }
        }
        let scaled = Weights::try_from(values.as_slice())
            .expect("weights")
            .normalized()
            .expect("scaled");
        for (left, right) in base.to_array().iter().zip(scaled.to_array().iter()) {
            assert!((left - right).abs() < 1e-12, "{left} vs {right}");
        }
    }

    #[test]
    fn the_starting_weights_are_already_normalized() {
        let values = Weights::v1().to_array();
        let total: f64 = feature_slots()
            .filter_map(|index| values.get(index).copied())
            .sum();
        assert!((total - 1.0).abs() < 1e-9, "feature weights sum to {total}");
    }

    #[test]
    fn normalizing_is_idempotent() {
        let once = Weights::v1().normalized().expect("once");
        let twice = once.normalized().expect("twice");
        for (left, right) in once.to_array().iter().zip(twice.to_array().iter()) {
            assert!((left - right).abs() < 1e-12, "{left} vs {right}");
        }
    }

    #[test]
    fn normalizing_all_zero_feature_weights_is_refused() {
        let mut values = vec![0.0_f64; WEIGHT_COUNT];
        if let Some(slot) = values.get_mut(LEAF_FEATURE_COUNT) {
            *slot = 1.0;
        }
        let weights = Weights::try_from(values.as_slice()).expect("weights");
        assert!(weights.normalized().is_none());
    }

    #[test]
    fn a_zero_weight_is_allowed() {
        let values = vec![0.0_f64; WEIGHT_COUNT];
        let weights = Weights::try_from(values.as_slice()).expect("weights");
        assert!(weights.leaf().iter().all(|weight| *weight == 0.0));
    }
}
