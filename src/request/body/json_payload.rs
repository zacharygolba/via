use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use std::fmt::{self, Formatter};
use std::marker::PhantomData;

pub struct JsonPayload<T> {
    pub data: T,
}

struct JsonPayloadVisitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T> Deserialize<'de> for JsonPayload<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "JsonPayload",
            &["data"],
            JsonPayloadVisitor {
                marker: PhantomData,
            },
        )
    }
}

impl<'de, T> Visitor<'de> for JsonPayloadVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = JsonPayload<T>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a JSON object with a `data` field")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut value = None;

        while let Some(key) = map.next_key::<String>()? {
            if key == "data" {
                // Similar to the JSON API in web browsers. Duplicate fields
                // result in subsequent keys overriding the value previously
                // defined in the payload.
                value = Some(map.next_value()?);
            } else {
                // Skip unknown fields.
                map.next_value::<de::IgnoredAny>()?;
            }
        }

        match value {
            Some(data) => Ok(JsonPayload { data }),
            None => Err(de::Error::missing_field("data")),
        }
    }
}
