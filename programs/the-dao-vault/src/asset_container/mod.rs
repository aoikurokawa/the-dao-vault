pub mod iter;
pub mod rate;
pub mod reserves;
pub mod u64;

pub use iter::*;
pub use rate::*;
pub use reserves::*;
pub use self::u64::*;

use core::ops::{Index, IndexMut};

use strum::{EnumCount, IntoEnumIterator};

use crate::reserves::Provider;

pub type AssetContainer<T> = AssetContainerGeneric<T, { Provider::COUNT }>;

/// Provides an abstraction over supported assets
#[derive(Debug, Clone)]
pub struct AssetContainerGeneric<T, const N: usize> {
    pub(crate) inner: [Option<T>; N],
}

impl<T, const N: usize> AssetContainerGeneric<T, N> {
    pub fn len(&self) -> usize {
        N
    }

    /// Returns if the container is uninitialized
    pub fn is_empty(&self) -> bool {
        self.inner.iter().all(Option::is_none)
    }
}

impl<T, const N: usize> Index<Provider> for AssetContainerGeneric<T, N> {
    type Output = T;

    fn index(&self, index: Provider) -> &Self::Output {
        self.inner[index as usize].as_ref().unwrap_or_else(|| {
            panic!(
                "missing index {:?} / {:?} in AssetContainerGeneric",
                index, index as usize
            )
        })
    }
}

impl<T, const N: usize> IndexMut<Provider> for AssetContainerGeneric<T, N> {
    fn index_mut(&mut self, index: Provider) -> &mut Self::Output {
        self.inner[index as usize].as_mut().unwrap_or_else(|| {
            panic!(
                "missing index {:?} / {:?} in AssetContainerGeneric",
                index, index as usize
            )
        })
    }
}

impl<T: Default, const N: usize> Default for AssetContainerGeneric<T, N> {
    fn default() -> Self {
        Self {
            inner: [(); N].map(|_| Some(T::default())),
        }
    }
}

impl<'a, T, const N: usize> From<&'a dyn Index<Provider, Output = &'a T>>
    for AssetContainerGeneric<&'a T, N>
where
    &'a T: Default,
{
    fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
        Provider::iter().fold(AssetContainerGeneric::default(), |mut acc, provider| {
            acc[provider] = p[provider];
            acc
        })
    }
}

impl<T, const N: usize> AssetContainerGeneric<T, N> {
    pub fn apply_owned<U: Clone + Default, F: Fn(Provider, T) -> U>(
        mut self,
        f: F,
    ) -> AssetContainerGeneric<U, N> {
        Provider::iter()
            .map(|provider| {
                (
                    provider,
                    f(
                        provider,
                        self.inner[provider as usize]
                            .take()
                            .expect("unable to take() in apply_owned()"),
                    ),
                )
            })
            .collect()
    }

    /// Applies 'f' to each element of the container individually, yielding a new container
    pub fn apply<U: Default, F: Fn(Provider, &T) -> U>(&self, f: F) -> AssetContainerGeneric<U, N> {
        // Because we have FromIterator<(Provider, T)> if we yield a tuple of
        // (Provider, U) we can collect() this into a AssetContainerGeneric<U>
        Provider::iter()
            .map(|provider| (provider, f(provider, &self[provider])))
            .collect()
    }

    /// Identical to 'apply' but returns a Result<AssetContainerGeneric<..>>
    pub fn try_apply<U: Default, E, F: Fn(Provider, &T) -> Result<U, E>>(
        &self,
        f: F,
    ) -> Result<AssetContainerGeneric<U, N>, E> {
        Provider::iter()
            .map(|provider| f(provider, &self[provider]).map(|res| (provider, res)))
            .collect()
    }
}
