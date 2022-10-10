use pi_async::prelude::{AsyncRuntimeBuilder, WorkerRuntime, SingleTaskRuntime, SingleTaskRunner};
use pi_share::{Share, ShareMutex};
use std::sync::Arc;

lazy_static! {
	pub static ref RUNNER_MULTI: Arc<ShareMutex<SingleTaskRunner<()>>> = Arc::new(ShareMutex::new(SingleTaskRunner::default()));
	// 多媒体运行时，多线程，需要主动推
	pub static ref MULTI_MEDIA_RUNTIME: SingleTaskRuntime<()> = RUNNER_MULTI.lock().startup().unwrap();
   

	pub static ref RUNNER_RENDER: Arc<ShareMutex<SingleTaskRunner<()>>> = Arc::new(ShareMutex::new(SingleTaskRunner::default()));
	// 多媒体运行时，多线程，需要主动推
	pub static ref RENDER_RUNTIME: SingleTaskRuntime<()> = RUNNER_RENDER.lock().startup().unwrap();
}