import {
  Column,
  Entity,
  PrimaryColumn,
  UpdateDateColumn,
  ValueTransformer,
} from "typeorm";

/**
 * Persists the last successfully processed ledger sequence for the
 * reconciliation task so each run only scans the delta since the last run.
 */
@Entity({ name: "reconciliation_checkpoints" })
export class ReconciliationCheckpoint {
    /** Logical name for the checkpoint, e.g. "main" */
    @PrimaryColumn({ type: "varchar", length: 50 })
    name!: string;

    /** Last ledger sequence that was fully reconciled */
    /** Value transformer to convert bigint string to number on read */
    @Column({ type: "bigint", transformer: { from: (v: string | number) => (typeof v === "string" ? Number(v) : v), to: (v: number) => String(v) } })
    lastLedger!: number;

    @UpdateDateColumn()
    updatedAt!: Date;
}