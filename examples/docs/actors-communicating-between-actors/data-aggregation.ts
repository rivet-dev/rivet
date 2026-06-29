import { actor, setup } from "rivetkit";

interface Stats {
  count: number;
  total: number;
}

interface Report {
  id: string;
  type: string;
  data: { users: Stats; orders: Stats; system: Stats };
  generatedAt: number;
}

const userMetrics = actor({
  state: {},
  actions: {
    getStats: (c): Stats => ({ count: 100, total: 500 })
  }
});

const orderMetrics = actor({
  state: {},
  actions: {
    getStats: (c): Stats => ({ count: 50, total: 10000 })
  }
});

const systemMetrics = actor({
  state: {},
  actions: {
    getStats: (c): Stats => ({ count: 5, total: 99 })
  }
});

const analyticsActor = actor({
  state: { reports: [] as Report[] },

  actions: {
    generateReport: async (c, reportType: string) => {
      const client = c.client<typeof registry>();

      // Collect data from multiple sources
      const userMetricsHandle = client.userMetrics.getOrCreate(["main"]);
      const orderMetricsHandle = client.orderMetrics.getOrCreate(["main"]);
      const systemMetricsHandle = client.systemMetrics.getOrCreate(["main"]);

      const [users, orders, system] = await Promise.all([
        userMetricsHandle.getStats(),
        orderMetricsHandle.getStats(),
        systemMetricsHandle.getStats()
      ]);

      const report: Report = {
        id: crypto.randomUUID(),
        type: reportType,
        data: { users, orders, system },
        generatedAt: Date.now()
      };

      c.state.reports.push(report);
      return report;
    }
  }
});

const registry = setup({ use: { userMetrics, orderMetrics, systemMetrics, analyticsActor } });
