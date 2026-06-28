import { actor, UserError } from "rivetkit";
import { z } from "zod";

// Define schema for action parameters
const IncrementSchema = z.object({
  count: z.number().int().positive()
});

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, params: unknown) => {
      // Validate parameters
      const result = IncrementSchema.safeParse(params);
      if (!result.success) {
        throw new UserError("Invalid parameters", {
          code: "invalid_params",
          metadata: { errors: result.error.issues }
        });
      }
      c.state.count += result.data.count;
      return c.state.count;
    }
  }
});
