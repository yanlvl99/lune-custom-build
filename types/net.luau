export type HttpMethod = "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH"

type HttpQueryOrHeaderMap = { [string]: string | { string } }
export type HttpQueryMap = HttpQueryOrHeaderMap
export type HttpHeaderMap = HttpQueryOrHeaderMap

--[=[
	@interface FetchParamsOptions
	@within Net

	Extra options for `FetchParams`.

	This is a dictionary that may contain one or more of the following values:

	* `decompress` - If the request body should be automatically decompressed when possible. Defaults to `true`
]=]
export type FetchParamsOptions = {
	decompress: boolean?,
}

--[=[
	@interface FetchParams
	@within Net

	Parameters for sending network requests with `net.request`.

	This is a dictionary that may contain one or more of the following values:

	* `url` - The URL to send a request to. This is always required
	* `method` - The HTTP method verb, such as `"GET"`, `"POST"`, `"PATCH"`, `"PUT"`, or `"DELETE"`. Defaults to `"GET"`
	* `body` - The request body
	* `query` - A table of key-value pairs representing query parameters in the request path
	* `headers` - A table of key-value pairs representing headers
	* `options` - Extra options for things such as automatic decompression of response bodies
]=]
export type FetchParams = {
	url: string,
	method: HttpMethod?,
	body: (string | buffer)?,
	query: HttpQueryMap?,
	headers: HttpHeaderMap?,
	options: FetchParamsOptions?,
}

--[=[
	@interface FetchResponse
	@within Net

	Response type for sending network requests with `net.request`.

	This is a dictionary containing the following values:

	* `ok` - If the status code is a canonical success status code, meaning within the range 200 -> 299
	* `statusCode` - The status code returned for the request
	* `statusMessage` - The canonical status message for the returned status code, such as `"Not Found"` for status code 404
	* `headers` - A table of key-value pairs representing headers
	* `body` - The request body, or an empty string if one was not given
]=]
export type FetchResponse = {
	ok: boolean,
	statusCode: number,
	statusMessage: string,
	headers: HttpHeaderMap,
	body: string,
}

--[=[
	@interface ServeRequest
	@within Net

	Data type for requests in `net.serve`.

	This is a dictionary containing the following values:

	* `path` - The path being requested, relative to the root. Will be `/` if not specified
	* `query` - A table of key-value pairs representing query parameters in the request path
	* `method` - The HTTP method verb, such as `"GET"`, `"POST"`, `"PATCH"`, `"PUT"`, or `"DELETE"`. Will always be uppercase
	* `headers` - A table of key-value pairs representing headers
	* `body` - The request body, or an empty string if one was not given
]=]
export type ServeRequest = {
	path: string,
	query: { [string]: string? },
	method: HttpMethod,
	headers: { [string]: string },
	body: string,
}

--[=[
	@interface ServeResponse
	@within Net

	Response type for requests in `net.serve`.

	This is a dictionary that may contain one or more of the following values:

	* `status` - The status code for the request, in the range `100` -> `599`
	* `headers` - A table of key-value pairs representing headers
	* `body` - The response body
]=]
export type ServeResponse = {
	status: number?,
	headers: { [string]: string }?,
	body: (string | buffer)?,
}

type ServeHttpHandler = (request: ServeRequest) -> string | ServeResponse
type ServeWebSocketHandler = (socket: WebSocket) -> ()

--[=[
	@interface ServeConfig
	@within Net

	Configuration for `net.serve`.

	This may contain one of or more of the following values:

	* `address` for setting the IP address to serve from. Defaults to the loopback interface (`http://localhost`).
	* `handleRequest` for handling normal http requests, equivalent to just passing a function to `net.serve`
	* `handleWebSocket` for handling web socket requests, which will receive a `WebSocket` object as its first and only parameter

	When setting `address`, the `handleRequest` callback must also be defined.

	### Example Usage

	```luau
	net.serve(8080, {
		address = "http://0.0.0.0",
		handleRequest = function(request)
			return {
				status = 200,
				body = "Echo:\n" .. request.body,
			}
		end
	})
	```
]=]
export type ServeConfig = {
	address: string?,
	handleRequest: ServeHttpHandler?,
	handleWebSocket: ServeWebSocketHandler?,
}

--[=[
	@interface ServeHandle
	@within Net

	A handle to a currently running web server, containing a single `stop` function to gracefully shut down the web server.
]=]
export type ServeHandle = {
	stop: () -> (),
}

--[=[
	@interface WebSocket
	@within Net

	A reference to a web socket connection.

	The web socket may be in either an "open" or a "closed" state, changing its current behavior.

	When open:

	* Any function on the socket such as `send`, `next` or `close` can be called without erroring
	* `next` can be called to yield until the next message is received or the socket becomes closed

	When closed:

	* `next` will no longer return any message(s) and instead instantly return nil
	* `send` will throw an error stating that the socket has been closed

	Once the websocket has been closed, `closeCode` will no longer be nil, and will be populated with a close
	code according to the [WebSocket specification](https://www.iana.org/assignments/websocket/websocket.xhtml).
	This will be an integer between 1000 and 4999, where 1000 is the canonical code for normal, error-free closure.
]=]
export type WebSocket = {
	closeCode: number?,
	close: (self: WebSocket, code: number?) -> (),
	send: (self: WebSocket, message: (string | buffer)?, asBinaryMessage: boolean?) -> (),
	next: (self: WebSocket) -> string?,
}

--[=[
	@interface TcpConfig
	@within Net

	Configuration options for a TCP stream.

	### Example Usage

	```luau
	-- Plain TCP connection
	local stream = net.tcp.connect("example.com", 80)

	-- TLS connection (shorthand)
	local stream = net.tcp.connect("example.com", 443, true)

	-- TLS connection (explicit config)
	local stream = net.tcp.connect("example.com", 443, { tls = true })

	-- Connection with custom TTL
	local stream = net.tcp.connect("192.168.1.100", 8080, {
		tls = false,
		ttl = 128
	})
	```
]=]
export type TcpConfig = {
	--[=[
		Whether or not to use TLS encryption.

		Defaults to `false`.
	]=]
	tls: boolean?,
	--[=[
		The TTL to use for packets sent over the socket.
	]=]
	ttl: number?,
}

--[=[
	@interface TcpStream
	@within Net

	A plain TCP stream, which may also be backed by a TLS connection.

	### Example Usage

	```luau
	local net = require("@lune/net")

	local conn = net.tcp.connect("example.com", 80)

	conn:write("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")

	local response = conn:read()
	print(response)

	conn:close()
	```
]=]
export type TcpStream = {
	--[=[
		The local IP address of the socket, if any.
	]=]
	localIp: string?,
	--[=[
		The local port of the socket, if any.
	]=]
	localPort: number?,
	--[=[
		The remote IP address of the socket, if any.
	]=]
	remoteIp: string?,
	--[=[
		The remote port of the socket, if any.
	]=]
	remotePort: number?,
	--[=[
		Closes the underlying I/O for the stream.

		Any writes will throw an error after this method is called.
	]=]
	close: (self: TcpStream) -> (),
	--[=[
		Writes the given data to the stream.

		- If the stream is closed, this will throw an error.
	]=]
	write: (self: TcpStream, data: string | buffer) -> (),
	--[=[
		Reads data from the stream, returning a string up to the given `size`.

		- If there is no data to read, this will yield until data is available.
		- If the stream is closed, this will return `nil`.
	]=]
	read: (self: TcpStream, size: number?) -> string?,
}

--[=[
	TCP primitives for the `net` library
]=]
local tcp = {}

--[=[
	Connects to the given host and port, returning a `TcpStream`.
]=]
function tcp.connect(host: string, port: number, config: (true | TcpConfig)?): TcpStream
	return nil :: any
end

--[=[
	Listens on the given address, returning a `TcpServer`.
]=]
function tcp.listen(
	addr: string
): { address: string, accept: (self: any) -> TcpStream?, close: (self: any) -> () }
	return nil :: any
end

--[=[
	@interface UdpSocket
	@within Net

	A UDP socket for connectionless communication.
]=]
export type UdpSocket = {
	--[=[
		The bound address of the socket.
	]=]
	address: string,
	--[=[
		Send data to a target address.
	]=]
	sendTo: (self: UdpSocket, data: string, address: string) -> number,
	--[=[
		Receive data from any sender.
		Returns { data: string, address: string }
	]=]
	recvFrom: (self: UdpSocket, maxSize: number?) -> { data: string, address: string },
	--[=[
		Connect to a remote address for send/recv without address.
	]=]
	connect: (self: UdpSocket, address: string) -> (),
	--[=[
		Send on connected socket.
	]=]
	send: (self: UdpSocket, data: string) -> number,
	--[=[
		Receive on connected socket.
	]=]
	recv: (self: UdpSocket, maxSize: number?) -> string,
	--[=[
		Close the socket.
	]=]
	close: (self: UdpSocket) -> (),
}

--[=[
	UDP primitives for the `net` library
]=]
local udp = {}

--[=[
	Bind to a local address, returning a `UdpSocket`.
]=]
function udp.bind(addr: string): UdpSocket
	return nil :: any
end

--[=[
	@class Net

	Built-in library for network access
]=]
local net = {}

net.tcp = tcp
net.udp = udp

--[=[
	@within Net

	Sends an HTTP request using the given url and / or parameters, and returns a dictionary that describes the response received.
]=]
function net.request(config: string | FetchParams): FetchResponse
	return nil :: any
end

--[=[
	@within Net
	@tag must_use

	Connects to a web socket at the given URL.
]=]
function net.socket(url: string): WebSocket
	return nil :: any
end

--[=[
	@within Net

	Creates an HTTP server that listens on the given `port`.
]=]
function net.serve(port: number, handlerOrConfig: ServeHttpHandler | ServeConfig): ServeHandle
	return nil :: any
end

function net.urlEncode(s: string, binary: boolean?): string
	return nil :: any
end

function net.urlDecode(s: string, binary: boolean?): string
	return nil :: any
end

return net
