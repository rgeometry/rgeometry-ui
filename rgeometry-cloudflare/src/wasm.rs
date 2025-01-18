use serde::{Deserialize, Serialize};
use std::str;
use std::{collections::HashMap, ffi::CStr};
use wasmi::core::{F32, F64};
use wasmi::{self, core::ValType, Engine, Extern, FuncType, Instance, Module, Store};
use wasmi::{Linker, Val};
use web_time::Instant;

/// JSON schema for render parameters. Each parameter can be one of:
/// ```json
/// {
///   "type": "time"                     // Represents time, typically for animations
/// }
/// ```
/// ```json
/// {
///   "type": "range_f32",
///   "min": 0.0,                        // Minimum value (float)
///   "max": 100.0,                      // Maximum value (float)
///   "default": 50.0                    // Default value (float)
/// }
/// ```
/// ```json
/// {
///   "type": "range_i32",
///   "min": 0,                          // Minimum value (integer)
///   "max": 100,                        // Maximum value (integer)
///   "default": 50                      // Default value (integer)
/// }
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SchemaType {
    Time,
    RangeF32 { min: f32, max: f32, default: f32 },
    RangeI32 { min: i32, max: i32, default: i32 },
}

impl SchemaType {
    fn to_val_type(&self) -> ValType {
        match self {
            SchemaType::Time => ValType::F64, // Time is typically passed as f64
            SchemaType::RangeF32 { .. } => ValType::F32,
            SchemaType::RangeI32 { .. } => ValType::I32,
        }
    }
}

type Schema = Vec<SchemaType>;

#[derive(Debug)]
pub struct Wasm {
    instance: Instance,
    schema: Schema,
    store: Store<String>,
    created_at: Instant,
    parameters: HashMap<usize, Val>,
    failed: bool,
}

impl Wasm {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        // Create a new WASM engine
        let engine = Engine::default();

        // Parse the module
        let module =
            Module::new(&engine, bytes).map_err(|e| format!("Failed to create module: {}", e))?;

        // Create store with state
        let mut store = Store::new(&engine, String::new());

        // // Define the host function for 'render'
        // let render_func = Func::new(
        //     &mut store,
        //     FuncType::new([wasmi::core::ValType::I32], []),
        //     |mut caller, params, _results| {
        //         // Get the pointer to the string from the first parameter
        //         let ptr = params[0]
        //             .i32()
        //             .ok_or(wasmi::Error::new("Failed to get i32 parameter"))?;

        //         // Get the memory from the caller
        //         let memory = caller
        //             .get_export("memory")
        //             .and_then(Extern::into_memory)
        //             .ok_or(wasmi::Error::new("Failed to get memory"))?;

        //         // Read the memory starting from ptr until null terminator
        //         let data = memory.data(&caller);
        //         let result = CStr::from_bytes_until_nul(&data[ptr as usize..])
        //             .map_err(|_| wasmi::Error::new("Failed to read null-terminated string"))?
        //             .to_str()
        //             .map_err(|_| wasmi::Error::new("Invalid UTF-8 string"))?;

        //         // Store the rendered string in the state
        //         *caller.data_mut() = result.to_string();

        //         Ok(())
        //     },
        // );

        let mut linker = <Linker<String>>::new(&engine);
        linker
            .func_new(
                "env",
                "render",
                FuncType::new([wasmi::core::ValType::I32], []),
                |mut caller, params, _results| {
                    // Get the pointer to the string from the first parameter
                    let ptr = params[0]
                        .i32()
                        .ok_or(wasmi::Error::new("Failed to get i32 parameter"))?;

                    // Get the memory from the caller
                    let memory = caller
                        .get_export("memory")
                        .and_then(Extern::into_memory)
                        .ok_or(wasmi::Error::new("Failed to get memory"))?;

                    // Read the memory starting from ptr until null terminator
                    let data = memory.data(&caller);
                    let result = CStr::from_bytes_until_nul(&data[ptr as usize..])
                        .map_err(|_| wasmi::Error::new("Failed to read null-terminated string"))?
                        .to_str()
                        .map_err(|_| wasmi::Error::new("Invalid UTF-8 string"))?;

                    // Store the rendered string in the state
                    *caller.data_mut() = result.to_string();

                    Ok(())
                },
            )
            .map_err(|e| format!("Failed to create render function: {}", e))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("Failed to instantiate module: {}", e))?
            .ensure_no_start(&mut store)
            .map_err(|e| format!("Failed to start module: {}", e))?;

        // // Create import object with only the 'render' function
        // let imports = [(Extern::Func(render_func))];

        // // Instantiate the module
        // let instance = Instance::new(&mut store, &module, &imports)
        //     .map_err(|e| format!("Failed to instantiate module: {}", e))?;

        // Check for required 'request_animation_frame' export
        let request_animation_frame = instance
            .get_func(&store, "request_animation_frame")
            .ok_or("Module must export 'request_animation_frame'")?;
        let request_animation_frame_ty = request_animation_frame.ty(&store);

        // Get memory
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or("Module must export memory")?;

        // Try to get SCHEMA global (default to empty vec if not found)
        let schema = if let Some(global) = instance.get_global(&store, "SCHEMA") {
            let ptr = global.get(&store).i32().ok_or("SCHEMA must be i32")? as u32;

            // Read null-terminated string from memory using CStr
            let schema_str = CStr::from_bytes_until_nul(&memory.data(&store)[ptr as usize..])
                .map_err(|_| "SCHEMA must be null-terminated")?
                .to_str()
                .map_err(|_| "SCHEMA must be valid UTF-8")?;

            // Parse the JSON string into Schema
            serde_json::from_str(schema_str).map_err(|e| format!("Invalid schema JSON: {}", e))?
        } else {
            Vec::new()
        };

        // Convert schema types to expected parameter types
        let expected_params: Vec<wasmi::core::ValType> = schema
            .iter()
            .map(|s: &SchemaType| s.to_val_type())
            .collect();

        // Get the actual parameter types from the function
        let actual_params = request_animation_frame_ty.params();

        // Verify that the parameter types match
        if actual_params != expected_params.as_slice() {
            return Err(format!(
                "request_animation_frame parameters don't match schema. Expected: {:?}, Got: {:?}",
                expected_params, actual_params
            ));
        }

        if !request_animation_frame_ty.results().is_empty() {
            return Err("request_animation_frame must not return any values".to_string());
        }

        Ok(Self {
            instance,
            schema,
            store,
            created_at: Instant::now(),
            parameters: HashMap::new(),
            failed: false,
        })
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    // Merge the schema definition with the given parameters to yield a vector
    // of values that will be passed to the request_animation_frame function.
    fn parameters_at(&self, now: Instant) -> Vec<Val> {
        let mut parameters = Vec::new();
        for (i, param) in self.schema.iter().enumerate() {
            let value = match *param {
                SchemaType::Time => {
                    // Convert duration to seconds as f64
                    let seconds = (now - self.created_at).as_secs_f64();
                    Val::F64(F64::from_float(seconds))
                }
                SchemaType::RangeF32 { default, .. } => self
                    .parameters
                    .get(&i)
                    .cloned()
                    .unwrap_or(Val::F32(F32::from_float(default))),
                SchemaType::RangeI32 { default, .. } => self
                    .parameters
                    .get(&i)
                    .cloned()
                    .unwrap_or(Val::I32(default)),
            };
            parameters.push(value);
        }
        parameters
    }

    pub fn render(&mut self) -> String {
        if self.failed {
            return self.store.data().clone();
        }

        let func = self
            .instance
            .get_func(&self.store, "request_animation_frame")
            .expect("request_animation_frame function not found");

        let params = self.parameters_at(Instant::now());
        if let Err(err) = func.call(&mut self.store, &params, &mut []) {
            log::error!("Failed to call request_animation_frame: {}", err);
            self.failed = true;
        }

        self.store.data().clone()
    }
}
// Panic: panicked at std/src/panicking.rs:131:9:cannot modify the panic hook from a panicking thread

#[cfg(test)]
mod tests {
    use super::*;
    use wat::parse_str;

    const SIMPLE_WAT: &str = r#"
        (module
            (type (;0;) (func (param i32)))
            (import "env" "render" (func (;0;) (type 0)))
            (global $SCHEMA (export "SCHEMA") i32 i32.const 1)
            (memory (export "memory") 1)
            (data (i32.const 1) "[{\"type\":\"time\"}]\00")
            (func (export "request_animation_frame"))
        )"#;

    const ACTUAL_WAT: &str = r#"
        (module
        (type (;0;) (func (param i32)))
        (type (;1;) (func))
        (import "env" "render" (func (;0;) (type 0)))
        (func (;1;) (type 1)
            i32.const 1048579
            call 0)
        (table (;0;) 1 1 funcref)
        (memory (;0;) 17)
        (global (;0;) (mut i32) (i32.const 1048576))
        (global (;1;) i32 (i32.const 1048576))
        (global (;2;) i32 (i32.const 1048592))
        (global (;3;) i32 (i32.const 1048592))
        (export "memory" (memory 0))
        (export "request_animation_frame" (func 1))
        (export "SCHEMA" (global 1))
        (export "__data_end" (global 2))
        (export "__heap_base" (global 3))
        (data (;0;) (i32.const 1048576) "[]\00Hello, world!"))"#;

    const WAT_WITH_PARAMS: &str = r#"
        (module
            (type (;0;) (func (param i32)))
            (type (;1;) (func (param f32 i32)))
            (import "env" "render" (func (;0;) (type 0)))
            (global $SCHEMA (export "SCHEMA") i32 i32.const 1)
            (memory (export "memory") 1)
            (data (i32.const 1) "[{\"type\":\"range_f32\",\"min\":0.0,\"max\":1.0,\"default\":0.5},{\"type\":\"range_i32\",\"min\":0,\"max\":100,\"default\":50}]\00")
            (func (export "request_animation_frame") (param f32 i32))
        )"#;

    #[test]
    fn test_schema() {
        let wasm_bytes = parse_str(SIMPLE_WAT).unwrap();
        let wasm = Wasm::new(&wasm_bytes).unwrap();
        assert_eq!(wasm.schema().len(), 1);
        matches!(wasm.schema()[0], SchemaType::Time);
    }

    #[test]
    fn minimal_schema() {
        let wasm_bytes = parse_str(ACTUAL_WAT).unwrap();
        let mut wasm = Wasm::new(&wasm_bytes).unwrap();
        assert!(wasm.schema().is_empty());
        assert_eq!(wasm.render(), "Hello, world!");
    }

    #[test]
    fn test_render_type_time_serialization() {
        let time = SchemaType::Time;
        let json = serde_json::to_string(&time).unwrap();
        assert_eq!(json, r#"{"type":"time"}"#);
    }

    #[test]
    fn test_render_type_time_deserialization() {
        let json = r#"{"type":"time"}"#;
        let time: SchemaType = serde_json::from_str(json).unwrap();
        matches!(time, SchemaType::Time);
    }

    #[test]
    fn test_render_type_range_f32_serialization() {
        let range = SchemaType::RangeF32 {
            min: 0.0,
            max: 100.0,
            default: 50.0,
        };
        let json = serde_json::to_string(&range).unwrap();
        assert_eq!(
            json,
            r#"{"type":"range_f32","min":0.0,"max":100.0,"default":50.0}"#
        );
    }

    #[test]
    fn test_render_type_range_f32_deserialization() {
        let json = r#"{"type":"range_f32","min":0.0,"max":100.0,"default":50.0}"#;
        let range: SchemaType = serde_json::from_str(json).unwrap();
        match range {
            SchemaType::RangeF32 { min, max, default } => {
                assert_eq!(min, 0.0);
                assert_eq!(max, 100.0);
                assert_eq!(default, 50.0);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_render_type_range_i32_serialization() {
        let range = SchemaType::RangeI32 {
            min: 0,
            max: 100,
            default: 50,
        };
        let json = serde_json::to_string(&range).unwrap();
        assert_eq!(
            json,
            r#"{"type":"range_i32","min":0,"max":100,"default":50}"#
        );
    }

    #[test]
    fn test_render_type_range_i32_deserialization() {
        let json = r#"{"type":"range_i32","min":0,"max":100,"default":50}"#;
        let range: SchemaType = serde_json::from_str(json).unwrap();
        match range {
            SchemaType::RangeI32 { min, max, default } => {
                assert_eq!(min, 0);
                assert_eq!(max, 100);
                assert_eq!(default, 50);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_render_type_invalid_json() {
        let json = r#"{"type":"invalid_type"}"#;
        let result: Result<SchemaType, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_params_match() {
        let wasm_bytes = parse_str(WAT_WITH_PARAMS).unwrap();
        let wasm = Wasm::new(&wasm_bytes).unwrap();
        assert_eq!(wasm.schema().len(), 2);
        matches!(&wasm.schema()[0], SchemaType::RangeF32 { .. });
        matches!(&wasm.schema()[1], SchemaType::RangeI32 { .. });
    }

    #[test]
    fn test_schema_params_mismatch() {
        let wat = r#"
            (module
                (type (;0;) (func (param i32)))
                (type (;1;) (func (param i32)))
                (import "env" "render" (func (;0;) (type 0)))
                (global $SCHEMA (export "SCHEMA") i32 i32.const 1)
                (memory (export "memory") 1)
                (data (i32.const 1) "[{\"type\":\"range_f32\",\"min\":0.0,\"max\":1.0,\"default\":0.5}]\00")
                (func (export "request_animation_frame") (param i32))
            )"#;
        let wasm_bytes = parse_str(wat).unwrap();
        let result = Wasm::new(&wasm_bytes);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("parameters don't match schema"));
    }
}
