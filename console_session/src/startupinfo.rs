use std::ffi::c_void;

use windows::{
    core::{w, Result},
    Win32::{
        Foundation::GetLastError,
        System::{
            Console::HPCON,
            Memory::{GetProcessHeap, HeapAlloc, HeapFree},
            Threading::{
                InitializeProcThreadAttributeList, UpdateProcThreadAttribute,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTUPINFOEXW,
            },
        },
    },
};

pub fn create_startup_info(hpcon: HPCON) -> Result<STARTUPINFOEXW> {
    let mut startup_info_ex = STARTUPINFOEXW::default();
    startup_info_ex.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    let mut bytes_required: usize = 0;

    _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut bytes_required) };

    if bytes_required == 0 {
        return Err(unsafe { GetLastError().into() });
    }

    let heap_handle = unsafe { GetProcessHeap() }?;

    startup_info_ex.lpAttributeList.0 =
        unsafe { HeapAlloc(heap_handle, Default::default(), bytes_required) };

    if startup_info_ex.lpAttributeList.0.is_null() {
        return Err(unsafe { GetLastError().into() });
    }

    if let Err(e) = unsafe {
        InitializeProcThreadAttributeList(
            Some(startup_info_ex.lpAttributeList),
            1,
            None,
            &mut bytes_required,
        )
    } {
        _ = unsafe {
            HeapFree(
                heap_handle,
                Default::default(),
                Some(startup_info_ex.lpAttributeList.0),
            )
        };
        return Err(e);
    };

    if let Err(e) = unsafe {
        UpdateProcThreadAttribute(
            startup_info_ex.lpAttributeList,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            Some(hpcon.0 as *const c_void),
            size_of::<HPCON>(),
            None,
            None,
        )
    } {
        _ = unsafe {
            HeapFree(
                heap_handle,
                Default::default(),
                Some(startup_info_ex.lpAttributeList.0),
            )
        }?;
        return Err(e);
    };

    Ok(startup_info_ex)
}
