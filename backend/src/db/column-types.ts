import { ColumnType } from "typeorm";

// The SQL.js test driver uses SQLite's datetime type; PostgreSQL uses timestamp.
// Passing Date resolves to "timestamp without time zone" in PostgreSQL and "datetime" in SQLite (SQL.js).
export const timestampColumnType: ColumnType = Date;
