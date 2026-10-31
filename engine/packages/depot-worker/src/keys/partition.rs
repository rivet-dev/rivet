//! Volume addresses of logs the tablet maintainer writes itself.

use rivet_keys::keys::codec::Placement;

// One tablet's maintainer log: a reserved volume placed at the start of the tablet's hash
// range, so the tablet's replica set holds it and a listing of the tablet finds it. The
// tablet id keeps the logs of a split's parent and children apart, and the placement is a
// component as well as the placement so the address unpacks without the tablet.
rivet_keys::volume_key! { pub MaintainerLogVolume(tablet_id: [u8; 16], placement: [u8; 16]) placed_by(maintainer_log_placement) => [MAINTAINER_LOG, {tablet_id}, {placement}] name [MAINTAINER_LOG]; }

fn maintainer_log_placement(volume: &MaintainerLogVolume) -> Placement {
	volume.placement
}
