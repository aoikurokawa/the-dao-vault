use core::iter::FromIterator;

use strum::IntoEnumIterator;

use crate::reserves::{Provider, ProviderIter};

use super::AssetContainerGeneric;

pub struct AssetContainerIterator<'inner, T, const N: usize> {
    inner: &'inner AssetContainerGeneric<T, N>,
    inner_iter: ProviderIter,
}

impl<'inner, T, const N: usize> Iterator for AssetContainerIterator<'inner, T, N> {
    type Item = (Provider, &'inner T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .map(|provider| (provider, &self.inner[provider]))
    }
}

pub struct OwnedAssetContainerIterator<T, const N: usize> {
    inner: AssetContainerGeneric<T, N>,
    inner_iter: ProviderIter,
}

impl<'inner, T, const N: usize> Iterator for OwnedAssetContainerIterator<T, N> {
    type Item = (Provider, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next().map(|provider| {
            (
                provider,
                self.inner.inner[provider as usize]
                    .take()
                    .expect("missing index in OwnedAssetContainerIterator"),
            )
        })
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a AssetContainerGeneric<T, N> {
    type Item = (Provider, &'a T);
    type IntoIter = AssetContainerIterator<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        AssetContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

impl<T, const N: usize> IntoIterator for AssetContainerGeneric<T, N> {
    type Item = (Provider, T);
    type IntoIter = OwnedAssetContainerIterator<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        OwnedAssetContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

// Allows us to create a AssetContainerGeneric<T, N> from an Iterator that yields (Provider, T)
impl<T: Default, const N: usize> FromIterator<(Provider, T)> for AssetContainerGeneric<T, N> {
    fn from_iter<U: IntoIterator<Item = (Provider, T)>>(iter: U) -> Self {
        iter.into_iter().fold(
            AssetContainerGeneric::default(),
            |mut acc, (provider, v)| {
                acc[provider] = v;
                acc
            },
        )
    }
}
