use std::env;
// use std::time::Instant;

// use pi_async_rt::{rt::multi_thread::{MultiTaskRuntime, StealableTaskPool, MultiTaskRuntimeBuilder}};
// use pi_share::ShareMutex;

#[cfg(not(feature = "single_thread"))]
lazy_static! {
	// pub static ref LOGS: ShareMutex<(Vec<String>, Instant)> = ShareMutex::new((Vec::new(), Instant::now()));

    // 多媒体运行时，多线程，不需要主动推
    pub static ref MULTI_MEDIA_RUNTIME: pi_async_rt::prelude::MultiTaskRuntime<()>  = {
        let count = match env::var("_ver") {
            Ok(r) => usize::from_str_radix(r.as_str(), 10).unwrap(),
            _ => num_cpus::get()
        };
        let pool = pi_async_rt::prelude::StealableTaskPool::with(count, 0x8000, [1, 1], 3000);
        // 线程池：每个线程1M的栈空间，10ms 休眠，10毫秒的定时器间隔
        let builder = pi_async_rt::prelude::MultiTaskRuntimeBuilder::new(pool).init_worker_size(count).set_worker_limit(count, count);
        builder.build()
    };

	// 渲染运行时，多线程，不需要主动推
    pub static ref RENDER_RUNTIME: pi_async_rt::prelude::MultiTaskRuntime<()>  = {
        let count = match env::var("_ver") {
            Ok(r) => usize::from_str_radix(r.as_str(), 10).unwrap(),
            _ => num_cpus::get()
        };
        let pool = pi_async_rt::prelude::StealableTaskPool::with(count, 0x8000, [1, 1], 3000);
        // 线程池：每个线程1M的栈空间，10ms 休眠，10毫秒的定时器间隔
        let builder = pi_async_rt::prelude::MultiTaskRuntimeBuilder::new(pool).init_worker_size(count).set_worker_limit(count, count);
        builder.build()
    };
}

// use pi_async_rt::prelude::SingleTaskRunner;


#[cfg(feature = "single_thread")]
lazy_static! {
	// pub static ref LOGS: ShareMutex<(Vec<String>, Instant)> = ShareMutex::new((Vec::new(), Instant::now()));

    // 多媒体运行时，多线程，不需要主动推
    pub static ref MULTI_MEDIA_RUNTIME: pi_async_rt::prelude::WorkerRuntime<()>  = {
		pi_async_rt::rt::AsyncRuntimeBuilder::default_worker_thread(Some("MULTI_MEDIA_RUNTIME"), None, None, None)
		// let runner = pi_async_rt::prelude::SingleTaskRunner::default();
		// runner.startup().unwrap()
    };

	// 渲染运行时，多线程，不需要主动推
    pub static ref RENDER_RUNTIME:  pi_async_rt::prelude::WorkerRuntime<()> = {
        pi_async_rt::rt::AsyncRuntimeBuilder::default_worker_thread(Some("RENDER_RUNTIME"), None, None, None)
    };
}


//     let runner = SingleTaskRunner::default();
//     let rt = runner.into_local();

//     rt

