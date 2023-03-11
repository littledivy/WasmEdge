use wasmedge_sys::ffi;
use wasmedge_sys::{
  AsImport, CallingFrame, FuncType, Function, ImportModule, WasmValue,
};
use wasmedge_types::{error::HostFuncError, ValType};

struct ValueConverter(WasmValue);

impl From<ValueConverter> for i32 {
  fn from(v: ValueConverter) -> Self {
    v.0.to_i32()
  }
}

impl From<ValueConverter> for u32 {
  fn from(v: ValueConverter) -> Self {
    v.0.to_i32() as _
  }
}

impl From<ValueConverter> for *const u8 {
  fn from(v: ValueConverter) -> Self {
    v.0.to_i32() as _
  }
}

impl From<ValueConverter> for u64 {
  fn from(v: ValueConverter) -> Self {
    v.0.to_i32() as _
  }
}

fn frame_memory(
  frame: &CallingFrame,
  index: u32,
  offset: u32,
) -> Result<*mut u8, HostFuncError> {
  let mut memory = frame.memory_mut(0).unwrap();
  let data = memory.data_pointer_mut(index, offset).unwrap();
  Ok(data)
}

fn w_adler32(
  frame: CallingFrame,
  inputs: Vec<WasmValue>,
) -> Result<Vec<WasmValue>, HostFuncError> {
  let adler = ValueConverter(inputs[0]).into();
  let start = ValueConverter(inputs[1]).into();
  let offset = ValueConverter(inputs[2]).into();

  let ptr = frame_memory(&frame, start, offset)?;

  let checksum = unsafe { libz_sys::adler32(adler, ptr, offset) };

  Ok(vec![WasmValue::from_i32(checksum as _)])
}

fn w_crc32(
  frame: CallingFrame,
  inputs: Vec<WasmValue>,
) -> Result<Vec<WasmValue>, HostFuncError> {
  let crc = ValueConverter(inputs[0]).into();
  let start = ValueConverter(inputs[1]).into();
  let offset = ValueConverter(inputs[2]).into();

  let ptr = frame_memory(&frame, start, offset)?;

  let checksum = unsafe { libz_sys::crc32(crc, ptr, offset) };

  Ok(vec![WasmValue::from_i32(checksum as _)])
}

mod zlib_32 {
  use super::*;
  mod flate2_libz_helpers {
    // Workaround for https://github.com/rust-lang/libz-sys/issues/55
    // See https://github.com/rust-lang/flate2-rs/blob/31fb07820345691352aaa64f367c1e482ad9cfdc/src/ffi/c.rs#L60
    use std::os::raw::c_void;
    use std::{
      alloc::{self, Layout},
      ptr,
    };

    const ALIGN: usize = std::mem::align_of::<usize>();

    fn align_up(size: usize, align: usize) -> usize {
      (size + align - 1) & !(align - 1)
    }

    pub extern "C" fn zalloc(
      _ptr: *mut c_void,
      items: u32,
      item_size: u32,
    ) -> *mut c_void {
      // We need to multiply `items` and `item_size` to get the actual desired
      // allocation size. Since `zfree` doesn't receive a size argument we
      // also need to allocate space for a `usize` as a header so we can store
      // how large the allocation is to deallocate later.
      let size = match (items as usize)
        .checked_mul(item_size as usize)
        .map(|size| align_up(size, ALIGN))
        .and_then(|i| i.checked_add(std::mem::size_of::<usize>()))
      {
        Some(i) => i,
        None => return ptr::null_mut(),
      };

      // Make sure the `size` isn't too big to fail `Layout`'s restrictions
      let layout = match Layout::from_size_align(size, ALIGN) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
      };

      unsafe {
        // Allocate the data, and if successful store the size we allocated
        // at the beginning and then return an offset pointer.
        let ptr = alloc::alloc(layout) as *mut usize;
        if ptr.is_null() {
          return ptr as *mut c_void;
        }
        *ptr = size;
        ptr.add(1) as *mut c_void
      }
    }

    pub extern "C" fn zfree(_ptr: *mut c_void, address: *mut c_void) {
      unsafe {
        // Move our address being free'd back one pointer, read the size we
        // stored in `zalloc`, and then free it using the standard Rust
        // allocator.
        let ptr = (address as *mut usize).offset(-1);
        let size = *ptr;
        let layout = Layout::from_size_align_unchecked(size, ALIGN);
        alloc::dealloc(ptr as *mut u8, layout)
      }
    }
  }

  #[repr(transparent)]
  #[derive(Debug, Copy, Clone)]
  pub struct Ptr<T>(u32, std::marker::PhantomData<T>);

  const _: () = {
    assert!(std::mem::size_of::<Ptr<()>>() == 4);
  };

  #[repr(C)]
  #[derive(Debug, Copy, Clone)]
  pub struct z_stream {
    pub next_in: Ptr<u8>,
    pub avail_in: i32,
    pub total_in: i32,
    pub next_out: Ptr<u8>,
    pub avail_out: i32,
    pub total_out: i32,
    pub msg: Ptr<u8>,
    pub state: Ptr<()>,
    pub zalloc: Ptr<()>,
    pub zfree: Ptr<()>,
    pub opaque: Ptr<()>,
    pub data_type: i32,
    pub adler: i32,
    pub reserved: i32,
  }

  impl z_stream {
    pub fn to_native(&self, frame: &CallingFrame) -> libz_sys::z_stream {
      libz_sys::z_stream {
        next_in: frame_memory(frame, self.next_in.0, 0).unwrap() as _,
        avail_in: self.avail_in as _,
        total_in: self.total_in as _,
        next_out: frame_memory(frame, self.next_out.0, 0).unwrap() as _,
        avail_out: self.avail_out as _,
        total_out: self.total_out as _,
        msg: frame_memory(frame, self.msg.0, 0).unwrap() as _,
        state: frame_memory(frame, self.state.0, 0).unwrap() as _,
        zalloc: flate2_libz_helpers::zalloc,
        zfree: flate2_libz_helpers::zfree,
        opaque: frame_memory(frame, self.opaque.0, 0).unwrap() as _,
        data_type: self.data_type as _,
        adler: self.adler as _,
        reserved: self.reserved as _,
      }
    }
  }
}

fn w_deflateInit(
  frame: CallingFrame,
  inputs: Vec<WasmValue>,
) -> Result<Vec<WasmValue>, HostFuncError> {
  let strm: u32 = ValueConverter(inputs[0]).into();
  let level: i32 = ValueConverter(inputs[1]).into();
  let version: u32 = ValueConverter(inputs[2]).into();
  let stream_size: i32 = ValueConverter(inputs[3]).into();

  let strm = frame_memory(&frame, strm, 0)?;
  let version = frame_memory(&frame, version, 0)?;
  let strm = unsafe { &mut *(strm as *mut zlib_32::z_stream) };

  let mut strm64 = strm.to_native(&frame);
  let status = unsafe {
    libz_sys::deflateInit_(
      &mut strm64 as *mut _,
      level,
      libz_sys::zlibVersion(),
      std::mem::size_of::<libz_sys::z_stream>() as _,
    )
  };

  Ok(vec![WasmValue::from_i32(status as _)])
}

unsafe extern "C" fn instance_create(
  _: *const ffi::WasmEdge_ModuleDescriptor,
) -> *mut ffi::WasmEdge_ModuleInstanceContext {
  let mut import =
    std::mem::ManuallyDrop::new(ImportModule::create("zlib").unwrap());
  let func_ty = FuncType::create(
    vec![ValType::I32, ValType::I32, ValType::I32],
    vec![ValType::I32],
  )
  .unwrap();
  let adler32 = Function::create(&func_ty, Box::new(w_adler32), 0).unwrap();
  let crc32 = Function::create(&func_ty, Box::new(w_crc32), 0).unwrap();
  import.add_func("adler32", adler32);
  import.add_func("crc32", crc32);
  let func_ty = FuncType::create(
    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
    vec![ValType::I32],
  )
  .unwrap();
  let deflateInit =
    Function::create(&func_ty, Box::new(w_deflateInit), 0).unwrap();
  import.add_func("deflateInit", deflateInit);
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
    ModuleCount: MODULES.len() as _,
    ProgramOptionCount: 0,
    ModuleDescriptions: MODULES.as_ptr() as _,
    ProgramOptions: std::ptr::null_mut(),
  };

  &DESC
}
