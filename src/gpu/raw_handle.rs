use std::sync::Arc;

pub trait HasRawAshHandle<T> {
    unsafe fn get_ash_handle<'t>(self: &'t Arc<Self>) -> &'t T;
}

pub trait HasRawVkHandle<T> {
    unsafe fn get_vk_handle(&self) -> T;
}
