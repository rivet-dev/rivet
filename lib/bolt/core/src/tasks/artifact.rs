use crate::context::{ProjectContext, ServiceContext};

pub async fn generate_project(_ctx: &ProjectContext) {
	// Nothing to do here
	//
	// Project-wide files that need to be generated after the service is built
	// can be created here.
}

pub async fn generate_all_services(ctx: &ProjectContext) {
	for svc_ctx in ctx.all_services().await {
		generate_service(&svc_ctx).await;
	}
}

pub async fn generate_service(_ctx: &ServiceContext) {
	// println!("  * Generating service {}", ctx.name());

	// Nothing to do here
	//
	// Service-specific files that need to be generated after the service is
	// built can be created here.
}
