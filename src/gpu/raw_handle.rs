pub trait HasRawAshHandle<T> {
    unsafe fn get_ash_handle<'t>(&'t self) -> &'t T;
}

pub trait HasRawVkHandle<T> {
    unsafe fn get_vk_handle(&self) -> T;
}
