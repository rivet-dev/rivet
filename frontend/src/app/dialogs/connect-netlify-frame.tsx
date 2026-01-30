import { ReactNode } from "react";
import * as ConnectNetlifyForm from "@/app/forms/connect-netlify-form";
import { ConnectFrameStepperForm } from "./connect-manual-serverless-frame";

export function ConnectNetlifyFrame(): ReactNode {
	return (
		<ConnectFrameStepperForm
			stepper={ConnectNetlifyForm.stepper}
			renderForm={({ step }) => (
				<>
					{step.id === "api-route" && <ConnectNetlifyForm.IntegrationCode plan="" />}
					{step.id === "frontend" && <ConnectNetlifyForm.FrontendIntegrationCode />}
					{step.id === "variables" && <ConnectNetlifyForm.EnvVariables />}
					{step.id === "deploy" && (
						<>
							<ConnectNetlifyForm.Plan className="mb-4" />
							<ConnectNetlifyForm.RunnerName className="mb-4" />
							<ConnectNetlifyForm.Datacenters className="mb-4" />
							<ConnectNetlifyForm.MinRunners className="mb-4" />
							<ConnectNetlifyForm.MaxRunners className="mb-4" />
							<ConnectNetlifyForm.SlotsPerRunner className="mb-4" />
							<ConnectNetlifyForm.RunnerMargin className="mb-4" />
							<ConnectNetlifyForm.Headers className="mb-4" />
							<ConnectNetlifyForm.Endpoint className="mb-4" />
							<ConnectNetlifyForm.ConnectionCheck />
						</>
					)}
				</>
			)}
		/>
	);
}