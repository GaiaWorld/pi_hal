
use pi_assets::{asset::{Asset, Garbageer, Handle, GarbageEmpty}, mgr::LoadResult};
use pi_futures::BoxFuture;

/// 异步资源加载器trait
/// 
/// # 泛型参数
/// - `A`: 资源类型，需实现Asset trait
/// - `D`: 资源描述符类型
/// - `G`: 资源垃圾回收器，默认为GarbageEmpty
pub trait AsyncLoader<'a, A: Asset, D: 'a, G: Garbageer<A> = GarbageEmpty> {
	/// 异步加载资源
    ///
    /// # 参数
    /// - `desc`: 资源描述符，用于标识要加载的资源
    /// - `result`: 加载结果容器，包含资源加载状态和资源管理器
    ///
    /// # 返回
    /// 返回BoxFuture包装的IO Result，成功时包含资源句柄
	fn async_load(desc: D, result: LoadResult<'a, A, G>) -> BoxFuture<'a, std::io::Result<Handle<A>>> ;
}
/// 同步资源加载器trait
///
/// # 泛型参数  
/// - `A`: 资源类型，需实现Asset trait
/// - `D`: 资源描述符类型
/// - `G`: 资源垃圾回收器类型
pub trait SyncLoader<'a, A: Asset,  D: 'a, G: Garbageer<A>> {
	/// 同步加载资源
    ///
    /// # 参数
    /// - `desc`: 资源描述符，用于标识要加载的资源
    /// - `result`: 加载结果容器，包含资源加载状态和资源管理器
    ///
    /// # 返回
    /// 返回IO Result，成功时包含资源句柄
	fn sync_load(desc: D, result: LoadResult<'a, A, G>) -> std::io::Result<Handle<A>>;
}
