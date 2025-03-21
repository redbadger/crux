#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
pub mod app {
    use crux_core::{render::Render, App};
    use serde::{Deserialize, Serialize};
    pub enum Event {
        Increment,
        Decrement,
        Reset,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for Event {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                match *self {
                    Event::Increment => {
                        _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "Event",
                            0u32,
                            "Increment",
                        )
                    }
                    Event::Decrement => {
                        _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "Event",
                            1u32,
                            "Decrement",
                        )
                    }
                    Event::Reset => {
                        _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "Event",
                            2u32,
                            "Reset",
                        )
                    }
                }
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for Event {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "variant identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            2u64 => _serde::__private::Ok(__Field::__field2),
                            _ => {
                                _serde::__private::Err(
                                    _serde::de::Error::invalid_value(
                                        _serde::de::Unexpected::Unsigned(__value),
                                        &"variant index 0 <= i < 3",
                                    ),
                                )
                            }
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "Increment" => _serde::__private::Ok(__Field::__field0),
                            "Decrement" => _serde::__private::Ok(__Field::__field1),
                            "Reset" => _serde::__private::Ok(__Field::__field2),
                            _ => {
                                _serde::__private::Err(
                                    _serde::de::Error::unknown_variant(__value, VARIANTS),
                                )
                            }
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"Increment" => _serde::__private::Ok(__Field::__field0),
                            b"Decrement" => _serde::__private::Ok(__Field::__field1),
                            b"Reset" => _serde::__private::Ok(__Field::__field2),
                            _ => {
                                let __value = &_serde::__private::from_utf8_lossy(__value);
                                _serde::__private::Err(
                                    _serde::de::Error::unknown_variant(__value, VARIANTS),
                                )
                            }
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<Event>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = Event;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "enum Event",
                        )
                    }
                    fn visit_enum<__A>(
                        self,
                        __data: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::EnumAccess<'de>,
                    {
                        match _serde::de::EnumAccess::variant(__data)? {
                            (__Field::__field0, __variant) => {
                                _serde::de::VariantAccess::unit_variant(__variant)?;
                                _serde::__private::Ok(Event::Increment)
                            }
                            (__Field::__field1, __variant) => {
                                _serde::de::VariantAccess::unit_variant(__variant)?;
                                _serde::__private::Ok(Event::Decrement)
                            }
                            (__Field::__field2, __variant) => {
                                _serde::de::VariantAccess::unit_variant(__variant)?;
                                _serde::__private::Ok(Event::Reset)
                            }
                        }
                    }
                }
                #[doc(hidden)]
                const VARIANTS: &'static [&'static str] = &[
                    "Increment",
                    "Decrement",
                    "Reset",
                ];
                _serde::Deserializer::deserialize_enum(
                    __deserializer,
                    "Event",
                    VARIANTS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<Event>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl ::core::clone::Clone for Event {
        #[inline]
        fn clone(&self) -> Event {
            match self {
                Event::Increment => Event::Increment,
                Event::Decrement => Event::Decrement,
                Event::Reset => Event::Reset,
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Event {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Event::Increment => "Increment",
                    Event::Decrement => "Decrement",
                    Event::Reset => "Reset",
                },
            )
        }
    }
    pub struct Model {
        count: isize,
    }
    #[automatically_derived]
    impl ::core::default::Default for Model {
        #[inline]
        fn default() -> Model {
            Model {
                count: ::core::default::Default::default(),
            }
        }
    }
    pub struct ViewModel {
        pub count: String,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for ViewModel {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "ViewModel",
                    false as usize + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "count",
                    &self.count,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for ViewModel {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "count" => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"count" => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<ViewModel>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = ViewModel;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct ViewModel",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            String,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct ViewModel with 1 element",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(ViewModel { count: __field0 })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<String> = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("count"),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("count")?
                            }
                        };
                        _serde::__private::Ok(ViewModel { count: __field0 })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["count"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "ViewModel",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<ViewModel>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl ::core::clone::Clone for ViewModel {
        #[inline]
        fn clone(&self) -> ViewModel {
            ViewModel {
                count: ::core::clone::Clone::clone(&self.count),
            }
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for ViewModel {
        #[inline]
        fn default() -> ViewModel {
            ViewModel {
                count: ::core::default::Default::default(),
            }
        }
    }
    pub struct Capabilities {
        render: Render<Event>,
    }
    pub enum Effect {
        Render(
            ::crux_core::Request<
                <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
            >,
        ),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Effect {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Effect::Render(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Render",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[serde(rename = "Effect")]
    pub enum EffectFfi {
        Render(<Render<Event> as ::crux_core::capability::Capability<Event>>::Operation),
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for EffectFfi {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                match *self {
                    EffectFfi::Render(ref __field0) => {
                        _serde::Serializer::serialize_newtype_variant(
                            __serializer,
                            "Effect",
                            0u32,
                            "Render",
                            __field0,
                        )
                    }
                }
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for EffectFfi {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "variant identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            _ => {
                                _serde::__private::Err(
                                    _serde::de::Error::invalid_value(
                                        _serde::de::Unexpected::Unsigned(__value),
                                        &"variant index 0 <= i < 1",
                                    ),
                                )
                            }
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "Render" => _serde::__private::Ok(__Field::__field0),
                            _ => {
                                _serde::__private::Err(
                                    _serde::de::Error::unknown_variant(__value, VARIANTS),
                                )
                            }
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"Render" => _serde::__private::Ok(__Field::__field0),
                            _ => {
                                let __value = &_serde::__private::from_utf8_lossy(__value);
                                _serde::__private::Err(
                                    _serde::de::Error::unknown_variant(__value, VARIANTS),
                                )
                            }
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<EffectFfi>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = EffectFfi;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "enum EffectFfi",
                        )
                    }
                    fn visit_enum<__A>(
                        self,
                        __data: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::EnumAccess<'de>,
                    {
                        match _serde::de::EnumAccess::variant(__data)? {
                            (__Field::__field0, __variant) => {
                                _serde::__private::Result::map(
                                    _serde::de::VariantAccess::newtype_variant::<
                                        <Render<
                                            Event,
                                        > as ::crux_core::capability::Capability<Event>>::Operation,
                                    >(__variant),
                                    EffectFfi::Render,
                                )
                            }
                        }
                    }
                }
                #[doc(hidden)]
                const VARIANTS: &'static [&'static str] = &["Render"];
                _serde::Deserializer::deserialize_enum(
                    __deserializer,
                    "Effect",
                    VARIANTS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<EffectFfi>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    impl ::crux_core::Effect for Effect {
        type Ffi = EffectFfi;
        fn serialize(self) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized) {
            match self {
                Effect::Render(request) => request.serialize(EffectFfi::Render),
            }
        }
    }
    impl ::crux_core::WithContext<Event, Effect> for Capabilities {
        fn new_with_context(
            context: ::crux_core::capability::ProtoContext<Effect, Event>,
        ) -> Capabilities {
            Capabilities {
                render: Render::new(context.specialize(Effect::Render)),
            }
        }
    }
    impl Effect {
        pub fn is_render(&self) -> bool {
            if let Effect::Render(_) = self { true } else { false }
        }
        pub fn into_render(
            self,
        ) -> Option<
            crux_core::Request<
                <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
            >,
        > {
            if let Effect::Render(request) = self { Some(request) } else { None }
        }
        pub fn expect_render(
            self,
        ) -> crux_core::Request<
            <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
        > {
            if let Effect::Render(request) = self {
                request
            } else {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("not a {0} effect", "render"),
                    );
                }
            }
        }
    }
    pub struct Counter;
    #[automatically_derived]
    impl ::core::default::Default for Counter {
        #[inline]
        fn default() -> Counter {
            Counter {}
        }
    }
    impl App for Counter {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;
        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
            caps: &Self::Capabilities,
        ) {
            match event {
                Event::Increment => model.count += 1,
                Event::Decrement => model.count -= 1,
                Event::Reset => model.count = 0,
            };
            caps.render.render();
        }
        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                count: ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!("Count is: {0}", model.count),
                    );
                    res
                }),
            }
        }
    }
}
use lazy_static::lazy_static;
pub use crux_core::{bridge::Bridge, Core, Request};
pub use app::*;
#[allow(dead_code)]
mod __unused {
    const _: &[u8] = b"[package]\nname = \"shared\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.66\"\n\n[lib]\ncrate-type = [\"lib\", \"staticlib\", \"cdylib\"]\nname = \"shared\"\n\n[features]\ntypegen = [\"crux_core/typegen\"]\n\n[dependencies]\ncrux_core.workspace = true\nserde = { workspace = true, features = [\"derive\"] }\nlazy_static = \"1.5.0\"\nuniffi = \"0.28.2\"\nwasm-bindgen = \"0.2.95\"\n\n[target.uniffi-bindgen.dependencies]\nuniffi = { version = \"0.28.2\", features = [\"cli\"] }\n\n[build-dependencies]\nuniffi = { version = \"0.28.2\", features = [\"build\"] }\n";
}
#[doc(hidden)]
pub struct UniFfiTag;
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub extern "C" fn ffi_shared_uniffi_contract_version() -> ::std::primitive::u32 {
    26u32
}
/// Export namespace metadata.
///
/// See `uniffi_bindgen::macro_metadata` for how this is used.
const UNIFFI_META_CONST_NAMESPACE_SHARED: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
        ::uniffi::metadata::codes::NAMESPACE,
    )
    .concat_str("shared")
    .concat_str("shared");
#[doc(hidden)]
#[no_mangle]
pub static UNIFFI_META_NAMESPACE_SHARED: [::std::primitive::u8; UNIFFI_META_CONST_NAMESPACE_SHARED
    .size] = UNIFFI_META_CONST_NAMESPACE_SHARED.into_array();
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub extern "C" fn ffi_shared_rustbuffer_alloc(
    size: ::std::primitive::u64,
    call_status: &mut ::uniffi::RustCallStatus,
) -> ::uniffi::RustBuffer {
    ::uniffi::ffi::uniffi_rustbuffer_alloc(size, call_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rustbuffer_from_bytes(
    bytes: ::uniffi::ForeignBytes,
    call_status: &mut ::uniffi::RustCallStatus,
) -> ::uniffi::RustBuffer {
    ::uniffi::ffi::uniffi_rustbuffer_from_bytes(bytes, call_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rustbuffer_free(
    buf: ::uniffi::RustBuffer,
    call_status: &mut ::uniffi::RustCallStatus,
) {
    ::uniffi::ffi::uniffi_rustbuffer_free(buf, call_status);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rustbuffer_reserve(
    buf: ::uniffi::RustBuffer,
    additional: ::std::primitive::u64,
    call_status: &mut ::uniffi::RustCallStatus,
) -> ::uniffi::RustBuffer {
    ::uniffi::ffi::uniffi_rustbuffer_reserve(buf, additional, call_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_u8(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<u8, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_u8(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<u8, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_u8(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> u8 {
    ::uniffi::ffi::rust_future_complete::<u8, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_u8(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<u8, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_i8(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<i8, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_i8(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<i8, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_i8(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> i8 {
    ::uniffi::ffi::rust_future_complete::<i8, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_i8(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<i8, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_u16(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<u16, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_u16(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<u16, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_u16(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> u16 {
    ::uniffi::ffi::rust_future_complete::<u16, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_u16(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<u16, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_i16(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<i16, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_i16(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<i16, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_i16(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> i16 {
    ::uniffi::ffi::rust_future_complete::<i16, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_i16(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<i16, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_u32(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<u32, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_u32(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<u32, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_u32(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> u32 {
    ::uniffi::ffi::rust_future_complete::<u32, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_u32(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<u32, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_i32(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<i32, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_i32(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<i32, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_i32(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> i32 {
    ::uniffi::ffi::rust_future_complete::<i32, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_i32(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<i32, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_u64(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<u64, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_u64(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<u64, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_u64(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> u64 {
    ::uniffi::ffi::rust_future_complete::<u64, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_u64(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<u64, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_i64(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<i64, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_i64(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<i64, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_i64(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> i64 {
    ::uniffi::ffi::rust_future_complete::<i64, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_i64(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<i64, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_f32(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<f32, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_f32(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<f32, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_f32(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> f32 {
    ::uniffi::ffi::rust_future_complete::<f32, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_f32(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<f32, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_f64(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<f64, crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_f64(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<f64, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_f64(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> f64 {
    ::uniffi::ffi::rust_future_complete::<f64, crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_f64(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<f64, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_pointer(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<
        *const ::std::ffi::c_void,
        crate::UniFfiTag,
    >(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_pointer(
    handle: ::uniffi::Handle,
) {
    ::uniffi::ffi::rust_future_cancel::<
        *const ::std::ffi::c_void,
        crate::UniFfiTag,
    >(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_pointer(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> *const ::std::ffi::c_void {
    ::uniffi::ffi::rust_future_complete::<
        *const ::std::ffi::c_void,
        crate::UniFfiTag,
    >(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_pointer(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<
        *const ::std::ffi::c_void,
        crate::UniFfiTag,
    >(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_rust_buffer(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<
        ::uniffi::RustBuffer,
        crate::UniFfiTag,
    >(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_rust_buffer(
    handle: ::uniffi::Handle,
) {
    ::uniffi::ffi::rust_future_cancel::<::uniffi::RustBuffer, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_rust_buffer(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> ::uniffi::RustBuffer {
    ::uniffi::ffi::rust_future_complete::<
        ::uniffi::RustBuffer,
        crate::UniFfiTag,
    >(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_rust_buffer(
    handle: ::uniffi::Handle,
) {
    ::uniffi::ffi::rust_future_free::<::uniffi::RustBuffer, crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_poll_void(
    handle: ::uniffi::Handle,
    callback: ::uniffi::RustFutureContinuationCallback,
    data: u64,
) {
    ::uniffi::ffi::rust_future_poll::<(), crate::UniFfiTag>(handle, callback, data);
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_cancel_void(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_cancel::<(), crate::UniFfiTag>(handle)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_complete_void(
    handle: ::uniffi::Handle,
    out_status: &mut ::uniffi::RustCallStatus,
) -> () {
    ::uniffi::ffi::rust_future_complete::<(), crate::UniFfiTag>(handle, out_status)
}
#[allow(clippy::missing_safety_doc, missing_docs)]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn ffi_shared_rust_future_free_void(handle: ::uniffi::Handle) {
    ::uniffi::ffi::rust_future_free::<(), crate::UniFfiTag>(handle)
}
#[allow(missing_docs)]
#[doc(hidden)]
pub const fn uniffi_reexport_hack() {}
#[allow(unused)]
#[doc(hidden)]
pub trait UniffiCustomTypeConverter {
    type Builtin;
    fn into_custom(val: Self::Builtin) -> ::uniffi::Result<Self>
    where
        Self: ::std::marker::Sized;
    fn from_custom(obj: Self) -> Self::Builtin;
}
/// Export info about the UDL while used to create us
/// See `uniffi_bindgen::macro_metadata` for how this is used.
const UNIFFI_META_CONST_UDL_SHARED: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
        ::uniffi::metadata::codes::UDL_FILE,
    )
    .concat_str("shared")
    .concat_str("shared")
    .concat_str("shared");
#[doc(hidden)]
#[no_mangle]
pub static UNIFFI_META_UDL_SHARED: [u8; UNIFFI_META_CONST_UDL_SHARED.size] = UNIFFI_META_CONST_UDL_SHARED
    .into_array();
#[doc(hidden)]
#[no_mangle]
extern "C" fn uniffi_shared_fn_func_handle_response(
    id: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    res: <<::std::vec::Vec<
        u8,
    > as ::uniffi::LiftRef<
        crate::UniFfiTag,
    >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    call_status: &mut ::uniffi::RustCallStatus,
) -> <::std::vec::Vec<u8> as ::uniffi::LowerReturn<crate::UniFfiTag>>::ReturnType {
    {
        let lvl = ::log::Level::Debug;
        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
            ::log::__private_api::log(
                format_args!("handle_response"),
                lvl,
                &("shared", "shared", ::log::__private_api::loc()),
                (),
            );
        }
    };
    let uniffi_lift_args = move || ::std::result::Result::Ok((
        match <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(id) {
            ::std::result::Result::Ok(v) => v,
            ::std::result::Result::Err(e) => return ::std::result::Result::Err(("id", e)),
        },
        match <<::std::vec::Vec<
            u8,
        > as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(res) {
            ::std::result::Result::Ok(v) => v,
            ::std::result::Result::Err(e) => {
                return ::std::result::Result::Err(("res", e));
            }
        },
    ));
    ::uniffi::rust_call(
        call_status,
        || {
            match uniffi_lift_args() {
                ::std::result::Result::Ok(uniffi_args) => {
                    let uniffi_result = handle_response(
                        uniffi_args.0,
                        <<::std::vec::Vec<
                            u8,
                        > as ::uniffi::LiftRef<
                            crate::UniFfiTag,
                        >>::LiftType as ::std::borrow::Borrow<
                            ::std::vec::Vec<u8>,
                        >>::borrow(&uniffi_args.1),
                    );
                    <::std::vec::Vec<
                        u8,
                    > as ::uniffi::LowerReturn<
                        crate::UniFfiTag,
                    >>::lower_return(uniffi_result)
                }
                ::std::result::Result::Err((arg_name, error)) => {
                    <::std::vec::Vec<
                        u8,
                    > as ::uniffi::LowerReturn<
                        crate::UniFfiTag,
                    >>::handle_failed_lift(::uniffi::LiftArgsError {
                        arg_name,
                        error,
                    })
                }
            }
        },
    )
}
#[doc(hidden)]
#[no_mangle]
extern "C" fn uniffi_shared_fn_func_process_event(
    msg: <<::std::vec::Vec<
        u8,
    > as ::uniffi::LiftRef<
        crate::UniFfiTag,
    >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    call_status: &mut ::uniffi::RustCallStatus,
) -> <::std::vec::Vec<u8> as ::uniffi::LowerReturn<crate::UniFfiTag>>::ReturnType {
    {
        let lvl = ::log::Level::Debug;
        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
            ::log::__private_api::log(
                format_args!("process_event"),
                lvl,
                &("shared", "shared", ::log::__private_api::loc()),
                (),
            );
        }
    };
    let uniffi_lift_args = move || ::std::result::Result::Ok((
        match <<::std::vec::Vec<
            u8,
        > as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(msg) {
            ::std::result::Result::Ok(v) => v,
            ::std::result::Result::Err(e) => {
                return ::std::result::Result::Err(("msg", e));
            }
        },
    ));
    ::uniffi::rust_call(
        call_status,
        || {
            match uniffi_lift_args() {
                ::std::result::Result::Ok(uniffi_args) => {
                    let uniffi_result = process_event(
                        <<::std::vec::Vec<
                            u8,
                        > as ::uniffi::LiftRef<
                            crate::UniFfiTag,
                        >>::LiftType as ::std::borrow::Borrow<
                            ::std::vec::Vec<u8>,
                        >>::borrow(&uniffi_args.0),
                    );
                    <::std::vec::Vec<
                        u8,
                    > as ::uniffi::LowerReturn<
                        crate::UniFfiTag,
                    >>::lower_return(uniffi_result)
                }
                ::std::result::Result::Err((arg_name, error)) => {
                    <::std::vec::Vec<
                        u8,
                    > as ::uniffi::LowerReturn<
                        crate::UniFfiTag,
                    >>::handle_failed_lift(::uniffi::LiftArgsError {
                        arg_name,
                        error,
                    })
                }
            }
        },
    )
}
#[doc(hidden)]
#[no_mangle]
extern "C" fn uniffi_shared_fn_func_view(
    call_status: &mut ::uniffi::RustCallStatus,
) -> <::std::vec::Vec<u8> as ::uniffi::LowerReturn<crate::UniFfiTag>>::ReturnType {
    {
        let lvl = ::log::Level::Debug;
        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
            ::log::__private_api::log(
                format_args!("view"),
                lvl,
                &("shared", "shared", ::log::__private_api::loc()),
                (),
            );
        }
    };
    let uniffi_lift_args = move || ::std::result::Result::Ok(());
    ::uniffi::rust_call(
        call_status,
        || {
            match uniffi_lift_args() {
                ::std::result::Result::Ok(uniffi_args) => {
                    let uniffi_result = view();
                    <::std::vec::Vec<
                        u8,
                    > as ::uniffi::LowerReturn<
                        crate::UniFfiTag,
                    >>::lower_return(uniffi_result)
                }
                ::std::result::Result::Err((arg_name, error)) => {
                    <::std::vec::Vec<
                        u8,
                    > as ::uniffi::LowerReturn<
                        crate::UniFfiTag,
                    >>::handle_failed_lift(::uniffi::LiftArgsError {
                        arg_name,
                        error,
                    })
                }
            }
        },
    )
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn uniffi_shared_checksum_func_handle_response() -> u16 {
    38599
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn uniffi_shared_checksum_func_process_event() -> u16 {
    35444
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn uniffi_shared_checksum_func_view() -> u16 {
    57786
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct CORE {
    __private_field: (),
}
#[doc(hidden)]
#[allow(non_upper_case_globals)]
static CORE: CORE = CORE { __private_field: () };
impl ::lazy_static::__Deref for CORE {
    type Target = Bridge<Effect, Counter>;
    fn deref(&self) -> &Bridge<Effect, Counter> {
        #[inline(always)]
        fn __static_ref_initialize() -> Bridge<Effect, Counter> {
            Bridge::new(Core::new())
        }
        #[inline(always)]
        fn __stability() -> &'static Bridge<Effect, Counter> {
            static LAZY: ::lazy_static::lazy::Lazy<Bridge<Effect, Counter>> = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for CORE {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE.process_event(data)
}
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    CORE.handle_response(id, data)
}
pub fn view() -> Vec<u8> {
    CORE.view()
}
