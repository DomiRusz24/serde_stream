//!
//! This library adds SerdeRead and SerdeWrite, both for std and tokio variants.
//!
//! [std]: crate::std_stream
//! [tokio]: crate::tokio_stream
//!
//! It uses MessagePack (https://crates.io/crates/rmp-serde) for both serialization and deserialization.
//!
//! Before sending the serialized data, it sends the amount of bytes that will be sent.
//!

use thiserror::Error;
use std::io;
use rmp_serde::{encode, decode};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Buffer sent is too large ({size} > {max_size})")]
    BufferTooLarge {
        size: usize,
        max_size: usize,
    },
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Error while deserializing")]
    Deserialize(#[from] decode::Error),
    #[error("Error while serializing")]
    Serialize(#[from] encode::Error),
}

#[cfg(feature = "std")]
pub mod std_stream {
    use serde::de::DeserializeOwned;
    use std::io::{Read, Write};
    use serde::Serialize;
    use crate::Error;

    /// Fetch and deserialize given object from stream.
    ///
    /// # Example
    ///
    /// ```edition2021
    /// use serde::Deserialize;
    /// use std::io::prelude::*;
    /// use std::net::TcpStream;
    /// use serde_stream::std_stream::SerdeRead;
    ///
    /// #[derive(Deserialize)]
    /// struct Foo {
    ///     value: u32
    /// }
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let mut stream = TcpStream::connect("127.0.0.1:34254")?;
    ///
    ///     let obj = stream.read_obj::<Foo>(1024)?;
    ///
    ///     println!("Value: {}", obj.value);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub trait SerdeRead {
        fn read_obj<T: DeserializeOwned>(&mut self, max_size: usize) -> Result<T, Error>;
    }

    /// Serialize given object, and send it through the stream.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use serde::Serialize;
    /// use std::io::prelude::*;
    /// use std::net::TcpStream;
    /// use serde_stream::std_stream::SerdeWrite;
    ///
    /// #[derive(Serialize)]
    /// struct Foo {
    ///     value: u32
    /// }
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let mut stream = TcpStream::connect("127.0.0.1:34254")?;
    ///
    ///     stream.write_obj(&Foo {
    ///         value: 1222
    ///     })?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub trait SerdeWrite {
        fn write_obj<T: Serialize>(&mut self, obj: &T) -> Result<(), Error>;
    }

    impl<R: Read> SerdeRead for R {
        fn read_obj<T: DeserializeOwned>(&mut self, max_size: usize) -> Result<T, Error> {
            let mut size_buffer = [0; 8];
            self.read_exact(&mut size_buffer)?;
            let size = usize::from_be_bytes(size_buffer);

            if size > max_size {
                return Err(Error::BufferTooLarge { size, max_size })
            }

            let mut value_buffer = vec![0; size];
            self.read_exact(&mut value_buffer)?;

            let res = rmp_serde::from_slice(&value_buffer)?;

            Ok(res)
        }
    }

    impl<W: Write> SerdeWrite for W {
        fn write_obj<T: Serialize>(&mut self, obj: &T) -> Result<(), Error> {
            let obj_bytes = rmp_serde::to_vec(obj)?;
            let size = obj_bytes.len();
            let size_bytes = size.to_be_bytes();
            self.write_all(&size_bytes)?;
            self.write_all(&obj_bytes)?;
            Ok(())
        }
    }
}

#[cfg(feature = "tokio")]
pub mod tokio_stream {
    use async_trait::async_trait;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use crate::Error;


    /// Fetch and deserialize given object from stream.
    ///
    /// #Example
    ///
    /// ```no_run
    /// use serde::Deserialize;
    /// use tokio::net::TcpStream;
    /// use tokio::io::AsyncWriteExt;
    /// use std::error::Error;
    /// use serde_stream::tokio_stream::SerdeRead;
    ///
    /// #[derive(Deserialize)]
    /// struct Foo {
    ///     value: u32
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    ///     let obj = stream.read_obj::<Foo>(1024).await?;
    ///
    ///     println!("Value: {}", obj.value);
    ///
    ///     Ok(())
    /// }
    /// ```
    #[async_trait]
    pub trait SerdeRead {
        async fn read_obj<T: DeserializeOwned>(&mut self, max_size: usize) -> Result<T, Error>;
    }

    /// Serialize given object, and send it through the stream.
    ///
    /// #Example
    ///
    /// ```no_run
    /// use serde::Serialize;
    /// use tokio::net::TcpStream;
    /// use tokio::io::AsyncWriteExt;
    /// use std::error::Error;
    /// use serde_stream::tokio_stream::SerdeWrite;
    ///
    /// #[derive(Serialize)]
    /// struct Foo {
    ///     value: u32
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    ///     stream.write_obj(&Foo {
    ///         value: 1222
    ///     }).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    #[async_trait]
    pub trait SerdeWrite {
        async fn write_obj<T: Serialize + Sync>(&mut self, obj: &T) -> Result<(), Error>;
    }

    #[async_trait]
    impl<R: AsyncReadExt + Unpin + Send> SerdeRead for R {
        async fn read_obj<T: DeserializeOwned>(&mut self, max_size: usize) -> Result<T, Error> {
            let mut size_buffer = [0; 8];
            self.read_exact(&mut size_buffer).await?;
            let size = usize::from_be_bytes(size_buffer);

            if size > max_size {
                return Err(Error::BufferTooLarge { size, max_size })
            }

            let mut value_buffer = vec![0; size];
            self.read_exact(&mut value_buffer).await?;

            let res = rmp_serde::from_slice(&value_buffer)?;

            Ok(res)
        }
    }

    #[async_trait]
    impl<W: AsyncWriteExt + Unpin + Send> SerdeWrite for W {
        async fn write_obj<T: Serialize + Sync>(&mut self, obj: &T) -> Result<(), Error> {
            let obj_bytes = rmp_serde::to_vec(obj)?;
            let size = obj_bytes.len();
            let size_bytes = size.to_be_bytes();
            self.write_all(&size_bytes).await?;
            self.write_all(&obj_bytes).await?;
            Ok(())
        }
    }
}