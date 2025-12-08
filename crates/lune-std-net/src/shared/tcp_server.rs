//! TCP Server implementation for Luau.
//!
//! Provides async TCP listener with accept loop.

use async_net::{TcpListener as AsyncTcpListener, TcpStream};
use mlua::prelude::*;
use mlua_luau_scheduler::LuaSpawnExt;
use std::sync::Arc;

/// Accepted TCP connection (simpler than client Tcp).
pub struct TcpConnection {
    stream: Arc<async_lock::Mutex<TcpStream>>,
    remote_addr: String,
}

impl TcpConnection {
    fn new(stream: TcpStream, addr: String) -> Self {
        Self {
            stream: Arc::new(async_lock::Mutex::new(stream)),
            remote_addr: addr,
        }
    }

    pub async fn read(&self, size: usize) -> LuaResult<Vec<u8>> {
        use futures_lite::AsyncReadExt;
        let mut buf = vec![0u8; size];
        let mut stream = self.stream.lock().await;
        let len = stream.read(&mut buf).await.into_lua_err()?;
        buf.truncate(len);
        Ok(buf)
    }

    pub async fn write(&self, data: &[u8]) -> LuaResult<usize> {
        use futures_lite::AsyncWriteExt;
        let mut stream = self.stream.lock().await;
        stream.write(data).await.into_lua_err()
    }

    pub async fn close(&self) -> LuaResult<()> {
        use futures_lite::AsyncWriteExt;
        let mut stream = self.stream.lock().await;
        stream.close().await.into_lua_err()
    }
}

impl Clone for TcpConnection {
    fn clone(&self) -> Self {
        Self {
            stream: Arc::clone(&self.stream),
            remote_addr: self.remote_addr.clone(),
        }
    }
}

impl LuaUserData for TcpConnection {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("address", |_, this| Ok(this.remote_addr.clone()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("read", |lua, this, size: Option<usize>| async move {
            let data = this.read(size.unwrap_or(4096)).await?;
            lua.create_string(&data)
        });

        methods.add_async_method("write", |_, this, data: LuaString| async move {
            let bytes = data.as_bytes().to_vec();
            this.write(&bytes).await
        });

        methods.add_async_method("close", |_, this, ()| async move { this.close().await });
    }
}

/// TCP Server that listens for incoming connections.
pub struct TcpServer {
    listener: Arc<AsyncTcpListener>,
    local_addr: String,
}

impl TcpServer {
    /// Bind to a local address and start listening.
    pub async fn listen(addr: &str) -> LuaResult<Self> {
        let listener = AsyncTcpListener::bind(addr).await.into_lua_err()?;

        let local_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| addr.to_owned());

        Ok(Self {
            listener: Arc::new(listener),
            local_addr,
        })
    }

    /// Accept a single incoming connection.
    pub async fn accept(&self) -> LuaResult<TcpConnection> {
        let (stream, addr) = self.listener.accept().await.into_lua_err()?;
        Ok(TcpConnection::new(stream, addr.to_string()))
    }
}

impl Clone for TcpServer {
    fn clone(&self) -> Self {
        Self {
            listener: Arc::clone(&self.listener),
            local_addr: self.local_addr.clone(),
        }
    }
}

impl LuaUserData for TcpServer {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("address", |_, this| Ok(this.local_addr.clone()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // accept() -> TcpConnection
        methods.add_async_method("accept", |_, this, ()| async move { this.accept().await });

        // serve(handler: (socket) -> ()) - Run accept loop with callback
        methods.add_method("serve", |lua, this, handler: LuaFunction| {
            let server = this.clone();

            lua.spawn_local(async move {
                loop {
                    match server.accept().await {
                        Ok(conn) => {
                            if let Err(e) = handler.call::<()>((conn,)) {
                                eprintln!("\x1b[33m[WARN]\x1b[0m TCP handler error: {e}");
                            }
                        }
                        Err(e) => {
                            eprintln!("\x1b[31m[ERROR]\x1b[0m TCP accept error: {e}");
                            break;
                        }
                    }
                }
            });

            Ok(())
        });

        methods.add_method("close", |_, _, ()| Ok(()));
    }
}

/// Create a TCP server listening on the given address.
pub async fn net_tcp_listen(_: Lua, addr: String) -> LuaResult<TcpServer> {
    TcpServer::listen(&addr).await
}
