export interface PoolsUpsertRequest {
    namespace?: string;
    drainOnVersionUpgrade?: boolean;
    actorEvictionDelay?: number;
    actorEvictionPeriod?: number;
    actorEvictionRate?: number;
    metadata?: unknown;
}
