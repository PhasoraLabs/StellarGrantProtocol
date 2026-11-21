import { Entity, PrimaryColumn, Column, CreateDateColumn, Index } from "typeorm";
import { timestampColumnType } from "../db/column-types";

/**
 * Consumed signature nonces for signed intents (milestone proofs, grant
 * feedback). Checked/persisted before accepting a signed payload so replay
 * protection does not depend on unrelated business-rule uniqueness (#1239).
 */
@Entity({ name: "used_nonces" })
@Index(["expiresAt"])
export class UsedNonce {
  @PrimaryColumn({ type: "varchar", length: 120 })
  nonce!: string;

  @CreateDateColumn()
  usedAt!: Date;

  /** Rows may be purged after this time; the nonce window is minutes. */
  @Column({ type: timestampColumnType })
  expiresAt!: Date;
}
