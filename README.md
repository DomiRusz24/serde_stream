# Send objects through streams

This library adds SerdeRead and SerdeWrite, traits that allow you to quickly send serializable enums and structs over streams, both for std and tokio variants.

It uses MessagePack (https://crates.io/crates/rmp-serde) for both serialization and deserialization.
Before sending the serialized data, it sends the amount of bytes that will be sent.
