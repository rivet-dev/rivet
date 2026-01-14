import { defineConfig, loadEnv, mergeConfig } from "vite";
import { cloudEnvSchema } from "./src/lib/env";
import engineConfig, { liveChatPlugin } from "./vite.engine.config";

// https://vitejs.dev/config/
export default defineConfig((config) => {
	const env = loadEnv(config.mode, process.cwd(), "");
	cloudEnvSchema.parse(env);
	return mergeConfig(
		engineConfig(config),
		defineConfig({
			base: "/",
			plugins: [
				{
					...liveChatPlugin(`<script>
(function(d, script) {
  script = d.createElement('script');
  script.async = false;
  script.onload = function(){
    Plain.init({
      requireAuthentication: true,
      appId: 'liveChatApp_01K5D3WHR3CGKA56RPRMBB7FX0',
	  hideLauncher: true,
	  theme: 'dark',
	  primaryColor: '#FF4F00',
    });
  };
  script.src = 'https://chat.cdn-plain.com/index.js';
  d.getElementsByTagName('head')[0].appendChild(script);
}(document));
</script>`),
					enforce: "pre",
				},
			],
			define: {
				__APP_TYPE__: JSON.stringify("cloud"),
			},
			server: {
				port: 43710,
			},
			preview: {
				port: 43710,
			},
		}),
		true,
	);
});
