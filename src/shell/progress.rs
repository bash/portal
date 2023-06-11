use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{io, thread};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Shell::{
    ITaskbarList3, TaskbarList, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellProgress {
    None,
    Indeterminate,
    Normal(u64, u64),
}

pub(crate) struct ProgressReporter {
    sender: Sender<(HWND, ShellProgress)>,
    current_value: ShellProgress,
}

impl ProgressReporter {
    pub(crate) fn start() -> Self {
        let (sender, receiver) = channel();
        let worker = thread::spawn(move || progress_worker(receiver));
        Self {
            sender,
            current_value: ShellProgress::None,
        }
    }

    pub(crate) fn report<W: HasRawWindowHandle>(&mut self, parent: &W, progress: ShellProgress) {
        let hwnd = match parent.raw_window_handle() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd as isize),
            _ => unreachable!(),
        };
        if progress != self.current_value {
            self.current_value = progress;
            _ = self.sender.send((hwnd, progress));
        }
    }
}

fn progress_worker(receiver: Receiver<(HWND, ShellProgress)>) {
    progress_worker_internal(receiver);
    // TODO: CoUninitialize, log errors
}

fn progress_worker_internal(receiver: Receiver<(HWND, ShellProgress)>) -> io::Result<()> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED)? };

    let taskbar: ITaskbarList3 = unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_ALL)? };

    while let Ok((hwnd, progress)) = receiver.recv() {
        let state = match progress {
            ShellProgress::None => TBPF_NOPROGRESS,
            ShellProgress::Indeterminate => TBPF_INDETERMINATE,
            ShellProgress::Normal(_, _) => TBPF_NORMAL,
        };
        unsafe { taskbar.SetProgressState(hwnd, state)? };
        if let ShellProgress::Normal(value, total) = progress {
            unsafe { taskbar.SetProgressValue(hwnd, value, total)? };
        }
    }

    Ok(())
}
