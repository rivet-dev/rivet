import { actor, UserError } from "rivetkit";

interface ConnParams {
  apiKey: string;
}

interface ConnState {
  userId: string;
  tier: string;
}

const apiActor = actor({
  state: {},

  createConnState: async (c, params: ConnParams): Promise<ConnState> => {
    const response = await fetch(`https://api.my-auth-provider.com/validate`, {
      method: "POST",
      headers: { "X-API-Key": params.apiKey },
    });

    if (!response.ok) {
      throw new UserError("Invalid API key", { code: "invalid_api_key" });
    }

    const data = await response.json();
    return { userId: data.id, tier: data.tier };
  },

  actions: {
    premiumAction: (c) => {
      if (c.conn.state.tier !== "premium") {
        throw new UserError("Premium subscription required", { code: "forbidden" });
      }
      return "Premium content";
    },
  },
});
