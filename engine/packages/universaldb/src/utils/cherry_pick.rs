use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;

use crate::{
	options::StreamingMode,
	transaction::Transaction,
	tuple::{TuplePack, TupleUnpack},
	utils::{FormalKey, IsolationLevel, Subspace},
};

#[async_trait::async_trait]
pub trait CherryPick {
	type Output;

	async fn cherry_pick<S: TuplePack + Send>(
		tx: &Transaction,
		subspace: S,
		isolation_level: IsolationLevel,
	) -> Result<Self::Output>;
}

// Implements `CherryPick` for any tuple size
macro_rules! impl_tuple {
	($($args:ident),*) => {
		#[async_trait::async_trait]
		impl<$($args: FormalKey + for<'de> TupleUnpack<'de>),*> CherryPick for ($($args),*)
		where
	    	$($args::Value: Send),*
		{
	    	type Output = ($($args::Value),*);

	    	async fn cherry_pick<S: TuplePack + Send>(
	        	tx: &Transaction,
	        	subspace: S,
	        	isolation_level: IsolationLevel,
	    	) -> Result<Self::Output> {
	        	let tx = tx.with_subspace(Subspace::new(&subspace));

	        	let mut stream = tx.read_range(
	            	$crate::range_option::RangeOption {
	                	mode: StreamingMode::WantAll,
	                	..(&Subspace::all()).into()
	            	},
	            	isolation_level,
	        	);

	        	$(
					#[allow(non_snake_case)]
	            	let mut $args = None;
	        	)*

	        	loop {
	            	let Some(entry) = stream.try_next().await? else {
	                	break;
	            	};

	            	$(
	                	if let Ok(key) = tx.unpack::<$args>(entry.key()) {
	                    	ensure!($args.is_none(), "{} already picked", std::any::type_name::<$args>());

	                    	let value = key.deserialize(entry.value())?;
	                    	$args = Some(value);
	                    	continue;
	                	}
	            	)*
	        	}

	        	Ok((
	            	$(
	                	$args.with_context(|| {
	                    	format!("key not found in cherry pick: {}", std::any::type_name::<$args>())
	                	})?,
	            	)*
	        	))
	    	}
		}
	}
}

impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
