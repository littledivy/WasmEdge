mod ffi {
  #[link(wasm_import_module = "zlib")]
  extern "C" {
      pub fn func_add(a: i32, b: i32) -> i32;
  }
}
fn main() {
  let a = 1;
  let b = 2;
  let c = unsafe { ffi::func_add(a, b) };
  println!("{} + {} = {}", a, b, c);
}
