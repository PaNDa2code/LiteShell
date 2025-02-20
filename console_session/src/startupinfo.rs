use std::
    ffi::c_void
;

use windows::{
    core::Result,
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

pub unsafe fn create_startup_info(hpcon: HPCON) -> Result<STARTUPINFOEXW> {
    let mut startup_info_ex = STARTUPINFOEXW::default();
    startup_info_ex.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    let mut bytes_required: usize = 0;

    _ = InitializeProcThreadAttributeList(None, 1, None, &mut bytes_required);

    if bytes_required == 0 {
        return Err(GetLastError().into());
    }

    let heap_handle = GetProcessHeap()?;

    startup_info_ex.lpAttributeList.0 = HeapAlloc(heap_handle, Default::default(), bytes_required);

    if startup_info_ex.lpAttributeList.0.is_null() {
        return Err(GetLastError().into());
    }

    if let Err(e) = InitializeProcThreadAttributeList(
        Some(startup_info_ex.lpAttributeList),
        1,
        None,
        &mut bytes_required,
    ) {
        _ = HeapFree(
            heap_handle,
            Default::default(),
            Some(startup_info_ex.lpAttributeList.0),
        );
        return Err(e);
    };

    if let Err(e) = UpdateProcThreadAttribute(
        startup_info_ex.lpAttributeList,
        0,
        PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
        Some(hpcon.0 as *const c_void),
        size_of::<HPCON>(),
        None,
        None,
    ) {
        _ = HeapFree(
            heap_handle,
            Default::default(),
            Some(startup_info_ex.lpAttributeList.0),
        )?;
        return Err(e);
    };

    Ok(startup_info_ex)
}
