import json
from typing import Any, Type, TypeVar


T = TypeVar("T")


def encode_json(value: Any) -> bytes:
    """
    Serialize a Python object into JSON bytes.

    Equivalent to Rust:

        serde_json::to_vec(&item)
    """

    if is_dataclass(value):
        value = asdict(value)

    return json.dumps(
        value,
        separators=(",", ":"),
    ).encode("utf-8")


def decode_json(
    data: bytes,
    model: Type[T],
) -> T:
    """
    Deserialize JSON bytes into a Python object.

    Equivalent to Rust:

        serde_json::from_slice(&buf)
    """

    if not data:
        raise ValueError(
            "Cannot decode empty data"
        )

    decoded = json.loads(
        data.decode("utf-8")
    )

    return model(**decoded)
