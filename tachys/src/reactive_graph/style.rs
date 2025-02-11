use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{html::style::IntoStyle, renderer::Rndr};
use reactive_graph::effect::RenderEffect;
use std::borrow::Cow;

pub struct RenderEffectWithCssStyleName<T>
where
    T: 'static,
{
    name: &'static str,
    effect: RenderEffect<T>,
}

impl<T> RenderEffectWithCssStyleName<T>
where
    T: 'static,
{
    fn new(name: &'static str, effect: RenderEffect<T>) -> Self {
        Self { effect, name }
    }
}

impl<F, S> IntoStyle for (&'static str, F)
where
    F: ReactiveFunction<Output = S>,
    S: Into<Cow<'static, str>> + 'static,
{
    type AsyncOutput = Self;
    type State = RenderEffectWithCssStyleName<(
        crate::renderer::types::CssStyleDeclaration,
        Cow<'static, str>,
    )>;
    type Cloneable = (&'static str, SharedReactiveFunction<S>);
    type CloneableOwned = (&'static str, SharedReactiveFunction<S>);

    fn to_html(self, style: &mut String) {
        let (name, mut f) = self;
        let value = f.invoke();
        style.push_str(name);
        style.push(':');
        style.push_str(&value.into());
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let (name, mut f) = self;
        let name = Rndr::intern(name);
        // TODO FROM_SERVER vs template
        let style = Rndr::style(el);
        RenderEffectWithCssStyleName::new(
            name,
            RenderEffect::new(move |prev| {
                let value = f.invoke().into();
                if let Some(mut state) = prev {
                    let (style, prev): &mut (
                        crate::renderer::types::CssStyleDeclaration,
                        Cow<'static, str>,
                    ) = &mut state;
                    if &value != prev {
                        Rndr::set_css_property(style, name, &value);
                    }
                    *prev = value;
                    state
                } else {
                    // only set the style in template mode
                    // in server mode, it's already been set
                    if !FROM_SERVER {
                        Rndr::set_css_property(&style, name, &value);
                    }
                    (style.clone(), value)
                }
            }),
        )
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, mut f) = self;
        let name = Rndr::intern(name);
        let style = Rndr::style(el);
        RenderEffectWithCssStyleName::new(
            name,
            RenderEffect::new(move |prev| {
                let value = f.invoke().into();
                if let Some(mut state) = prev {
                    let (style, prev): &mut (
                        crate::renderer::types::CssStyleDeclaration,
                        Cow<'static, str>,
                    ) = &mut state;
                    if &value != prev {
                        Rndr::set_css_property(style, name, &value);
                    }
                    *prev = value;
                    state
                } else {
                    // always set the style initially without checking
                    Rndr::set_css_property(&style, name, &value);
                    (style.clone(), value)
                }
            }),
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, mut f) = self;
        // Name might've updated:
        state.name = name;
        state.effect = RenderEffect::new_with_value(
            move |prev| {
                let value = f.invoke().into();
                if let Some(mut state) = prev {
                    let (style, prev) = &mut state;
                    if &value != prev {
                        Rndr::set_css_property(style, name, &value);
                    }
                    *prev = value;
                    state
                } else {
                    unreachable!()
                }
            },
            state.effect.take_value(),
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, self.1.into_shared())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, self.1.into_shared())
    }

    fn dry_resolve(&mut self) {
        self.1.invoke();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn reset(state: &mut Self::State) {
        let name = state.name;
        state.effect = RenderEffect::new_with_value(
            move |prev| {
                if let Some(mut state) = prev {
                    let (style, prev) = &mut state;
                    Rndr::remove_css_property(style, name);
                    *prev = Cow::Borrowed("");
                    state
                } else {
                    unreachable!()
                }
            },
            state.effect.take_value(),
        );
    }
}

impl<F, C> IntoStyle for F
where
    F: ReactiveFunction<Output = C>,
    C: IntoStyle + 'static,
    C::State: 'static,
{
    type AsyncOutput = C::AsyncOutput;
    type State = RenderEffect<C::State>;
    type Cloneable = SharedReactiveFunction<C>;
    type CloneableOwned = SharedReactiveFunction<C>;

    fn to_html(mut self, style: &mut String) {
        let value = self.invoke();
        value.to_html(style);
    }

    fn hydrate<const FROM_SERVER: bool>(
        mut self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        // TODO FROM_SERVER vs template
        let el = el.clone();
        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.hydrate::<FROM_SERVER>(&el)
            }
        })
    }

    fn build(mut self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.clone();
        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.build(&el)
            }
        })
    }

    fn rebuild(mut self, state: &mut Self::State) {
        let prev_value = state.take_value();
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&mut state);
                    state
                } else {
                    unreachable!()
                }
            },
            prev_value,
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_shared()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into_shared()
    }

    fn dry_resolve(&mut self) {
        self.invoke();
    }

    async fn resolve(mut self) -> Self::AsyncOutput {
        self.invoke().resolve().await
    }

    fn reset(state: &mut Self::State) {
        *state = RenderEffect::new_with_value(
            move |prev| {
                if let Some(mut state) = prev {
                    C::reset(&mut state);
                    state
                } else {
                    unreachable!()
                }
            },
            state.take_value(),
        );
    }
}

#[cfg(not(feature = "nightly"))]
mod stable {
    use crate::{
        reactive_graph::style::RenderEffectWithCssStyleName,
        renderer::{types::CssStyleDeclaration, Rndr},
    };

    macro_rules! style_signal {
        ($sig:ident) => {
            impl<C> IntoStyle for $sig<C>
            where
                $sig<C>: Get<Value = C>,
                C: IntoStyle + Clone + Send + Sync + 'static,
                C::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<Option<C::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let value = self.try_get();
                    value.to_html(style);
                }

                fn hydrate<const FROM_SERVER: bool>(self, el: &crate::renderer::types::Element) -> Self::State {
                    // TODO FROM_SERVER vs template
                    let el = el.clone();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.hydrate::<FROM_SERVER>(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.hydrate::<FROM_SERVER>(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(self, el: &crate::renderer::types::Element) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.build(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State) {
                    let prev_value = state.take_value();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }

                fn dry_resolve(&mut self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

                fn reset(state: &mut Self::State) {
                    *state = RenderEffect::new_with_value(
                        move |prev| match (prev) {
                            Some(Some(mut state)) => {
                                C::reset(&mut state);
                                Some(state)
                            }
                            Some(None) => None,
                            None => None, // unreachable!()
                        },
                        state.take_value(),
                    );
                }
            }

            impl<S> IntoStyle for (&'static str, $sig<S>)
            where
                $sig<S>: Get<Value = S>,
                S: Into<Cow<'static, str>> + Send + Sync + Clone + 'static,
            {
                type AsyncOutput = Self;
                type State = crate::reactive_graph::style::RenderEffectWithCssStyleName<(
                    CssStyleDeclaration,
                    Option<Cow<'static, str>>,
                )>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let (name, f) = self;
                    let value = f.try_get();
                    if let Some(value) = value {
                        style.push_str(name);
                        style.push(':');
                        style.push_str(&value.into());
                        style.push(';');
                    }
                }

                fn hydrate<const FROM_SERVER: bool>(self, el: &crate::renderer::types::Element) -> Self::State {
                    let (name, f) = self;
                    let name = Rndr::intern(name);
                    // TODO FROM_SERVER vs template
                    let style = Rndr::style(el);
                    RenderEffectWithCssStyleName::new(
                        name,
                        RenderEffect::new(move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (None, Some(value)) => {
                                    if !FROM_SERVER {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    (style.clone(), Some(value))
                                }
                                _ => (style.clone(), None),
                            }
                        }),
                    )
                }

                fn build(self, el: &crate::renderer::types::Element) -> Self::State {
                    let (name, f) = self;
                    let name = Rndr::intern(name);
                    // TODO FROM_SERVER vs template
                    let style = Rndr::style(el);
                    RenderEffectWithCssStyleName::new(
                        name,
                        RenderEffect::new(move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (None, Some(value)) => {
                                    // always set the style initially without checking
                                    Rndr::set_css_property(&style, name, &value);
                                    (style.clone(), Some(value))
                                }
                                _ => (style.clone(), None),
                            }
                        }),
                    )
                }

                fn rebuild(self, state: &mut Self::State) {
                    let (name, f) = self;
                    // Name might've updated:
                    state.name = name;
                    state.effect = RenderEffect::new_with_value(
                        move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (Some((style, Some(prev))), None) => (style, Some(prev)),
                                (Some((style, None)), Some(_)) => (style, None),
                                (Some((style, None)), None) => (style, None),
                                (None, _) => unreachable!(),
                            }
                        },
                        state.effect.take_value(),
                    );
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }

                fn dry_resolve(&mut self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

                fn reset(state: &mut Self::State) {
                    let name = state.name;
                    *state = RenderEffectWithCssStyleName::new(
                        state.name,
                        RenderEffect::new_with_value(
                            move |prev| match (prev) {
                                Some((style, Some(mut prev))) => {
                                    crate::reactive_graph::Rndr::remove_css_property(&style, name);
                                    prev = Cow::Borrowed("");
                                    (style, Some(prev))
                                }
                                Some((style, None)) => (style, None),
                                None => unreachable!(),
                            },
                            state.effect.take_value(),
                        ),
                    );
                }
            }
        };
    }

    macro_rules! style_signal_arena {
        ($sig:ident) => {
            #[allow(deprecated)]
            impl<C, S> IntoStyle for $sig<C, S>
            where
                $sig<C, S>: Get<Value = C>,
                S: Storage<C> + Storage<Option<C>>,
                S: Send + Sync + 'static,
                C: IntoStyle + Send + Sync + Clone + 'static,
                C::State: 'static,
            {
                type AsyncOutput = Self;
                type State = RenderEffect<Option<C::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let value = self.try_get();
                    value.to_html(style);
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    // TODO FROM_SERVER vs template
                    let el = el.clone();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.hydrate::<FROM_SERVER>(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.hydrate::<FROM_SERVER>(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(self, el: &crate::renderer::types::Element) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.build(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State) {
                    let prev_value = state.take_value();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }

                fn dry_resolve(&mut self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

                fn reset(state: &mut Self::State) {
                    *state = RenderEffect::new_with_value(
                        move |prev| match (prev) {
                            Some(Some(mut state)) => {
                                C::reset(&mut state);
                                Some(state)
                            }
                            Some(None) => None,
                            None => None, // unreachable!()
                        },
                        state.take_value(),
                    );
                }
            }

            #[allow(deprecated)]
            impl<S, St> IntoStyle for (&'static str, $sig<S, St>)
            where
                $sig<S, St>: Get<Value = S>,
                St: Send + Sync + 'static,
                St: Storage<S> + Storage<Option<S>>,
                S: Into<Cow<'static, str>> + Send + Sync + Clone + 'static,
            {
                type AsyncOutput = Self;
                type State =
                    RenderEffectWithCssStyleName<(CssStyleDeclaration, Option<Cow<'static, str>>)>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let (name, f) = self;
                    let value = f.try_get();
                    if let Some(value) = value {
                        style.push_str(name);
                        style.push(':');
                        style.push_str(&value.into());
                        style.push(';');
                    }
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    let (name, f) = self;
                    let name = Rndr::intern(name);
                    // TODO FROM_SERVER vs template
                    let style = Rndr::style(el);
                    RenderEffectWithCssStyleName::new(
                        name,
                        RenderEffect::new(move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (None, Some(value)) => {
                                    if !FROM_SERVER {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    (style.clone(), Some(value))
                                }
                                _ => (style.clone(), None),
                            }
                        }),
                    )
                }

                fn build(self, el: &crate::renderer::types::Element) -> Self::State {
                    let (name, f) = self;
                    let name = Rndr::intern(name);
                    // TODO FROM_SERVER vs template
                    let style = Rndr::style(el);
                    RenderEffectWithCssStyleName::new(
                        name,
                        RenderEffect::new(move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (None, Some(value)) => {
                                    // always set the style initially without checking
                                    Rndr::set_css_property(&style, name, &value);
                                    (style.clone(), Some(value))
                                }
                                _ => (style.clone(), None),
                            }
                        }),
                    )
                }

                fn rebuild(self, state: &mut Self::State) {
                    let (name, f) = self;
                    // Name might've updated:
                    state.name = name;
                    state.effect = RenderEffect::new_with_value(
                        move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (Some((style, Some(prev))), None) => (style, Some(prev)),
                                (Some((style, None)), Some(_)) => (style, None),
                                (Some((style, None)), None) => (style, None),
                                (None, _) => unreachable!(),
                            }
                        },
                        state.effect.take_value(),
                    );
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }

                fn dry_resolve(&mut self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

                fn reset(state: &mut Self::State) {
                    let name = state.name;
                    *state = RenderEffectWithCssStyleName::new(
                        state.name,
                        RenderEffect::new_with_value(
                            move |prev| match (prev) {
                                Some((style, Some(mut prev))) => {
                                    crate::reactive_graph::Rndr::remove_css_property(&style, name);
                                    prev = Cow::Borrowed("");
                                    (style, Some(prev))
                                }
                                Some((style, None)) => (style, None),
                                None => unreachable!(),
                            },
                            state.effect.take_value(),
                        ),
                    );
                }
            }
        };
    }

    macro_rules! style_store_field {
        ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
        {
            impl<$($gen),*> IntoStyle for $name<$($gen),*>
            where
                $v: IntoStyle + Clone + Send + Sync + 'static,
                <$v as IntoStyle>::State: 'static,
                $($where_clause)*
            {
                type AsyncOutput = Self;
                type State = RenderEffect<Option<<$v as IntoStyle>::State>>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let value = self.try_get();
                    value.to_html(style);
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    el: &crate::renderer::types::Element,
                ) -> Self::State {
                    // TODO FROM_SERVER vs template
                    let el = el.clone();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        // Outer Some means there was a previous state
                        // Inner Some means the previous state was valid
                        // (i.e., the signal was successfully accessed)
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.hydrate::<FROM_SERVER>(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.hydrate::<FROM_SERVER>(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn build(self, el: &crate::renderer::types::Element) -> Self::State {
                    let el = el.to_owned();
                    RenderEffect::new(move |prev| {
                        let value = self.try_get();
                        match (prev, value) {
                            (Some(Some(mut state)), Some(value)) => {
                                value.rebuild(&mut state);
                                Some(state)
                            }
                            (None, Some(value)) => Some(value.build(&el)),
                            (Some(Some(state)), None) => Some(state),
                            (Some(None), Some(value)) => Some(value.build(&el)),
                            (Some(None), None) => None,
                            (None, None) => None,
                        }
                    })
                }

                fn rebuild(self, state: &mut Self::State) {
                    let prev_value = state.take_value();
                    *state = RenderEffect::new_with_value(
                        move |prev| {
                            let value = self.try_get();
                            match (prev, value) {
                                (Some(Some(mut state)), Some(value)) => {
                                    value.rebuild(&mut state);
                                    Some(state)
                                }
                                (Some(Some(state)), None) => Some(state),
                                (Some(None), Some(_)) => None,
                                (Some(None), None) => None,
                                (None, Some(_)) => None, // unreachable!()
                                (None, None) => None,    // unreachable!()
                            }
                        },
                        prev_value,
                    );
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }

                fn dry_resolve(&mut self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

                fn reset(state: &mut Self::State) {
                    *state = RenderEffect::new_with_value(
                        move |prev| match (prev) {
                            Some(Some(mut state)) => {
                                <$v>::reset(&mut state);
                                Some(state)
                            }
                            Some(None) => None,
                            None => None, // unreachable!()
                        },
                        state.take_value(),
                    );
                }
            }
        };
    }

    macro_rules! tuple_style_store_field {
        ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
        {
            impl<$($gen),*> IntoStyle for (&'static str, $name<$($gen),*>)
            where
                $v: Into<Cow<'static, str>> + Send + Sync + Clone + 'static,
                $($where_clause)*
            {
                type AsyncOutput = Self;
                type State = crate::reactive_graph::style::RenderEffectWithCssStyleName<(
                    CssStyleDeclaration,
                    Option<Cow<'static, str>>,
                )>;
                type Cloneable = Self;
                type CloneableOwned = Self;

                fn to_html(self, style: &mut String) {
                    let (name, f) = self;
                    let value = f.try_get();
                    if let Some(value) = value {
                        style.push_str(name);
                        style.push(':');
                        style.push_str(&value.into());
                        style.push(';');
                    }
                }

                fn hydrate<const FROM_SERVER: bool>(self, el: &crate::renderer::types::Element) -> Self::State {
                    let (name, f) = self;
                    let name = Rndr::intern(name);
                    // TODO FROM_SERVER vs template
                    let style = Rndr::style(el);
                    RenderEffectWithCssStyleName::new(
                        name,
                        RenderEffect::new(move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (None, Some(value)) => {
                                    if !FROM_SERVER {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    (style.clone(), Some(value))
                                }
                                _ => (style.clone(), None),
                            }
                        }),
                    )
                }

                fn build(self, el: &crate::renderer::types::Element) -> Self::State {
                    let (name, f) = self;
                    let name = Rndr::intern(name);
                    // TODO FROM_SERVER vs template
                    let style = Rndr::style(el);
                    RenderEffectWithCssStyleName::new(
                        name,
                        RenderEffect::new(move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (None, Some(value)) => {
                                    // always set the style initially without checking
                                    Rndr::set_css_property(&style, name, &value);
                                    (style.clone(), Some(value))
                                }
                                _ => (style.clone(), None),
                            }
                        }),
                    )
                }

                fn rebuild(self, state: &mut Self::State) {
                    let (name, f) = self;
                    // Name might've updated:
                    state.name = name;
                    state.effect = RenderEffect::new_with_value(
                        move |prev| {
                            let value = f.try_get().map(Into::into);
                            match (prev, value) {
                                (Some((style, Some(mut prev))), Some(value)) => {
                                    if value != prev {
                                        Rndr::set_css_property(&style, name, &value);
                                    }
                                    prev = value;
                                    (style, Some(prev))
                                }
                                (Some((style, Some(prev))), None) => (style, Some(prev)),
                                (Some((style, None)), Some(_)) => (style, None),
                                (Some((style, None)), None) => (style, None),
                                (None, _) => unreachable!(),
                            }
                        },
                        state.effect.take_value(),
                    );
                }

                fn into_cloneable(self) -> Self::Cloneable {
                    self
                }

                fn into_cloneable_owned(self) -> Self::CloneableOwned {
                    self
                }

                fn dry_resolve(&mut self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

                fn reset(state: &mut Self::State) {
                    let name = state.name;
                    *state = RenderEffectWithCssStyleName::new(
                        state.name,
                        RenderEffect::new_with_value(
                            move |prev| match (prev) {
                                Some((style, Some(mut prev))) => {
                                    crate::reactive_graph::Rndr::remove_css_property(&style, name);
                                    prev = Cow::Borrowed("");
                                    (style, Some(prev))
                                }
                                Some((style, None)) => (style, None),
                                None => unreachable!(),
                            },
                            state.effect.take_value(),
                        ),
                    );
                }
            }
        };
    }
    use super::RenderEffect;
    use crate::html::style::IntoStyle;
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    use reactive_stores::{
        ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field,
        KeyedSubfield, Store, StoreField, Subfield,
    };
    use std::{
        borrow::Cow,
        ops::{Deref, DerefMut, Index, IndexMut},
    };

    style_store_field!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );
    style_store_field!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );
    style_store_field!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );
    style_store_field!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );

    style_store_field!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    tuple_style_store_field!(
        Subfield,
        <Inner, Prev, V>,
        V,
        Subfield<Inner, Prev, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
    );
    tuple_style_store_field!(
        AtKeyed,
        <Inner, Prev, K, V>,
        V,
        AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );
    tuple_style_store_field!(
        KeyedSubfield,
        <Inner, Prev, K, V>,
        V,
        KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
        Prev: Send + Sync + 'static,
        Inner: Send + Sync + Clone + 'static,
        K: Send + Sync + std::fmt::Debug + Clone + 'static,
        for<'a> &'a V: IntoIterator,
    );
    tuple_style_store_field!(
        DerefedField,
        <S>,
        <S::Value as Deref>::Target,
        S: Clone + StoreField + Send + Sync + 'static,
        <S as StoreField>::Value: Deref + DerefMut
    );

    tuple_style_store_field!(
        AtIndex,
        <Inner, Prev>,
        <Prev as Index<usize>>::Output,
        AtIndex<Inner, Prev>: Get<Value = Prev::Output>,
        Prev: Send + Sync + IndexMut<usize> + 'static,
        Inner: Send + Sync + Clone + 'static,
    );

    style_signal_arena!(Store);
    style_signal_arena!(Field);
    style_signal_arena!(RwSignal);
    style_signal_arena!(ReadSignal);
    style_signal_arena!(Memo);
    style_signal_arena!(Signal);
    style_signal_arena!(MaybeSignal);
    style_signal!(ArcStore);
    style_signal!(ArcField);
    style_signal!(ArcRwSignal);
    style_signal!(ArcReadSignal);
    style_signal!(ArcMemo);
    style_signal!(ArcSignal);
}

/*
impl<Fut> IntoStyle for Suspend<Fut>
where
    Fut: Clone + Future + Send + 'static,
    Fut::Output: IntoStyle,
{
    type AsyncOutput = Fut::Output;
    type State = Rc<RefCell<Option<<Fut::Output as IntoStyle>::State>>>;
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_html(self, style: &mut String) {
        if let Some(inner) = self.inner.now_or_never() {
            inner.to_html(style);
        } else {
            panic!("You cannot use Suspend on an attribute outside Suspense");
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let el = el.to_owned();
        let state = Rc::new(RefCell::new(None));
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(&state);
            async move {
                *state.borrow_mut() =
                    Some(self.inner.await.hydrate::<FROM_SERVER>(&el));
                self.subscriber.forward();
            }
        });
        state
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.to_owned();
        let state = Rc::new(RefCell::new(None));
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(&state);
            async move {
                *state.borrow_mut() = Some(self.inner.await.build(&el));
                self.subscriber.forward();
            }
        });
        state
    }

    fn rebuild(self, state: &mut Self::State) {
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(state);
            async move {
                let value = self.inner.await;
                let mut state = state.borrow_mut();
                if let Some(state) = state.as_mut() {
                    value.rebuild(state);
                }
                self.subscriber.forward();
            }
        });
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self.inner.await
    }
}
*/
