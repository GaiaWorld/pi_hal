use std::env;

use pi_async::rt::multi_thread::{MultiTaskRuntime, StealableTaskPool, MultiTaskRuntimeBuilder};

lazy_static! {
    // 多媒体运行时，多线程，不需要主动推
    pub static ref MULTI_MEDIA_RUNTIME: MultiTaskRuntime<()> = {
        let count = match env::var("_ver") {
            Ok(r) => usize::from_str_radix(r.as_str(), 10).unwrap(),
            _ => num_cpus::get()
        };
        let pool = StealableTaskPool::with(count, count);
        // 线程池：每个线程1M的栈空间，10ms 休眠，10毫秒的定时器间隔
        let builder = MultiTaskRuntimeBuilder::new(pool).init_worker_size(count).set_worker_limit(count, count);
        builder.build()
    };

	// 渲染运行时，多线程，不需要主动推
    pub static ref RENDER_RUNTIME: MultiTaskRuntime<()> = {
        let count = match env::var("_ver") {
            Ok(r) => usize::from_str_radix(r.as_str(), 10).unwrap(),
            _ => num_cpus::get()
        };
        let pool = StealableTaskPool::with(count, count);
        // 线程池：每个线程1M的栈空间，10ms 休眠，10毫秒的定时器间隔
        let builder = MultiTaskRuntimeBuilder::new(pool).init_worker_size(count).set_worker_limit(count, count);
        builder.build()
    };
}

