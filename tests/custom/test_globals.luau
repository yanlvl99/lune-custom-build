--!nocheck
--!nocheck
-- Test Extended Globals
print("=== Extended Globals Test ===")
print("")

-- Test math extensions
print("Math Extensions:")
print("  math.clamp(5, 0, 10) =", math.clamp(5, 0, 10))
print("  math.clamp(-5, 0, 10) =", math.clamp(-5, 0, 10))
print("  math.clamp(15, 0, 10) =", math.clamp(15, 0, 10))

-- lerp might not exist, check
if math.lerp then
	print("  math.lerp(0, 100, 0.5) =", math.lerp(0, 100, 0.5))
else
	print("  math.lerp: not available")
end

-- sign
if math.sign then
	print("  math.sign(5) =", math.sign(5))
	print("  math.sign(-5) =", math.sign(-5))
	print("  math.sign(0) =", math.sign(0))
else
	print("  math.sign: not available")
end

-- round
if math.round then
	print("  math.round(3.7) =", math.round(3.7))
else
	print("  math.round: not available")
end

-- tau
if math.tau then
	print("  math.tau =", math.tau)
end

print("")

-- Test UUID if available
print("UUID:")
if uuid then
	local id4 = uuid.v4()
	print("  uuid.v4() =", id4)

	if uuid.v7 then
		local id7 = uuid.v7()
		print("  uuid.v7() =", id7)
	end
else
	print("  uuid: not available")
end

print("")

-- Test warn
print("Testing warn()...")
warn("This is a test warning")

print("")
print("=== Extended Globals Test Complete ===")
