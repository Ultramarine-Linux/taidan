use color_eyre::eyre::{ContextCompat, OptionExt};

use crate::prelude::*;
use std::net::ToSocketAddrs;

/// Process the request, returning a response.
///
/// # Errors
/// If the URL cannot be parsed, or the connection failed, errors will occur
pub async fn https_req(mut req: http_types::Request) -> color_eyre::Result<http_types::Response> {
    //? https://github.com/smol-rs/smol/blob/master/examples/async-h1-client.rs
    let host = req.url().host().context("cannot parse host")?.to_string();
    let port = req
        .url()
        .port_or_known_default()
        .context("cannot guess port")?;
    let socket_addr = {
        let host = host.clone();
        blocking::unblock(move || (&*host, port).to_socket_addrs())
            .await?
            .next()
            .ok_or_eyre("cannot resolve address")?
    };

    let stream = async_io::Async::<std::net::TcpStream>::connect(socket_addr).await?;
    let stream = async_native_tls::connect(host, stream).await?;
    req.insert_header(http_types::headers::USER_AGENT, crate::APPID);
    async_h1::connect(stream, req)
        .await
        .map_err(|e| eyre!("{e}"))
}

pub async fn exist<P: AsRef<std::path::Path>>(path: P) -> bool {
    async_fs::metadata(path).await.is_ok()
}
