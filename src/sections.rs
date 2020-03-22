use crate::builtins::blob::{Blob, RawBlob};
use crate::builtins::lazy::Lazy;
use crate::indices::{FuncId, GlobalId, LocalId, MemId, TableId, TypeId};
use crate::instructions::Expression;
use crate::types::{FuncType, GlobalType, MemType, TableType, ValueType};
use crate::{
    wasmbin_discriminants, DecodeError, Wasmbin, WasmbinCountable, WasmbinDecode,
    WasmbinDecodeWithDiscriminant, WasmbinEncode,
};
use arbitrary::Arbitrary;
use custom_debug::CustomDebug;

#[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct ModuleNameSubSection {
    pub name: String,
}

#[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct NameAssoc<I, V> {
    pub index: I,
    pub value: V,
}

impl<I, V> WasmbinCountable for NameAssoc<I, V> {}

#[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct NameMap<I, V> {
    pub items: Vec<NameAssoc<I, V>>,
}

#[wasmbin_discriminants]
#[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
#[repr(u8)]
pub enum NameSubSection {
    Module(Blob<String>) = 0,
    Func(Blob<NameMap<FuncId, String>>) = 1,
    Local(Blob<NameMap<FuncId, NameMap<LocalId, String>>>) = 2,
}

impl WasmbinEncode for [NameSubSection] {
    fn encode(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        for sub in self {
            sub.encode(w)?;
        }
        Ok(())
    }
}

impl WasmbinDecode for Vec<NameSubSection> {
    fn decode(r: &mut impl std::io::Read) -> Result<Self, DecodeError> {
        let mut sub = Vec::new();
        loop {
            match u8::decode(r) {
                Err(DecodeError::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(sub);
                }
                Err(err) => {
                    return Err(err);
                }
                Ok(discriminant) => {
                    sub.push(NameSubSection::decode_with_discriminant(discriminant, r)?);
                }
            }
        }
    }
}

#[derive(Wasmbin, CustomDebug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct RawCustomSection {
    name: String,

    #[debug(with = "custom_debug::hexbuf_str")]
    data: Vec<u8>,
}

macro_rules! define_custom_sections {
    ($($name:ident($ty:ty) = $disc:literal,)*) => {
        #[derive(Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
        pub enum CustomSection {
            $($name(Lazy<$ty>),)*
            Other(RawCustomSection),
        }

        impl WasmbinEncode for CustomSection {
            fn encode(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
                match self {
                    $(CustomSection::$name(data) => {
                        $disc.encode(w)?;
                        data.encode(w)
                    })*
                    CustomSection::Other(raw) => raw.encode(w)
                }
            }
        }

        impl WasmbinDecode for CustomSection {
            fn decode(r: &mut impl std::io::Read) -> Result<Self, DecodeError> {
                let raw = RawCustomSection::decode(r)?;
                Ok(match raw.name.as_str() {
                    $($disc => CustomSection::$name(Lazy::from_raw(raw.data)),)*
                    _ => CustomSection::Other(raw)
                })
            }
        }
    };
}

define_custom_sections! {
    Name(Vec<NameSubSection>) = "name",
}

#[wasmbin_discriminants]
#[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
#[repr(u8)]
pub enum ImportDesc {
    Func(TypeId) = 0x00,
    Table(TableType) = 0x01,
    Mem(MemType) = 0x02,
    Global(GlobalType) = 0x03,
}

#[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct ImportPath {
    pub module: String,
    pub name: String,
}

#[derive(Wasmbin, WasmbinCountable, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct Import {
    pub path: ImportPath,
    pub desc: ImportDesc,
}

#[derive(Wasmbin, WasmbinCountable, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct Global {
    pub ty: GlobalType,
    pub init: Expression,
}

#[wasmbin_discriminants]
#[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
#[repr(u8)]
pub enum ExportDesc {
    Func(FuncId) = 0x00,
    Table(TableId) = 0x01,
    Mem(MemId) = 0x02,
    Global(GlobalId) = 0x03,
}

#[derive(Wasmbin, WasmbinCountable, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct Export {
    pub name: String,
    pub desc: ExportDesc,
}

#[derive(Wasmbin, WasmbinCountable, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct Element {
    pub table: TableId,
    pub offset: Expression,
    pub init: Vec<FuncId>,
}

#[derive(Wasmbin, WasmbinCountable, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct Locals {
    pub repeat: u32,
    pub ty: ValueType,
}

#[derive(Wasmbin, WasmbinCountable, Debug, Default, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct Func {
    pub locals: Vec<Locals>,
    pub body: Expression,
}

#[derive(Wasmbin, WasmbinCountable, CustomDebug, Arbitrary, PartialEq, Eq, Hash, Clone)]
pub struct Data {
    pub memory: MemId,
    pub offset: Expression,
    #[debug(with = "custom_debug::hexbuf_str")]
    pub init: RawBlob,
}

pub trait Payload: WasmbinEncode + WasmbinDecode + Into<Section> {
    const KIND: Kind;

    fn try_from_ref(section: &Section) -> Option<&Blob<Self>>;
    fn try_from_mut(section: &mut Section) -> Option<&mut Blob<Self>>;
    fn try_from(section: Section) -> Result<Blob<Self>, Section>;
}

pub trait StdPayload: Payload {}

macro_rules! define_sections {
    ($($name:ident($ty:ty) = $disc:literal,)*) => {
        pub mod payload {
            $(pub type $name = $ty;)*
        }

        #[wasmbin_discriminants]
        #[derive(Wasmbin, Debug, Arbitrary, PartialEq, Eq, Hash, Clone)]
        #[repr(u8)]
        pub enum Section {
            $($name(Blob<payload::$name>) = $disc,)*
        }

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Kind {
            $($name = $disc,)*
        }

        $(
            impl From<Blob<payload::$name>> for Section {
                fn from(value: Blob<payload::$name>) -> Self {
                    Section::$name(value)
                }
            }

            impl From<payload::$name> for Section {
                fn from(value: payload::$name) -> Self {
                    Section::$name(Blob::from(value))
                }
            }

            impl Payload for payload::$name {
                const KIND: Kind = Kind::$name;

                fn try_from_ref(section: &Section) -> Option<&Blob<Self>> {
                    match section {
                        Section::$name(res) => Some(res),
                        _ => None,
                    }
                }

                fn try_from_mut(section: &mut Section) -> Option<&mut Blob<Self>> {
                    match section {
                        Section::$name(res) => Some(res),
                        _ => None,
                    }
                }

                fn try_from(section: Section) -> Result<Blob<Self>, Section> {
                    match section {
                        Section::$name(res) => Ok(res),
                        _ => Err(section),
                    }
                }
            }
        )*

        impl Section {
            pub fn kind(&self) -> Kind {
                match self {
                    $(Section::$name(_) => Kind::$name,)*
                }
            }

            pub fn try_as<T: Payload>(&self) -> Option<&Blob<T>> {
                T::try_from_ref(self)
            }

            pub fn try_as_mut<T: Payload>(&mut self) -> Option<&mut Blob<T>> {
                T::try_from_mut(self)
            }
        }

        define_sections!(@std $($name)*);
    };

    (@std $ignore_custom:ident $($name:ident)*) => {
        $(impl StdPayload for payload::$name {})*
    };
}

define_sections! {
    Custom(super::CustomSection) = 0,
    Type(Vec<super::FuncType>) = 1,
    Import(Vec<super::Import>) = 2,
    Function(Vec<super::TypeId>) = 3,
    Table(Vec<super::TableType>) = 4,
    Memory(Vec<super::MemType>) = 5,
    Global(Vec<super::Global>) = 6,
    Export(Vec<super::Export>) = 7,
    Start(super::FuncId) = 8,
    Element(Vec<super::Element>) = 9,
    Code(Vec<super::Blob<super::Func>>) = 10,
    Data(Vec<super::Data>) = 11,
}

impl WasmbinEncode for [Section] {
    fn encode(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        for section in self {
            section.encode(w)?;
        }
        Ok(())
    }
}

impl WasmbinDecode for Vec<Section> {
    fn decode(r: &mut impl std::io::Read) -> Result<Self, DecodeError> {
        let mut sections = Vec::new();
        loop {
            match u8::decode(r) {
                Err(DecodeError::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(sections);
                }
                Err(err) => {
                    return Err(err);
                }
                Ok(discriminant) => {
                    sections.push(Section::decode_with_discriminant(discriminant, r)?);
                }
            }
        }
    }
}
