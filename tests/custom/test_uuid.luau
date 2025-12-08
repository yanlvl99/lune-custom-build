--!nocheck
--!nocheck
-- Test UUID
print("=== UUID Test ===")

if uuid then
	print("uuid exists!")
	local id4 = uuid.v4()
	print("uuid.v4() =", id4)

	if uuid.v7 then
		local id7 = uuid.v7()
		print("uuid.v7() =", id7)
	else
		print("uuid.v7 not available")
	end
else
	print("uuid is nil - not injected as global")
end

print("")
print("=== UUID Test Complete ===")
