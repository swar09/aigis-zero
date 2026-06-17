use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tonic::Status;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

#[derive(Debug, Clone)]
pub struct JsonCodec<T, U>(PhantomData<(T, U)>);

impl<T, U> Default for JsonCodec<T, U> {
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

    fn encoder(&mut self) -> Self::Encoder {
        JsonEncoder(PhantomData)
    }

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
