
use pi_assets::{asset::{Asset, Garbageer, Handle, GarbageEmpty}, mgr::LoadResult};
use pi_futures::BoxFuture;
pub trait AsyncLoader<'a, A: Asset, D: 'a, G: Garbageer<A> = GarbageEmpty> {
	fn async_load(desc: D, result: LoadResult<'a, A, G>) -> BoxFuture<'a, std::io::Result<Handle<A>>> ;
}

pub trait SyncLoader<'a, A: Asset,  D: 'a, G: Garbageer<A>> {
	fn sync_load(desc: D, result: LoadResult<'a, A, G>) -> std::io::Result<Handle<A>>;
}
