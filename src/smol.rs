use std::{
    future::{self, Future},
    io,
    os::windows::io::{AsRawHandle, OwnedHandle},
    pin::Pin,
    task::{Context, Poll},
};

use async_io::os::windows::Waitable;
use futures::{Stream, ready};
use windows::{
    Win32::{Foundation::HANDLE, System::Threading::ResetEvent},
    core::Result,
};
use xenstore_rs::{AsyncWatch, AsyncXs, Xs};

use crate::{WatchContext, XsWindows};

pub struct XsSmolWindows(XsWindows);

impl XsSmolWindows {
    pub async fn new() -> Result<Self> {
        Ok(Self(XsWindows::new()?))
    }
}

// TODO: Find a way to use overlapped IO instead.
impl AsyncXs for XsSmolWindows {
    fn directory(&self, path: &str) -> impl Future<Output = io::Result<Vec<Box<str>>>> + Send {
        future::ready(self.0.directory(path))
    }

    fn read(&self, path: &str) -> impl Future<Output = io::Result<Box<str>>> + Send {
        future::ready(self.0.read(path))
    }

    fn write(&self, path: &str, data: &str) -> impl Future<Output = io::Result<()>> + Send {
        future::ready(self.0.write(path, data))
    }

    fn rm(&self, path: &str) -> impl Future<Output = io::Result<()>> + Send {
        future::ready(self.0.rm(path))
    }
}

pub struct XsWindowsWatch {
    device: XsWindows,
    waitable: Waitable<OwnedHandle>,
    context: WatchContext,
    path: Box<str>,
}

impl Stream for XsWindowsWatch {
    type Item = Box<str>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(ready!(self.waitable.poll_ready(cx)).ok().map(|_| {
            unsafe {
                ResetEvent(HANDLE(self.waitable.get_ref().as_raw_handle()))
                    .inspect_err(|e| log::error!("Unable to reset event handle: {e}"))
                    .ok()
            };
            self.path.clone()
        }))
    }
}

impl Drop for XsWindowsWatch {
    fn drop(&mut self) {
        if let Err(e) = self.device.destroy_watch(self.context) {
            log::warn!("Unable to destroy watch object {e}")
        }
    }
}

impl AsyncWatch for XsSmolWindows {
    async fn watch(
        &self,
        path: &str,
    ) -> io::Result<impl Stream<Item = Box<str>> + Unpin + 'static> {
        // We want a clone of the device handle to be able to destroy the watch.
        let device = self.0.try_clone()?;
        let (event_handle, context) = self.0.make_watch(path)?;
        let waitable = Waitable::new(event_handle)?;

        Ok(XsWindowsWatch {
            device,
            context,
            waitable,
            path: path.into(),
        })
    }
}
