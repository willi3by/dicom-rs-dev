//! DICOM JSON deserialization module

use std::{marker::PhantomData, str::FromStr};

use crate::DicomJson;
use dicom_core::{
    value::{InMemFragment, Value, C},
    DataDictionary, DataElement, PrimitiveValue, Tag, VR,
};
use dicom_object::InMemDicomObject;
use serde::de::{Deserialize, DeserializeOwned, Error as _, Visitor};

use self::value::{DicomJsonPerson, NumberOrText};

mod value;

/// Deserialize a piece of DICOM data from a string of JSON.
pub fn from_str<'a, T>(string: &'a str) -> Result<T, serde_json::Error>
where
    DicomJson<T>: Deserialize<'a>,
{
    serde_json::from_str::<DicomJson<T>>(string).map(DicomJson::into_inner)
}

/// Deserialize a piece of DICOM data from a byte slice.
pub fn from_slice<'a, T>(slice: &'a [u8]) -> Result<T, serde_json::Error>
where
    DicomJson<T>: Deserialize<'a>,
{
    serde_json::from_slice::<DicomJson<T>>(slice).map(DicomJson::into_inner)
}

/// Deserialize a piece of DICOM data from a standard byte reader.
pub fn from_reader<R, T>(reader: R) -> Result<T, serde_json::Error>
where
    R: std::io::Read,
    DicomJson<T>: DeserializeOwned,
{
    serde_json::from_reader::<_, DicomJson<T>>(reader).map(DicomJson::into_inner)
}

/// Deserialize a piece of DICOM data from a serde JSON value.
pub fn from_value<T>(value: serde_json::Value) -> Result<T, serde_json::Error>
where
    DicomJson<T>: DeserializeOwned,
{
    serde_json::from_value::<DicomJson<T>>(value).map(DicomJson::into_inner)
}

#[derive(Debug)]
struct InMemDicomObjectVisitor<D>(PhantomData<D>);

impl<D> Default for InMemDicomObjectVisitor<D> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<'de, D> Visitor<'de> for InMemDicomObjectVisitor<D>
where
    D: Default + DataDictionary + Clone,
{
    type Value = InMemDicomObject<D>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a DICOM data set map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut obj = InMemDicomObject::<D>::new_empty_with_dict(D::default());
        while let Some(e) = map.next_entry::<DicomJson<Tag>, JsonDataElement<D>>()? {
            let (DicomJson(tag), JsonDataElement { vr, value }) = e;
            obj.put(DataElement::new(tag, vr, value));
        }
        Ok(obj)
    }
}

impl<'de, I> Deserialize<'de> for DicomJson<InMemDicomObject<I>>
where
    I: Default + Clone + DataDictionary,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_map(InMemDicomObjectVisitor::default())
            .map(DicomJson::from)
            .map_err(From::from)
    }
}

#[derive(Debug)]
struct JsonDataElement<D> {
    vr: VR,
    value: Value<InMemDicomObject<D>, InMemFragment>,
}

#[derive(Debug)]
struct DataElementVisitor<D>(PhantomData<D>);

impl<D> Default for DataElementVisitor<D> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<'de, D> Visitor<'de> for DataElementVisitor<D>
where
    D: Default + Clone + DataDictionary,
{
    type Value = JsonDataElement<D>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a data element object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut values: Option<_> = None;
        let mut inline_binary = None;

        // first field should be "vr"
        let key: String = map
            .next_key()?
            .ok_or_else(|| A::Error::custom("\"vr\" is not set"))?;

        if key != "vr" {
            eprintln!("First field is \"{}\" instead of \"vr\"", key);
            return Err(A::Error::custom("expected \"vr\" to be the first field"));
        }

        // read VR
        let val: String = map.next_value()?;
        let vr = VR::from_str(&val).unwrap_or(
            // unrecognized VR
            VR::UN,
        );

        while let Some(key) = map.next_key::<String>()? {
            match &*key {
                "vr" => {
                    return Err(A::Error::custom("\"vr\" should only be set once"));
                }
                "Value" => {
                    if inline_binary.is_some() {
                        return Err(A::Error::custom(
                            "\"Value\" conflicts with \"InlineBinary\"",
                        ));
                    }

                    // deserialize value in different ways
                    // depending on VR
                    match vr {
                        // sequence
                        VR::SQ => {
                            let items: Vec<DicomJson<InMemDicomObject<D>>> = map.next_value()?;
                            let items: Vec<_> =
                                items.into_iter().map(DicomJson::into_inner).collect();
                            values = Some(Value::Sequence(items.into()));
                        }
                        // always text
                        VR::AE
                        | VR::AS
                        | VR::CS
                        | VR::DA
                        | VR::DT
                        | VR::LO
                        | VR::LT
                        | VR::SH
                        | VR::ST
                        | VR::UT
                        | VR::UR
                        | VR::TM
                        | VR::UC
                        | VR::UI => {
                            let items: Vec<String> = map.next_value()?;
                            values = Some(PrimitiveValue::Strs(items.into()).into());
                        }

                        // should always be signed 16-bit integers
                        VR::SS => {
                            let items: Vec<i16> = map.next_value()?;
                            values = Some(PrimitiveValue::I16(items.into()).into());
                        }
                        // should always be unsigned 16-bit integers
                        VR::US | VR::OW => {
                            let items: Vec<u16> = map.next_value()?;
                            values = Some(PrimitiveValue::U16(items.into()).into());
                        }
                        // should always be signed 32-bit integers
                        VR::SL => {
                            let items: Vec<i32> = map.next_value()?;
                            values = Some(PrimitiveValue::I32(items.into()).into());
                        }
                        VR::OB => {
                            let items: Vec<u8> = map.next_value()?;
                            values = Some(PrimitiveValue::U8(items.into()).into());
                        }
                        // sometimes numbers, sometimes text,
                        // should parse on the spot
                        VR::FL | VR::OF => {
                            let items: Vec<NumberOrText<f32>> = map.next_value()?;
                            let items: C<f32> = items
                                .into_iter()
                                .map(|v| v.to_num())
                                .collect::<Result<C<f32>, _>>()
                                .map_err(A::Error::custom)?;
                            values = Some(PrimitiveValue::F32(items).into());
                        }
                        VR::FD | VR::OD => {
                            let items: Vec<NumberOrText<f64>> = map.next_value()?;
                            let items: C<f64> = items
                                .into_iter()
                                .map(|v| v.to_num())
                                .collect::<Result<C<f64>, _>>()
                                .map_err(A::Error::custom)?;
                            values = Some(PrimitiveValue::F64(items).into());
                        }
                        VR::SV => {
                            let items: Vec<NumberOrText<i64>> = map.next_value()?;
                            let items: C<i64> = items
                                .into_iter()
                                .map(|v| v.to_num())
                                .collect::<Result<C<i64>, _>>()
                                .map_err(A::Error::custom)?;
                            values = Some(PrimitiveValue::I64(items).into());
                        }
                        VR::UL | VR::OL => {
                            let items: Vec<NumberOrText<u32>> = map.next_value()?;
                            let items: C<u32> = items
                                .into_iter()
                                .map(|v| v.to_num())
                                .collect::<Result<C<u32>, _>>()
                                .map_err(A::Error::custom)?;
                            values = Some(PrimitiveValue::U32(items).into());
                        }
                        VR::UV | VR::OV => {
                            let items: Vec<NumberOrText<u64>> = map.next_value()?;
                            let items: C<u64> = items
                                .into_iter()
                                .map(|v| v.to_num())
                                .collect::<Result<C<u64>, _>>()
                                .map_err(A::Error::custom)?;
                            values = Some(PrimitiveValue::U64(items).into());
                        }
                        // sometimes numbers, sometimes text,
                        // but retain string form
                        VR::DS => {
                            let items: Vec<NumberOrText<f64>> = map.next_value()?;
                            let items: C<String> =
                                items.into_iter().map(|v| v.to_string()).collect();
                            values = Some(PrimitiveValue::Strs(items).into());
                        }
                        VR::IS => {
                            let items: Vec<NumberOrText<f64>> = map.next_value()?;
                            let items: C<String> =
                                items.into_iter().map(|v| v.to_string()).collect();
                            values = Some(PrimitiveValue::Strs(items).into());
                        }
                        // person names
                        VR::PN => {
                            let items: Vec<DicomJsonPerson> = map.next_value()?;
                            let items: C<String> =
                                items.into_iter().map(|v| v.to_string()).collect();
                            values = Some(PrimitiveValue::Strs(items).into());
                        }
                        // tags
                        VR::AT => {
                            let items: Vec<DicomJson<Tag>> = map.next_value()?;
                            let items: C<Tag> =
                                items.into_iter().map(DicomJson::into_inner).collect();
                            values = Some(PrimitiveValue::Tags(items).into());
                        }
                        // unknown
                        VR::UN => return Err(A::Error::custom("can't parse JSON Value in UN")),
                    }
                }
                "InlineBinary" => {
                    if values.is_some() {
                        return Err(A::Error::custom(
                            "\"InlineBinary\" conflicts with \"Value\"",
                        ));
                    }
                    // read value as string
                    let val: String = map.next_value()?;
                    inline_binary = Some(val);
                }
                _ => {
                    return Err(A::Error::custom("Unrecognized data element field"));
                }
            }
        }

        let value = match (values, inline_binary) {
            (None, None) => PrimitiveValue::Empty.into(),
            (None, Some(inline_binary)) => {
                // decode from Base64
                use base64::Engine;
                let data = base64::engine::general_purpose::STANDARD
                    .decode(inline_binary)
                    .map_err(|_| A::Error::custom("inline binary data is not valid base64"))?;
                PrimitiveValue::from(data).into()
            }
            (Some(values), None) => values,
            (Some(_), Some(_)) => unreachable!(),
        };

        Ok(JsonDataElement { vr, value })
    }
}

impl<'de, I> Deserialize<'de> for JsonDataElement<I>
where
    I: Default + Clone + DataDictionary,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "DataElement",
            &["vr", "Value", "InlineData", "BulkDataURI"],
            DataElementVisitor(PhantomData),
        )
    }
}

#[derive(Debug)]
struct TagVisitor;

impl Visitor<'_> for TagVisitor {
    type Value = Tag;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a DICOM tag string in the form \"GGGGEEEE\"")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse().map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for DicomJson<Tag> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(TagVisitor).map(DicomJson)
    }
}

#[cfg(test)]
mod tests {
    use super::from_str;
    use dicom_core::{DataElement, Tag, VR};
    use dicom_object::InMemDicomObject;

    #[test]
    fn can_parse_tags() {
        let serialized = "\"00080010\"";
        let tag: Tag = from_str(serialized).unwrap();
        assert_eq!(tag, Tag(0x0008, 0x0010));

        let serialized = "\"00200013\"";
        let tag: Tag = from_str(serialized).unwrap();
        assert_eq!(tag, Tag(0x0020, 0x0013));
    }

    #[test]
    fn can_parse_simple_data_sets() {
        let serialized = serde_json::json!({
            "00080005": {
                "vr": "CS",
                "Value": [ "ISO_IR 192" ]
            },
            "00080020": {
                "vr": "DA",
                "Value": [ "20130409" ]
            },
            "00080061": {
                "vr": "CS",
                "Value": [
                    "CT",
                    "PET"
                ]
            },
            "00080090": {
                "vr": "PN",
                "Value": [
                  {
                    "Alphabetic": "^Bob^^Dr."
                  }
                ]
            },
            "00091002": {
                "vr": "UN",
                "InlineBinary": "z0x9c8v7"
            },
            "00101010": {
                "vr": "AS",
                "Value": [ "30Y" ]
            }
        });

        let obj: InMemDicomObject = super::from_value(serialized).unwrap();

        let tag = Tag(0x0008, 0x0005);
        assert_eq!(
            obj.get(tag),
            Some(&DataElement::new(tag, VR::CS, "ISO_IR 192")),
        )
    }
}
