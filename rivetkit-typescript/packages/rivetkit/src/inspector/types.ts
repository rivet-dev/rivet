
export type DatabaseColumn = {
    cid: number;
    name: string;
    type: string;
    notnull: boolean;
    dflt_value: string | null;
    pk: boolean | null;
};

export type DatabaseForeignKey = {
    id: number;
    table: string;
    from: string;
    to: string;
};

export type DatabaseTableInfo = {
    table: { schema: string; name: string; type: string };
    columns: DatabaseColumn[];
    foreignKeys: DatabaseForeignKey[];
    records: number;
};

export type DatabaseSchema = {
    tables: DatabaseTableInfo[];
};