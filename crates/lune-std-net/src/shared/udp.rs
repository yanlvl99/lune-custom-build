//! UDP Socket implementation for Luau.
//!
//! Provides async UDP bind, send, and receive operations.

use async_io::Async;
use futures_lite::future;
use mlua::prelude::*;
use std::net::UdpSocket as StdUdpSocket;
use std::sync::Arc;

/// Async UDP socket wrapper for Lua userdata.
pub struct UdpSocket {
    inner: Arc<Async<StdUdpSocket>>,
    bound_addr: String,
}

impl UdpSocket {
    /// Bind to a local address.
    pub fn bind(addr: &str) -> LuaResult<Self> {
        let socket = StdUdpSocket::bind(addr).into_lua_err()?;
        socket.set_nonblocking(true).into_lua_err()?;

        let bound_addr = socket
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| addr.to_owned());

        let async_socket = Async::new(socket).into_lua_err()?;

        Ok(Self {
            inner: Arc::new(async_socket),
            bound_addr,
        })
    }

    /// Send data to a target address.
    pub async fn send_to(&self, data: &[u8], target: &str) -> LuaResult<usize> {
        let target: std::net::SocketAddr = target.parse().into_lua_err()?;
        self.inner
            .write_with(|sock| sock.send_to(data, target))
            .await
            .into_lua_err()
    }

    /// Receive data with sender address.
    pub async fn recv_from(&self, max_size: usize) -> LuaResult<(Vec<u8>, String)> {
        let mut buf = vec![0u8; max_size];
        let (len, addr) = self
            .inner
            .read_with(|sock| sock.recv_from(&mut buf))
            .await
            .into_lua_err()?;

        buf.truncate(len);
        Ok((buf, addr.to_string()))
    }

    /// Connect to a remote address for send/recv without address.
    pub fn connect(&self, addr: &str) -> LuaResult<()> {
        self.inner.get_ref().connect(addr).into_lua_err()
    }

    /// Send on connected socket.
    pub async fn send(&self, data: &[u8]) -> LuaResult<usize> {
        self.inner
            .write_with(|sock| sock.send(data))
            .await
            .into_lua_err()
    }

    /// Receive on connected socket.
    pub async fn recv(&self, max_size: usize) -> LuaResult<Vec<u8>> {
        let mut buf = vec![0u8; max_size];
        let len = self
            .inner
            .read_with(|sock| sock.recv(&mut buf))
            .await
            .into_lua_err()?;

        buf.truncate(len);
        Ok(buf)
    }
}

impl Clone for UdpSocket {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            bound_addr: self.bound_addr.clone(),
        }
    }
}

impl LuaUserData for UdpSocket {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("address", |_, this| Ok(this.bound_addr.clone()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // sendTo(data: buffer, address: string) -> number
        methods.add_async_method(
            "sendTo",
            |_, this, (data, target): (LuaString, String)| async move {
                let bytes = data.as_bytes().to_vec();
                this.send_to(&bytes, &target).await
            },
        );

        // recvFrom(maxSize?: number) -> { data: buffer, address: string }
        methods.add_async_method(
            "recvFrom",
            |lua, this, max_size: Option<usize>| async move {
                let (data, addr) = this.recv_from(max_size.unwrap_or(65535)).await?;
                let result = lua.create_table()?;
                result.set("data", lua.create_string(&data)?)?;
                result.set("address", addr)?;
                Ok(result)
            },
        );

        // connect(address: string) -> ()
        methods.add_method("connect", |_, this, addr: String| this.connect(&addr));

        // send(data: buffer) -> number
        methods.add_async_method("send", |_, this, data: LuaString| async move {
            let bytes = data.as_bytes().to_vec();
            this.send(&bytes).await
        });

        // recv(maxSize?: number) -> buffer
        methods.add_async_method("recv", |lua, this, max_size: Option<usize>| async move {
            let data = this.recv(max_size.unwrap_or(65535)).await?;
            lua.create_string(&data)
        });

        // close() - not really needed as drop handles it, but for explicitness
        methods.add_method("close", |_, _, ()| Ok(()));
    }
}

/// Bind a new UDP socket.
pub async fn net_udp_bind(_: Lua, addr: String) -> LuaResult<UdpSocket> {
    // Run bind in blocking context since it might do DNS
    future::block_on(async { UdpSocket::bind(&addr) })
}
