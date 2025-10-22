import { TemplateContext } from '../../context';

export function generateRunner(context: TemplateContext) {
  // The runner files are shared and located at sdks/typescript/test-runner/
  // We just need to reference them in the docker-compose
  // No specific files need to be generated here for now
}
