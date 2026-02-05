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
    #[cfg(feature = "native_bridge")]
    pub enum EffectOutput {
        Render(<RenderOperation as ::crux_core::capability::Operation>::Output),
    }
    #[cfg(feature = "native_bridge")]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: EffectFfi,
    }
    #[cfg(feature = "native_bridge")]
    impl ::crux_core::EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;
        fn into_native(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => {
                    req.into_native(
                        EffectFfi::Render,
                        |o| match o {
                            EffectOutput::Render(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Render".to_string(),
                                })
                            }
                        },
                    )
                }
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
    #[cfg(feature = "native_bridge")]
    pub enum EffectOutput {
        Render(<RenderOperation as ::crux_core::capability::Operation>::Output),
    }
    #[cfg(feature = "native_bridge")]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: MyEffectFfi,
    }
    #[cfg(feature = "native_bridge")]
    impl ::crux_core::EffectNative for MyEffect {
        type Ffi = MyEffectFfi;
        type Output = EffectOutput;
        fn into_native(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
            match self {
                MyEffect::Render(req) => {
                    req.into_native(
                        MyEffectFfi::Render,
                        |o| match o {
                            EffectOutput::Render(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Render".to_string(),
                                })
                            }
                        },
                    )
                }
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
        facet(name = "Effect"),
        repr(C)
    )]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
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
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
    pub enum EffectOutput {
        Render(<RenderOperation as ::crux_core::capability::Operation>::Output),
    }
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Record))]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: EffectFfi,
    }
    #[cfg(feature = "native_bridge")]
    impl ::crux_core::EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;
        fn into_native(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => {
                    req.into_native(
                        EffectFfi::Render,
                        |o| match o {
                            EffectOutput::Render(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Render".to_string(),
                                })
                            }
                        },
                    )
                }
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
        facet(name = "MyEffect"),
        repr(C)
    )]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
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
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
    pub enum EffectOutput {
        Render(<RenderOperation as ::crux_core::capability::Operation>::Output),
    }
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Record))]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: MyEffectFfi,
    }
    #[cfg(feature = "native_bridge")]
    impl ::crux_core::EffectNative for MyEffect {
        type Ffi = MyEffectFfi;
        type Output = EffectOutput;
        fn into_native(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
            match self {
                MyEffect::Render(req) => {
                    req.into_native(
                        MyEffectFfi::Render,
                        |o| match o {
                            EffectOutput::Render(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Render".to_string(),
                                })
                            }
                        },
                    )
                }
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
    #[cfg(feature = "native_bridge")]
    pub enum EffectOutput {
        Render(<RenderOperation as ::crux_core::capability::Operation>::Output),
        Http(<HttpRequest as ::crux_core::capability::Operation>::Output),
    }
    #[cfg(feature = "native_bridge")]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: EffectFfi,
    }
    #[cfg(feature = "native_bridge")]
    impl ::crux_core::EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;
        fn into_native(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => {
                    req.into_native(
                        EffectFfi::Render,
                        |o| match o {
                            EffectOutput::Render(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Render".to_string(),
                                })
                            }
                        },
                    )
                }
                Effect::Http(req) => {
                    req.into_native(
                        EffectFfi::Http,
                        |o| match o {
                            EffectOutput::Http(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Http".to_string(),
                                })
                            }
                        },
                    )
                }
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
        facet(name = "Effect"),
        repr(C)
    )]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
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
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
    pub enum EffectOutput {
        Render(<RenderOperation as ::crux_core::capability::Operation>::Output),
        Http(<HttpRequest as ::crux_core::capability::Operation>::Output),
    }
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Record))]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: EffectFfi,
    }
    #[cfg(feature = "native_bridge")]
    impl ::crux_core::EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;
        fn into_native(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => {
                    req.into_native(
                        EffectFfi::Render,
                        |o| match o {
                            EffectOutput::Render(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Render".to_string(),
                                })
                            }
                        },
                    )
                }
                Effect::Http(req) => {
                    req.into_native(
                        EffectFfi::Http,
                        |o| match o {
                            EffectOutput::Http(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Http".to_string(),
                                })
                            }
                        },
                    )
                }
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
    "#);
}

#[test]
fn facet_typegen_with_namespace_attribute() {
    let args = Some(format_ident!("facet_typegen"));
    let input = parse_quote! {
        #[facet(namespace = "crux")]
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
        facet(namespace = "crux"),
        facet(name = "Effect"),
        repr(C)
    )]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
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
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
    pub enum EffectOutput {
        Render(<RenderOperation as ::crux_core::capability::Operation>::Output),
    }
    #[cfg(feature = "native_bridge")]
    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Record))]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: EffectFfi,
    }
    #[cfg(feature = "native_bridge")]
    impl ::crux_core::EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;
        fn into_native(
            self,
        ) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => {
                    req.into_native(
                        EffectFfi::Render,
                        |o| match o {
                            EffectOutput::Render(v) => Ok(v),
                            _ => {
                                Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                    expected: "Render".to_string(),
                                })
                            }
                        },
                    )
                }
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
