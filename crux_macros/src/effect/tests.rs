#![expect(
    clippy::too_many_lines,
    reason = "snapshot tests embed full macro expansion"
)]

use quote::format_ident;
use syn::parse_quote;

use crate::pretty_print;

use super::macro_impl::*;

#[test]
#[should_panic(expected = "Unexpected attribute: typo, did you mean typegen or facet_typegen?")]
fn bad_args() {
    let args = Some(format_ident!("typo"));
    let input = parse_quote! {
        pub enum Effect {
            Render(RenderOperation),
        }
    };

    effect_impl(args, input);
}

#[test]
fn single_with_typegen() {
    let args = Some(format_ident!("typegen"));
    let input = parse_quote! {
        pub enum Effect {
            Render(RenderOperation),
        }
    };

    let actual = effect_impl(args, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename = "Effect")]
    pub enum EffectFfi {
        Render(RenderOperation),
    }
    impl crux_core::Effect for Effect {}
    impl crux_core::EffectFFI for Effect {
        type Ffi = EffectFfi;
        fn serialize<T: ::crux_core::bridge::FfiFormat>(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
            match self {
                Effect::Render(request) => request.serialize(EffectFfi::Render),
            }
        }
    }
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    #[cfg(feature = "typegen")]
    impl ::crux_core::type_generation::serde::Export for Effect {
        fn register_types(
            generator: &mut ::crux_core::type_generation::serde::TypeGen,
        ) -> ::crux_core::type_generation::serde::Result {
            use ::crux_core::capability::Operation;
            RenderOperation::register_types(generator)?;
            generator.register_type::<EffectFfi>()?;
            generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
            Ok(())
        }
    }
    "#);
}

#[test]
fn single_with_new_name() {
    let args = Some(format_ident!("typegen"));
    let input = parse_quote! {
        pub enum MyEffect {
            Render(RenderOperation),
        }
    };

    let actual = effect_impl(args, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum MyEffect {
        Render(::crux_core::Request<RenderOperation>),
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename = "MyEffect")]
    pub enum MyEffectFfi {
        Render(RenderOperation),
    }
    impl crux_core::Effect for MyEffect {}
    impl crux_core::EffectFFI for MyEffect {
        type Ffi = MyEffectFfi;
        fn serialize<T: ::crux_core::bridge::FfiFormat>(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
            match self {
                MyEffect::Render(request) => request.serialize(MyEffectFfi::Render),
            }
        }
    }
    impl From<::crux_core::Request<RenderOperation>> for MyEffect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<MyEffect> for ::crux_core::Request<RenderOperation> {
        type Error = MyEffect;
        fn try_from(value: MyEffect) -> Result<Self, Self::Error> {
            if let MyEffect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl MyEffect {
        pub fn is_render(&self) -> bool {
            if let MyEffect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let MyEffect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let MyEffect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    pub trait MyEffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> MyEffectTestExt<Event> for ::crux_core::Command<MyEffect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    #[cfg(feature = "typegen")]
    impl ::crux_core::type_generation::serde::Export for MyEffect {
        fn register_types(
            generator: &mut ::crux_core::type_generation::serde::TypeGen,
        ) -> ::crux_core::type_generation::serde::Result {
            use ::crux_core::capability::Operation;
            RenderOperation::register_types(generator)?;
            generator.register_type::<MyEffectFfi>()?;
            generator.register_type::<::crux_core::bridge::Request<MyEffectFfi>>()?;
            Ok(())
        }
    }
    "#);
}

#[test]
fn single_with_facet_typegen() {
    let args = Some(format_ident!("facet_typegen"));
    let input = parse_quote! {
        pub enum Effect {
            Render(RenderOperation),
        }
    };

    let actual = effect_impl(args, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename = "Effect")]
    #[cfg_attr(
        feature = "facet_typegen",
        derive(::facet::Facet),
        facet(rename = "Effect"),
        repr(C)
    )]
    pub enum EffectFfi {
        Render(RenderOperation),
    }
    impl crux_core::Effect for Effect {}
    impl crux_core::EffectFFI for Effect {
        type Ffi = EffectFfi;
        fn serialize<T: ::crux_core::bridge::FfiFormat>(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
            match self {
                Effect::Render(request) => request.serialize(EffectFfi::Render),
            }
        }
    }
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    #[cfg(feature = "facet_typegen")]
    impl ::crux_core::type_generation::facet::Export for Effect {
        fn register_types(
            generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
        ) -> Result<
            &mut ::crux_core::type_generation::facet::TypeRegistry,
            ::crux_core::type_generation::facet::TypeGenError,
        > {
            use ::crux_core::capability::Operation;
            let generator = RenderOperation::register_types_facet(generator)
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            generator
                .register_type::<EffectFfi>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?
                .register_type::<::crux_core::bridge::Request<EffectFfi>>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            Ok(generator)
        }
    }
    "#);
}

#[test]
fn single_facet_typegen_with_new_name() {
    let args = Some(format_ident!("facet_typegen"));
    let input = parse_quote! {
        pub enum MyEffect {
            Render(RenderOperation),
        }
    };

    let actual = effect_impl(args, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum MyEffect {
        Render(::crux_core::Request<RenderOperation>),
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename = "MyEffect")]
    #[cfg_attr(
        feature = "facet_typegen",
        derive(::facet::Facet),
        facet(rename = "MyEffect"),
        repr(C)
    )]
    pub enum MyEffectFfi {
        Render(RenderOperation),
    }
    impl crux_core::Effect for MyEffect {}
    impl crux_core::EffectFFI for MyEffect {
        type Ffi = MyEffectFfi;
        fn serialize<T: ::crux_core::bridge::FfiFormat>(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
            match self {
                MyEffect::Render(request) => request.serialize(MyEffectFfi::Render),
            }
        }
    }
    impl From<::crux_core::Request<RenderOperation>> for MyEffect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<MyEffect> for ::crux_core::Request<RenderOperation> {
        type Error = MyEffect;
        fn try_from(value: MyEffect) -> Result<Self, Self::Error> {
            if let MyEffect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl MyEffect {
        pub fn is_render(&self) -> bool {
            if let MyEffect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let MyEffect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let MyEffect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    pub trait MyEffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> MyEffectTestExt<Event> for ::crux_core::Command<MyEffect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    #[cfg(feature = "facet_typegen")]
    impl ::crux_core::type_generation::facet::Export for MyEffect {
        fn register_types(
            generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
        ) -> Result<
            &mut ::crux_core::type_generation::facet::TypeRegistry,
            ::crux_core::type_generation::facet::TypeGenError,
        > {
            use ::crux_core::capability::Operation;
            let generator = RenderOperation::register_types_facet(generator)
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            generator
                .register_type::<MyEffectFfi>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?
                .register_type::<::crux_core::bridge::Request<MyEffectFfi>>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            Ok(generator)
        }
    }
    "#);
}

#[test]
fn single_without_typegen() {
    let input = parse_quote! {
        pub enum Effect {
            Render(RenderOperation),
        }
    };

    let actual = effect_impl(None, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
    }
    impl crux_core::Effect for Effect {}
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    "#);
}

#[allow(clippy::too_many_lines)]
#[test]
fn multiple_with_typegen() {
    let args = Some(format_ident!("typegen"));
    let input = parse_quote! {
        pub enum Effect {
            Render(RenderOperation),
            Http(HttpRequest),
        }
    };

    let actual = effect_impl(args, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
        Http(::crux_core::Request<HttpRequest>),
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename = "Effect")]
    pub enum EffectFfi {
        Render(RenderOperation),
        Http(HttpRequest),
    }
    impl crux_core::Effect for Effect {}
    impl crux_core::EffectFFI for Effect {
        type Ffi = EffectFfi;
        fn serialize<T: ::crux_core::bridge::FfiFormat>(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
            match self {
                Effect::Render(request) => request.serialize(EffectFfi::Render),
                Effect::Http(request) => request.serialize(EffectFfi::Http),
            }
        }
    }
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl From<::crux_core::Request<HttpRequest>> for Effect {
        fn from(value: ::crux_core::Request<HttpRequest>) -> Self {
            Self::Http(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<HttpRequest> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Http(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    impl Effect {
        pub fn is_http(&self) -> bool {
            if let Effect::Http(_) = self { true } else { false }
        }
        pub fn into_http(self) -> Option<::crux_core::Request<HttpRequest>> {
            if let Effect::Http(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_http(self) -> ::crux_core::Request<HttpRequest> {
            if let Effect::Http(request) = self {
                request
            } else {
                panic!("not a {} effect", "Http")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn expect_http(&mut self) -> &mut Self;
        fn expect_http_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&HttpRequest);
        fn expect_only_http(&mut self);
        fn expect_only_http_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&HttpRequest);
        fn resolve_http<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &HttpRequest,
            ) -> <HttpRequest as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn expect_http(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let _ = effect.expect_http();
            self
        }
        #[track_caller]
        fn expect_http_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&HttpRequest),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let req = effect.expect_http();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_http(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let _ = effect.expect_http();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_http_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&HttpRequest),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let req = effect.expect_http();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_http<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &HttpRequest,
            ) -> <HttpRequest as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let mut req = effect.expect_http();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    #[cfg(feature = "typegen")]
    impl ::crux_core::type_generation::serde::Export for Effect {
        fn register_types(
            generator: &mut ::crux_core::type_generation::serde::TypeGen,
        ) -> ::crux_core::type_generation::serde::Result {
            use ::crux_core::capability::Operation;
            RenderOperation::register_types(generator)?;
            HttpRequest::register_types(generator)?;
            generator.register_type::<EffectFfi>()?;
            generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
            Ok(())
        }
    }
    "#);
}

#[allow(clippy::too_many_lines)]
#[test]
fn multiple_with_facet_typegen() {
    let args = Some(format_ident!("facet_typegen"));
    let input = parse_quote! {
        pub enum Effect {
            Render(RenderOperation),
            Http(HttpRequest),
        }
    };

    let actual = effect_impl(args, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
        Http(::crux_core::Request<HttpRequest>),
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename = "Effect")]
    #[cfg_attr(
        feature = "facet_typegen",
        derive(::facet::Facet),
        facet(rename = "Effect"),
        repr(C)
    )]
    pub enum EffectFfi {
        Render(RenderOperation),
        Http(HttpRequest),
    }
    impl crux_core::Effect for Effect {}
    impl crux_core::EffectFFI for Effect {
        type Ffi = EffectFfi;
        fn serialize<T: ::crux_core::bridge::FfiFormat>(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
            match self {
                Effect::Render(request) => request.serialize(EffectFfi::Render),
                Effect::Http(request) => request.serialize(EffectFfi::Http),
            }
        }
    }
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl From<::crux_core::Request<HttpRequest>> for Effect {
        fn from(value: ::crux_core::Request<HttpRequest>) -> Self {
            Self::Http(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<HttpRequest> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Http(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    impl Effect {
        pub fn is_http(&self) -> bool {
            if let Effect::Http(_) = self { true } else { false }
        }
        pub fn into_http(self) -> Option<::crux_core::Request<HttpRequest>> {
            if let Effect::Http(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_http(self) -> ::crux_core::Request<HttpRequest> {
            if let Effect::Http(request) = self {
                request
            } else {
                panic!("not a {} effect", "Http")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn expect_http(&mut self) -> &mut Self;
        fn expect_http_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&HttpRequest);
        fn expect_only_http(&mut self);
        fn expect_only_http_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&HttpRequest);
        fn resolve_http<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &HttpRequest,
            ) -> <HttpRequest as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn expect_http(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let _ = effect.expect_http();
            self
        }
        #[track_caller]
        fn expect_http_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&HttpRequest),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let req = effect.expect_http();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_http(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let _ = effect.expect_http();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_http_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&HttpRequest),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let req = effect.expect_http();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_http<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &HttpRequest,
            ) -> <HttpRequest as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let mut req = effect.expect_http();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    #[cfg(feature = "facet_typegen")]
    impl ::crux_core::type_generation::facet::Export for Effect {
        fn register_types(
            generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
        ) -> Result<
            &mut ::crux_core::type_generation::facet::TypeRegistry,
            ::crux_core::type_generation::facet::TypeGenError,
        > {
            use ::crux_core::capability::Operation;
            let generator = RenderOperation::register_types_facet(generator)
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            let generator = HttpRequest::register_types_facet(generator)
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            generator
                .register_type::<EffectFfi>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?
                .register_type::<::crux_core::bridge::Request<EffectFfi>>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            Ok(generator)
        }
    }
    "#);
}

#[test]
fn multiple_without_typegen() {
    let input = parse_quote! {
        pub enum Effect {
            Render(RenderOperation),
            Http(HttpRequest),
        }
    };

    let actual = effect_impl(None, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
        Http(::crux_core::Request<HttpRequest>),
    }
    impl crux_core::Effect for Effect {}
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl From<::crux_core::Request<HttpRequest>> for Effect {
        fn from(value: ::crux_core::Request<HttpRequest>) -> Self {
            Self::Http(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<HttpRequest> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Http(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    impl Effect {
        pub fn is_http(&self) -> bool {
            if let Effect::Http(_) = self { true } else { false }
        }
        pub fn into_http(self) -> Option<::crux_core::Request<HttpRequest>> {
            if let Effect::Http(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_http(self) -> ::crux_core::Request<HttpRequest> {
            if let Effect::Http(request) = self {
                request
            } else {
                panic!("not a {} effect", "Http")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn expect_http(&mut self) -> &mut Self;
        fn expect_http_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&HttpRequest);
        fn expect_only_http(&mut self);
        fn expect_only_http_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&HttpRequest);
        fn resolve_http<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &HttpRequest,
            ) -> <HttpRequest as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn expect_http(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let _ = effect.expect_http();
            self
        }
        #[track_caller]
        fn expect_http_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&HttpRequest),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let req = effect.expect_http();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_http(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let _ = effect.expect_http();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_http_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&HttpRequest),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let req = effect.expect_http();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_http<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &HttpRequest,
            ) -> <HttpRequest as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Http effect but no more effects remain")
                });
            let mut req = effect.expect_http();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    "#);
}

#[test]
fn single_without_typegen_with_attributes() {
    let input = parse_quote! {
        #[derive(Debug, PartialEq)]
        pub enum Effect {
            Render(RenderOperation),
        }
    };

    let actual = effect_impl(None, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    #[derive(Debug, PartialEq)]
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
    }
    impl crux_core::Effect for Effect {}
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    "#);
}

#[test]
fn facet_typegen_with_namespace_attribute() {
    let args = Some(format_ident!("facet_typegen"));
    let input = parse_quote! {
        #[facet(facet_generate_attrs::namespace = "crux")]
        pub enum Effect {
            Render(RenderOperation),
        }
    };

    let actual = effect_impl(args, input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    pub enum Effect {
        Render(::crux_core::Request<RenderOperation>),
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename = "Effect")]
    #[cfg_attr(
        feature = "facet_typegen",
        derive(::facet::Facet),
        facet(facet_generate_attrs::namespace = "crux"),
        facet(rename = "Effect"),
        repr(C)
    )]
    pub enum EffectFfi {
        Render(RenderOperation),
    }
    impl crux_core::Effect for Effect {}
    impl crux_core::EffectFFI for Effect {
        type Ffi = EffectFfi;
        fn serialize<T: ::crux_core::bridge::FfiFormat>(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
            match self {
                Effect::Render(request) => request.serialize(EffectFfi::Render),
            }
        }
    }
    impl From<::crux_core::Request<RenderOperation>> for Effect {
        fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }
    impl TryFrom<Effect> for ::crux_core::Request<RenderOperation> {
        type Error = Effect;
        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            if let Effect::Render(value) = value { Ok(value) } else { Err(value) }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        #[track_caller]
        pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
            if let Effect::Render(request) = self {
                request
            } else {
                panic!("not a {} effect", "Render")
            }
        }
    }
    pub trait EffectTestExt<Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        fn expect_render(&mut self) -> &mut Self;
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn expect_only_render(&mut self);
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation);
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output;
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event);
    }
    impl<Event> EffectTestExt<Event> for ::crux_core::Command<Effect, Event>
    where
        Event: ::core::marker::Send + 'static,
    {
        #[track_caller]
        fn expect_render(&mut self) -> &mut Self {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self
        }
        #[track_caller]
        fn expect_render_with<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self
        }
        #[track_caller]
        fn expect_only_render(&mut self) {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let _ = effect.expect_render();
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn expect_only_render_with<F>(&mut self, f: F)
        where
            F: ::core::ops::FnOnce(&RenderOperation),
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let req = effect.expect_render();
            f(&req.operation);
            self.expect_no_effect_or_events();
        }
        #[track_caller]
        fn resolve_render<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(
                &RenderOperation,
            ) -> <RenderOperation as ::crux_core::capability::Operation>::Output,
        {
            let effect = self
                .effects()
                .next()
                .unwrap_or_else(|| {
                    panic!("expected Render effect but no more effects remain")
                });
            let mut req = effect.expect_render();
            let output = f(&req.operation);
            req.resolve(output).expect("resolve failed");
            self
        }
        #[track_caller]
        fn then_event<F>(&mut self, f: F) -> &mut Self
        where
            F: ::core::ops::FnOnce(&Event),
        {
            let ev = self
                .events()
                .next()
                .unwrap_or_else(|| panic!("expected an event but got none"));
            f(&ev);
            self
        }
    }
    #[cfg(feature = "facet_typegen")]
    impl ::crux_core::type_generation::facet::Export for Effect {
        fn register_types(
            generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
        ) -> Result<
            &mut ::crux_core::type_generation::facet::TypeRegistry,
            ::crux_core::type_generation::facet::TypeGenError,
        > {
            use ::crux_core::capability::Operation;
            let generator = RenderOperation::register_types_facet(generator)
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            generator
                .register_type::<EffectFfi>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?
                .register_type::<::crux_core::bridge::Request<EffectFfi>>()
                .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    err.to_string(),
                ))?;
            Ok(generator)
        }
    }
    "#);
}
