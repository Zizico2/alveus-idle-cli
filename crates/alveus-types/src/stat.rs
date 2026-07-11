use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

/// Discrete units on the game's stat scale.
///
/// This type is direction-agnostic: it can be a stored value, an improvement
/// amount, or a worsening amount. Callers that mutate stored state remain
/// responsible for clamping to the configured scale.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Reflect,
    Serialize,
    Deserialize,
)]
#[type_path = "alveus_types"]
#[reflect(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Stat(pub u32);

impl Stat {
    pub const ZERO: Self = Self(0);

    pub const fn get(self) -> u32 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn saturating_add_capped(self, amount: Self, cap: Self) -> Self {
        let summed = self.0.saturating_add(amount.0);
        Self(if summed < cap.0 { summed } else { cap.0 })
    }

    pub const fn saturating_sub(self, amount: Self) -> Self {
        Self(self.0.saturating_sub(amount.0))
    }
}

impl From<Stat> for u32 {
    fn from(value: Stat) -> Self {
        value.0
    }
}

/// Hunger delta authored by a feed action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[type_path = "alveus_types"]
#[reflect(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FeedStat(pub Stat);

/// Happiness delta authored by an enrichment action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[type_path = "alveus_types"]
#[reflect(Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnrichStat(pub Stat);

/// Enclosure cleanliness delta authored by a cleaning action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[type_path = "alveus_types"]
#[reflect(Serialize, Deserialize)]
#[serde(transparent)]
pub struct CleanStat(pub Stat);

impl From<FeedStat> for Stat {
    fn from(value: FeedStat) -> Self {
        value.0
    }
}

impl From<EnrichStat> for Stat {
    fn from(value: EnrichStat) -> Self {
        value.0
    }
}

impl From<CleanStat> for Stat {
    fn from(value: CleanStat) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturating_add_capped_clamps_at_cap() {
        assert_eq!(
            Stat(900).saturating_add_capped(Stat(200), Stat(1000)),
            Stat(1000)
        );
    }

    #[test]
    fn saturating_add_capped_below_cap() {
        assert_eq!(
            Stat(500).saturating_add_capped(Stat(200), Stat(1000)),
            Stat(700)
        );
    }

    #[test]
    fn saturating_sub_floors_at_zero() {
        assert_eq!(Stat(100).saturating_sub(Stat(200)), Stat::ZERO);
    }

    #[test]
    fn saturating_sub_in_range() {
        assert_eq!(Stat(500).saturating_sub(Stat(200)), Stat(300));
    }

    #[test]
    fn action_wrappers_convert_to_inner_stat() {
        assert_eq!(Stat::from(FeedStat(Stat(1000))), Stat(1000));
        assert_eq!(Stat::from(EnrichStat(Stat(250))), Stat(250));
        assert_eq!(Stat::from(CleanStat(Stat(350))), Stat(350));
    }

    #[test]
    fn get_and_is_zero() {
        assert_eq!(Stat(42).get(), 42);
        assert!(Stat::ZERO.is_zero());
        assert!(!Stat(1).is_zero());
    }
}
