use pi_async::prelude::{AsyncRuntimeBuilder, WorkerRuntime};

lazy_static! {
    // 多媒体运行时，多线程，不需要主动推
    pub static ref MULTI_MEDIA_RUNTIME: WorkerRuntime<()> = AsyncRuntimeBuilder::default_worker_thread(
		None,
		None,
		None,
		None,
	);

	// 渲染运行时，多线程，不需要主动推
    pub static ref RENDER_RUNTIME: WorkerRuntime<()> = AsyncRuntimeBuilder::default_worker_thread(
		None,
		None,
		None,
		None,
	);
}
