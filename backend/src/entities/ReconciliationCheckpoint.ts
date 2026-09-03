import { Column, Entity, PrimaryColumn, UpdateDateColumn } from "typeorm";

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
    @Column({ type: "bigint" })
    lastLedger!: number;

    @UpdateDateColumn()
    updatedAt!: Date;
}
