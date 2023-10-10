use std::{sync::Arc, ops::Deref, marker::Sync};
use std::cell::OnceCell;

use pi_async_rt::rt::serial_local_compatible_wasm_runtime::{LocalTaskRuntime, LocalTaskRunner};
// use std::future::Future;
// use std::io::Result as IOResult;
// use pi_share::{Share, ShareMutex};

pub struct OnceCellWrap(pub OnceCell<LocalTaskRuntime<()>>);

unsafe impl Sync for OnceCellWrap {}

impl Deref for OnceCellWrap {
	type Target = LocalTaskRuntime<()>;

    // Required method
    fn deref(&self) -> &Self::Target {
		unsafe{ self.0.get().unwrap() }
	}
}

// impl OnceCellWrap {
// 	pub fn block_on<F>(&self, future: F) -> IOResult<F::Output>
// 	where
// 		F: Future + 'static,
// 		<F as Future>::Output: Default + 'static,
// 	{
// 		unsafe { self.0.get().unwrap().block_on(future)}
// 	}
// 	pub fn spawn<F>(&self, future: F) -> Result<(), std::io::Error>
// 	where
// 		F: Future<Output = ()> + 'static,
// 	{
// 		unsafe{self.0.get().unwrap().spawn(future)}
// 	}
// }

// 在外部初始化
pub static MULTI_MEDIA_RUNTIME: OnceCellWrap = OnceCellWrap(OnceCell::new());
pub static RENDER_RUNTIME: OnceCellWrap = OnceCellWrap(OnceCell::new());

// lazy_static! {
// 	pub static ref RUNNER_MULTI: Arc<ShareMutex<LocalTaskRunner<()>>> = Arc::new(ShareMutex::new(LocalTaskRunner::new()));
// 	// 多媒体运行时，多线程，需要主动推
// 	pub static ref MULTI_MEDIA_RUNTIME: LocalTaskRuntime<()> = RUNNER_MULTI.lock().get_runtime();
   

// 	pub static ref RUNNER_RENDER: Arc<ShareMutex<LocalTaskRunner<()>>> = Arc::new(ShareMutex::new(LocalTaskRunner::new()));
// 	// 多媒体运行时，多线程，需要主动推
	// pub static ref RENDER_RUNTIME: LocalTaskRuntime<()> = {
	// 	let mut runner = pi_async_rt::rt::serial_local_compatible_wasm_runtime::LocalTaskRunner::new();
	// 	runner.get_runtime()
	// };
	// RUNNER_RENDER.lock().get_runtime();
// }