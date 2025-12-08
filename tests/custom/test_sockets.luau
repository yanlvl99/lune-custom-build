--!nocheck
--!nocheck
-- Fixed UDP/TCP test - using correct API
local net = require("@lune/net")
local task = require("@lune/task")

print("=== UDP/TCP Test ===")
print("")

-- Test UDP binding
print("1. Testing UDP bind...")
local udpSocket = net.udp.bind("127.0.0.1:0")
print("   Bound to:", udpSocket.address)
print("   UDP bind: OK")

-- Test UDP connect (sets default destination)
udpSocket:connect("127.0.0.1:12345")
print("   UDP connect: OK")
udpSocket:close()

print("")

-- Test TCP listen
print("2. Testing TCP listen...")
local tcpServer = net.tcp.listen("127.0.0.1:0")
local serverAddr = tcpServer.address
print("   Listening on:", serverAddr)
print("   TCP listen: OK")

-- Parse server address to get host and port
local host, port = serverAddr:match("([^:]+):(%d+)")
port = tonumber(port)

print("")

-- Test TCP connect using host, port format
print("3. Testing TCP connect...")
print("   Connecting to", host, "port", port)

-- Spawn server accept in background
task.spawn(function()
	local conn = tcpServer:accept()
	if conn then
		local data = conn:read(1024)
		print("   Server received:", data)
		conn:write("Hello from server!")
		conn:close()
	end
end)

-- Wait for server to be ready
task.wait(0.05)

-- Connect to our own server (using host, port format)
local client = net.tcp.connect(host, port)
print("   Connected!")
client:write("Hello from client!")
local response = client:read(1024)
print("   Server response:", response)
client:close()

task.wait(0.1)
tcpServer:close()

print("")
print("=== UDP/TCP Test Complete - All OK! ===")
