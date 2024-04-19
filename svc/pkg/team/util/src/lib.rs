use proto::backend;
use rivet_operation::prelude::*;

pub fn format_deactivate_reasons(reasons: &[i32]) -> GlobalResult<String> {
	Ok(reasons
		.iter()
		.map(|reason| {
			let reason_proto = unwrap!(backend::team::DeactivateReason::from_i32(*reason));

			let reason_str = match reason_proto {
				backend::team::DeactivateReason::Unknown => "Unknown",
				backend::team::DeactivateReason::NoPaymentMethod => "NoPaymentMethod",
				backend::team::DeactivateReason::PaymentFailed => "PaymentFailed",
				backend::team::DeactivateReason::Banned => "Banned",
			};

			Ok(reason_str)
		})
		.collect::<GlobalResult<Vec<_>>>()?
		.join(", "))
}
