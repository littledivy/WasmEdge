mod ffi {
  #[link(wasm_import_module = "zlib")]
  extern "C" {
      pub fn adler32(adler: u32, buf: *const u8, len: u32) -> u32;
      pub fn deflateInit(strm: *mut super::z_stream, level: i32, version: *const u8, stream_size: i32) -> i32;
      pub fn deflate(strm: *mut super::z_stream, flush: i32) -> i32;
  }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct z_stream {
    pub next_in: *const u8,
    pub avail_in: i32,
    pub total_in: i32,
    pub next_out:  *const u8,
    pub avail_out: i32,
    pub total_out: i32,
    pub msg:  *const u8,
    pub state:  *const u8,
    pub zalloc:  *const u8,
    pub zfree:  *const u8,
    pub opaque:  *const u8,
    pub data_type: i32,
    pub adler: i32,
    pub reserved: i32,
}

fn main() {
  let adler = unsafe { ffi::adler32(1, "hello".as_ptr(), 5) };
  println!("adler32: {}", adler);
  
  let mut strm = unsafe { std::mem::zeroed::<z_stream>() };
  let version = "1.2.11".as_ptr();
  let status = unsafe { ffi::deflateInit(&mut strm, -1, version, std::mem::size_of::<z_stream>() as i32) };
  println!("deflateInit: {}", status);

  let input = b"hello";
  strm.next_in = input.as_ptr();
  strm.avail_in = input.len() as _;

  let mut output = [0u8; 1024];
  strm.next_out = &mut output as *mut _;
  strm.avail_out = output.len() as _;

  let status = unsafe { ffi::deflate(&mut strm, 0) };
  println!("deflate: {}", status);
  println!("output: {:?}", &output[..strm.total_out as usize]);
}
