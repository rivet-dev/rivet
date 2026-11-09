import { actor, setup } from "rivetkit";

interface WorkflowResult {
  workflowId: string;
  result: { finalized: boolean };
  completedAt: number;
}

const dataProcessor = actor({
  actions: {
    initialize: (c, workflowId: string) => ({ workflowId, data: "initialized" })
  }
});

const validator = actor({
  actions: {
    validate: (c, data: { workflowId: string; data: string }) => ({ valid: true, data })
  }
});

const finalizer = actor({
  actions: {
    finalize: (c, validationResult: { valid: boolean }) => ({ finalized: validationResult.valid })
  }
});

const workflowActor = actor({
  // Completed workflows are kept in memory for this example
  createVars: () => ({ workflows: [] as WorkflowResult[] }),

  actions: {
    executeWorkflow: async (c, workflowId: string) => {
      const client = c.client<typeof registry>();

      // Step 1: Initialize data
      const dataProcessorHandle = client.dataProcessor.getOrCreate(["main"]);
      const data = await dataProcessorHandle.initialize(workflowId);

      // Step 2: Process through multiple actors
      const validatorHandle = client.validator.getOrCreate(["main"]);
      const validationResult = await validatorHandle.validate(data);

      // Step 3: Finalize
      const finalizerHandle = client.finalizer.getOrCreate(["main"]);
      const result = await finalizerHandle.finalize(validationResult);

      c.vars.workflows.push({ workflowId, result, completedAt: Date.now() });
      return result;
    }
  }
});

const registry = setup({ use: { dataProcessor, validator, finalizer, workflowActor } });
