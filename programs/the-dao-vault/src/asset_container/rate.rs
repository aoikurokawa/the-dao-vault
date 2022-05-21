use anchor_lang::prelude::Result;
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