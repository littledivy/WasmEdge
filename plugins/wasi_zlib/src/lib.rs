use wasmedge_sys::ffi;
use wasmedge_sys::{
  AsImport, CallingFrame, FuncType, Function, ImportModule, WasmValue,
};
use wasmedge_types::{error::HostFuncError, ValType};

fn extern_add(
  _: CallingFrame,
  inputs: Vec<WasmValue>,
) -> Result<Vec<WasmValue>, HostFuncError> {
  let val1 = if inputs[0].ty() == ValType::ExternRef {
    inputs[0]
  } else {
    return Err(HostFuncError::User(2));
  };
  let val1 = val1
    .extern_ref::<i32>()
    .expect("fail to get i32 from an ExternRef");
  dbg!(val1);
  let val2 = if inputs[1].ty() == ValType::I32 {
    inputs[1].to_i32()
  } else {
    return Err(HostFuncError::User(3));
  };

  Ok(vec![WasmValue::from_i32(val1 + val2)])
}

unsafe extern "C" fn instance_create(
  _: *const ffi::WasmEdge_ModuleDescriptor,
) -> *mut ffi::WasmEdge_ModuleInstanceContext {
  let mut import =
    std::mem::ManuallyDrop::new(ImportModule::create("zlib").unwrap());
  let func_ty = FuncType::create(
    vec![ValType::ExternRef, ValType::I32],
    vec![ValType::I32],
  )
  .unwrap();
  let host_func = Function::create(&func_ty, Box::new(extern_add), 0).unwrap();
  import.add_func("func_add", host_func);
  import.as_ptr() as *mut _
}

#[export_name = "WasmEdge_Plugin_GetDescriptor"]
pub extern "C" fn init() -> *const ffi::WasmEdge_PluginDescriptor {
  const MODULES: &'static [ffi::WasmEdge_ModuleDescriptor] =
    &[ffi::WasmEdge_ModuleDescriptor {
      Name: "zlib\0".as_ptr() as *const _,
      Description: "Zlib binding\0".as_ptr() as *const _,
      Create: Some(instance_create),
    }];
  const DESC: ffi::WasmEdge_PluginDescriptor = ffi::WasmEdge_PluginDescriptor {
    Name: "zlib\0".as_ptr() as *const _,
    Description: "Zlib bindings\0".as_ptr() as *const _,
    APIVersion: ffi::WasmEdge_Plugin_CurrentAPIVersion,
    Version: ffi::WasmEdge_PluginVersionData {
      Major: 0,
      Minor: 0,
      Patch: 0,
      Build: 0,
    },
    ModuleCount: 0,
    ProgramOptionCount: 0,
    ModuleDescriptions: MODULES.as_ptr() as _,
    ProgramOptions: std::ptr::null_mut(),
  };

  &DESC
}
