use anchor_lang::prelude::*;
use boolinator::Boolinator;
use solana_maths::{Rate, TryAdd};

use crate::errors::ErrorCode;

use super::AssetContainerGeneric;

impl<const N: usize> AssetContainerGeneric<Rate, N> {
    /// Return error if weights do not add up to 100%
    /// OR if you are greator than the allocation cap
    pub fn verify_weights(&self, allocation_cap_pct: u8) -> Result<()> {
        let cap = &Rate::from_percent(allocation_cap_pct);
        let max = self
            .into_iter()
            .map(|(_, r)| r)
            .max()
            .ok_or(ErrorCode::InvalidProposedWeights)?;

        let sum = self
            .into_iter()
            .map(|(_, r)| r)
            .try_fold(Rate::zero(), |acc, x| acc.try_add(*x))?;

        (sum == Rate::one() && max <= cap).ok_or_else(|| ErrorCode::InvalidProposedWeights.into())
    }
}

// Create new type as a wrapper to make this clear
impl<const N: usize> From<AssetContainerGeneric<u16, N>> for AssetContainerGeneric<Rate, N> {
    fn from(c: AssetContainerGeneric<u16, N>) -> Self {
        c.apply(|_, v| Rate::from_bips(u64::from(*v)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_weights_happy() {
        let rates = AssetContainerGeneric::<Rate, 3> {
            inner: [
                Some(Rate::from_percent(0)),
                Some(Rate::from_percent(0)),
                Some(Rate::from_percent(100)),
            ],
        };
        assert!(rates.verify_weights(100).is_ok())
    }

    #[test]
    fn test_verify_weights_happy2() {
        let rates = AssetContainerGeneric::<Rate, 3> {
            inner: [
                Some(Rate::from_percent(1)),
                Some(Rate::from_percent(59)),
                Some(Rate::from_percent(40)),
            ],
        };
        assert!(rates.verify_weights(59).is_ok())
    }

    #[test]
    fn test_verify_weights_unhappy_gt1() {
        let rates = AssetContainerGeneric::<Rate, 3> {
            inner: [
                Some(Rate::from_percent(2)),
                Some(Rate::from_percent(59)),
                Some(Rate::from_percent(40)),
            ],
        };
        assert!(matches!(rates.verify_weights(100), Err(_)));
        // Err(AnchorError(AnchorError { error_name: "InvalidProposedWeights", error_code_number: 6008, error_msg: "Proposed weights do not meet the required constraints", error_origin: None, compared_values: None }))
    }

    #[test]
    fn test_verify_weights_unhappy_alloc_cap() {
        let rates = AssetContainerGeneric::<Rate, 3> {
            inner: [
                Some(Rate::from_percent(1)),
                Some(Rate::from_percent(59)),
                Some(Rate::from_percent(40)),
            ],
        };
        assert!(matches!(rates.verify_weights(58), Err(_)));
    }
}
