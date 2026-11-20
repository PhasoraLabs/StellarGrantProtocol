import { Column, CreateDateColumn, Entity, Index, PrimaryColumn } from "typeorm";

@Entity({ name: "user_watchlist" })
@Index("IDX_user_watchlist_address", ["address"])
@Index("IDX_user_watchlist_grant_id", ["grantId"])
@Index("IDX_user_watchlist_unique", ["address", "grantId"], { unique: true })
export class UserWatchlist {
  @PrimaryColumn({ type: "varchar", length: 120 })
  address!: string;

  @PrimaryColumn({ type: "int" })
  grantId!: number;

  @CreateDateColumn()
  createdAt!: Date;
}
