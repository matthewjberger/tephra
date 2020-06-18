pub mod camera;
pub mod input;
pub mod vulkan;

/// # Safety
///
/// This method will convert any slice to a byte slice.
/// Use with slices of number primitives.
pub unsafe fn byte_slice_from<T: Sized>(data: &T) -> &[u8] {
    let data_ptr = (data as *const T) as *const u8;
    std::slice::from_raw_parts(data_ptr, std::mem::size_of::<T>())
}
