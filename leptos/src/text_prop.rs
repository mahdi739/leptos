use oco_ref::Oco;
use reactive_graph::{owner::Storage, prelude::Get};
use std::sync::Arc;
use tachys::prelude::IntoAttributeValue;

/// Describes a value that is either a static or a reactive string, i.e.,
/// a [`String`], a [`&str`], a `Signal`, a `Store` or a reactive `Fn() -> String`.
#[derive(Clone)]
pub struct TextProp(Arc<dyn Fn() -> Oco<'static, str> + Send + Sync>);

impl TextProp {
    /// Accesses the current value of the property.
    #[inline(always)]
    pub fn get(&self) -> Oco<'static, str> {
        (self.0)()
    }
}

impl core::fmt::Debug for TextProp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TextProp").finish()
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        let s: Oco<'_, str> = Oco::Counted(Arc::from(s));
        TextProp(Arc::new(move || s.clone()))
    }
}

impl From<&'static str> for TextProp {
    fn from(s: &'static str) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Arc::new(move || s.clone()))
    }
}

impl From<Arc<str>> for TextProp {
    fn from(s: Arc<str>) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Arc::new(move || s.clone()))
    }
}

impl From<Oco<'static, str>> for TextProp {
    fn from(s: Oco<'static, str>) -> Self {
        TextProp(Arc::new(move || s.clone()))
    }
}

// TODO
/*impl<T> From<T> for MaybeProp<TextProp>
where
    T: Into<Oco<'static, str>>,
{
    fn from(s: T) -> Self {
        Self(Some(MaybeSignal::from(Some(s.into().into()))))
    }
}*/

impl<F, S> From<F> for TextProp
where
    F: Fn() -> S + 'static + Send + Sync,
    S: Into<Oco<'static, str>>,
{
    #[inline(always)]
    fn from(s: F) -> Self {
        TextProp(Arc::new(move || s().into()))
    }
}

impl Default for TextProp {
    fn default() -> Self {
        Self(Arc::new(|| Oco::Borrowed("")))
    }
}

impl IntoAttributeValue for TextProp {
    type Output = Arc<dyn Fn() -> Oco<'static, str> + Send + Sync>;

    fn into_attribute_value(self) -> Self::Output {
        self.0
    }
}

macro_rules! textprop_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> From<$name<$($gen),*>> for TextProp
        where
            $v: Into<Oco<'static, str>>  + Clone + Send + Sync + 'static,
            $($where_clause)*
        {
            #[inline(always)]
            fn from(s: $name<$($gen),*>) -> Self {
                TextProp(Arc::new(move || s.get().into()))
            }
        }
    };
}

#[cfg(not(feature = "nightly"))]
mod stable {
    use super::TextProp;
    use oco_ref::Oco;
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    use std::sync::Arc;

    textprop_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    textprop_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    textprop_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    textprop_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}

#[allow(deprecated)]
use reactive_stores::{
    ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field, KeyedSubfield,
    Store, StoreField, Subfield,
};
use std::ops::{Deref, DerefMut, Index, IndexMut};

textprop_reactive!(
    Subfield,
    <Inner, Prev, V>,
    V,
    Subfield<Inner, Prev, V>: Get<Value = V>,
    Prev: Send + Sync + 'static,
    Inner: Send + Sync + Clone + 'static,
);

textprop_reactive!(
    AtKeyed,
    <Inner, Prev, K, V>,
    V,
    AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
    Prev: Send + Sync + 'static,
    Inner: Send + Sync + Clone + 'static,
    K: Send + Sync + std::fmt::Debug + Clone + 'static,
    for<'a> &'a V: IntoIterator,
);

textprop_reactive!(
    KeyedSubfield,
    <Inner, Prev, K, V>,
    V,
    KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
    Prev: Send + Sync + 'static,
    Inner: Send + Sync + Clone + 'static,
    K: Send + Sync + std::fmt::Debug + Clone + 'static,
    for<'a> &'a V: IntoIterator,
);

textprop_reactive!(
    DerefedField,
    <S>,
    <S::Value as Deref>::Target,
    S: Clone + StoreField + Send + Sync + 'static,
    <S as StoreField>::Value: Deref + DerefMut
);

textprop_reactive!(
    AtIndex,
    <Inner, Prev>,
    <Prev as Index<usize>>::Output,
    AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
    Prev: Send + Sync + IndexMut<usize> + 'static,
    Inner: Send + Sync + Clone + 'static,
);
textprop_reactive!(
    Store,
    <V, S>,
    V,
    Store<V, S>: Get<Value = V>,
    S: Storage<V> + Storage<Option<V>>,
    S: Send + Sync + 'static,
);
textprop_reactive!(
    Field,
    <V, S>,
    V,
    Field<V, S>: Get<Value = V>,
    S: Storage<V> + Storage<Option<V>>,
    S: Send + Sync + 'static,
);
textprop_reactive!(ArcStore, <V>, V, ArcStore<V>: Get<Value = V>);
textprop_reactive!(ArcField, <V>, V, ArcField<V>: Get<Value = V>);

/// Extension trait for `Option<TextProp>`
pub trait OptionTextPropExt {
    /// Accesses the current value of the `Option<TextProp>` as an `Option<Oco<'static, str>>`.
    fn get(&self) -> Option<Oco<'static, str>>;
}

impl OptionTextPropExt for Option<TextProp> {
    fn get(&self) -> Option<Oco<'static, str>> {
        self.as_ref().map(|text_prop| text_prop.get())
    }
}
