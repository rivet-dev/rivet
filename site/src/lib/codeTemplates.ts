import dedent from "dedent";

export const backendModule = ({ module, script }) =>
    dedent`
    const data = await ctx.modules.${module}.${script}({
      // Request body
    });
`;

export const frontendModule = ({ module, script }) =>
    dedent`
        import { Backend } from "rivet-sdk";
        
        const backend = new Backend({ 
            endpoint: "http://localhost:6420"
        });
        const data = await backend.${module}.${script}({
            // Request body
        });
`;

export const godotModule = ({ module, script }) =>
    dedent`
        extends Node

        func _ready():
            backend.${module}.${script}({
                # Request body
            })
    `;

export const unityModule = ({ module, script }) =>
    dedent`
        using Rivet;

        public class MyScript : MonoBehaviour
        {
            async void Start()
            {
                var config = new Configuration();
                var client = new BackendClient(config.BackendEndpoint);
                var data = await client.${module}.${script}({
                    // Request body
                });
            }
        }
    `;

export const unrealModule = ({ module, script }) =>
    dedent`
        // IMPORTANT: Auto-generated SDK coming soon
        #include "HttpModule.h"
        #include "Interfaces/IHttpRequest.h"
        #include "Interfaces/IHttpResponse.h"
        #include "Misc/DefaultValueHelper.h"

        void UYourClassName::PostRequest()
        {
            TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
            Request->OnProcessRequestComplete().BindUObject(this, &UYourClassName::OnResponseReceived);
            Request->SetURL(TEXT("https://localhost:6420/modules/lobbies/scripts/create/call"));
            Request->SetVerb(TEXT("POST"));
            Request->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
            FString Payload = TEXT("{"key":"value"}");
            Request->SetContentAsString(Payload);
            Request->ProcessRequest();
        }

        void UYourClassName::OnResponseReceived(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bWasSuccessful)
        {
            if(!bWasSuccessful || !Response.IsValid())
            {
                UE_LOG(LogTemp, Error, TEXT("Request Failed"));
                return
            }

            FString data = Response->GetContentAsString();

            // Success
        }
    `;

export const bashModule = ({ module, script }) =>
    dedent`
        curl -X POST -H "Content-Type: application/json" -d '{"key":"value"}' https://localhost:6420/modules/${module}/scripts/${script}/call`;
