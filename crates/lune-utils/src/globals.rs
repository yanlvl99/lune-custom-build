//! Extended global functions for Luau runtime.
//!
//! Provides additional math functions and colored warn output.

use mlua::prelude::*;

/// Inject extended globals into the Lua state.
///
/// # Errors
///
/// Returns error if injection fails.
pub fn inject_globals(lua: &Lua) -> LuaResult<()> {
    inject_math_extensions(lua)?;
    inject_colored_warn(lua)?;
    inject_uuid(lua)?;
    Ok(())
}

fn inject_math_extensions(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();
    let math: LuaTable = globals.get("math")?;

    // math.clamp(value, min, max)
    math.set(
        "clamp",
        lua.create_function(|_, (value, min, max): (f64, f64, f64)| Ok(value.max(min).min(max)))?,
    )?;

    // math.lerp(a, b, t)
    math.set(
        "lerp",
        lua.create_function(|_, (a, b, t): (f64, f64, f64)| Ok(a + (b - a) * t))?,
    )?;

    // math.inverseLerp(a, b, value)
    math.set(
        "inverseLerp",
        lua.create_function(|_, (a, b, value): (f64, f64, f64)| {
            if (b - a).abs() < f64::EPSILON {
                Ok(0.0)
            } else {
                Ok((value - a) / (b - a))
            }
        })?,
    )?;

    // math.map(value, inMin, inMax, outMin, outMax)
    math.set(
        "map",
        lua.create_function(
            |_, (value, in_min, in_max, out_min, out_max): (f64, f64, f64, f64, f64)| {
                let t = (value - in_min) / (in_max - in_min);
                Ok(out_min + (out_max - out_min) * t)
            },
        )?,
    )?;

    // math.sign(value)
    math.set(
        "sign",
        lua.create_function(|_, value: f64| {
            Ok(if value > 0.0 {
                1.0
            } else if value < 0.0 {
                -1.0
            } else {
                0.0
            })
        })?,
    )?;

    // math.round(value, decimals?)
    math.set(
        "roundTo",
        lua.create_function(|_, (value, decimals): (f64, Option<i32>)| {
            let decimals = decimals.unwrap_or(0);
            let factor = 10_f64.powi(decimals);
            Ok((value * factor).round() / factor)
        })?,
    )?;

    // math.tau
    math.set("tau", std::f64::consts::TAU)?;

    Ok(())
}

fn inject_colored_warn(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    globals.set(
        "warn",
        lua.create_function(|_, args: LuaMultiValue| {
            let message: String = args
                .iter()
                .map(|v| match v {
                    LuaValue::String(s) => s.to_str().map_or("?".to_owned(), |s| s.to_owned()),
                    LuaValue::Nil => "nil".to_owned(),
                    LuaValue::Boolean(b) => b.to_string(),
                    LuaValue::Integer(i) => i.to_string(),
                    LuaValue::Number(n) => n.to_string(),
                    _ => format!("{v:?}"),
                })
                .collect::<Vec<_>>()
                .join(" ");

            eprintln!("\x1b[33m[WARN]\x1b[0m {message}");
            Ok(())
        })?,
    )?;

    Ok(())
}

fn inject_uuid(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    // Create uuid table
    let uuid_table = lua.create_table()?;

    // uuid.v4() - Random UUID
    uuid_table.set(
        "v4",
        lua.create_function(|_, ()| Ok(uuid::Uuid::new_v4().to_string()))?,
    )?;

    // uuid.v7() - Time-ordered UUID
    uuid_table.set(
        "v7",
        lua.create_function(|_, ()| Ok(uuid::Uuid::now_v7().to_string()))?,
    )?;

    globals.set("uuid", uuid_table)?;

    Ok(())
}
