use async_trait::async_trait;
use pi_assets::{asset::{Asset, Garbageer, Handle, GarbageEmpty}, mgr::LoadResult};

#[async_trait]
pub trait AsyncLoader<'a, A: Asset, D: 'a, G: Garbageer<A> = GarbageEmpty> {
	async fn async_load(desc: D, result: LoadResult<'a, A, G>) -> std::io::Result<Handle<A>>;
}

pub trait SyncLoader<'a, A: Asset,  D: 'a, G: Garbageer<A>> {
	fn sync_load(desc: D, result: LoadResult<'a, A, G>) -> std::io::Result<Handle<A>>;
}
