use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tonic::Status;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

#[derive(Debug, Clone)]
pub struct JsonCodec<T, U>(PhantomData<(T, U)>);

impl<T, U> Default for JsonCodec<T, U> {
    /// Creates a default JSON codec.
    ///
    /// # Examples
    ///
    /// ```
    /// let codec: JsonCodec<String, String> = Default::default();
    /// ```
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub struct JsonEncoder<T>(PhantomData<T>);
pub struct JsonDecoder<U>(PhantomData<U>);

impl<T, U> Codec for JsonCodec<T, U>
where
    T: Serialize + Send + 'static,
    U: for<'de> Deserialize<'de> + Send + 'static,
{
    type Encode = T;
    type Decode = U;
    type Encoder = JsonEncoder<T>;
    type Decoder = JsonDecoder<U>;

    /// Creates a JSON encoder for this codec.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut codec = JsonCodec::default();
    /// let _encoder = codec.encoder();
    /// ```
    fn encoder(&mut self) -> Self::Encoder {
        JsonEncoder(PhantomData)
    }

    /// Creates a new JSON decoder for this codec.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut codec = JsonCodec::default();
    /// let decoder = codec.decoder();
    /// ```
    fn decoder(&mut self) -> Self::Decoder {
        JsonDecoder(PhantomData)
    }
}

impl<T> Encoder for JsonEncoder<T>
where
    T: Serialize,
{
    type Item = T;
    type Error = Status;

    /// Encodes an item to JSON and appends it to the destination buffer.
    ///
    /// # Errors
    ///
    /// Returns an internal `Status` error if the item cannot be serialized to JSON.
    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        let bytes = serde_json::to_vec(&item).map_err(|e| Status::internal(e.to_string()))?;
        dst.put_slice(&bytes);
        Ok(())
    }
}

impl<U> Decoder for JsonDecoder<U>
where
    U: for<'de> Deserialize<'de>,
{
    type Item = U;
    type Error = Status;

    /// Decodes a JSON-encoded message from the buffer.
    ///
    /// Returns `None` if the buffer is empty. Otherwise, deserializes all remaining
    /// bytes as a JSON message of type `U`.
    ///
    /// # Errors
    ///
    /// Returns a `Status::internal` error if JSON deserialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut decoder = JsonDecoder::<i32>(PhantomData);
    /// let mut empty_buffer = DecodeBuf::new(&[]);
    /// assert_eq!(decoder.decode(&mut empty_buffer), Ok(None));
    /// ```
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }

        let len = src.remaining();
        let mut buf = vec![0u8; len];
        src.copy_to_slice(&mut buf);
        let item: U = serde_json::from_slice(&buf).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Some(item))
    }
}
