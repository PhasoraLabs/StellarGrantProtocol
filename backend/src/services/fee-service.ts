import { Repository } from "typeorm";
import { FeeCollection } from "../entities/FeeCollection";
import { PlatformConfig } from "../entities/PlatformConfig";

export class FeeService {
  constructor(
    private readonly feeRepo: Repository<FeeCollection>,
    private readonly configRepo: Repository<PlatformConfig>
  ) {}

  async calculateAndRecordFee(grantId: number, funderAddress: string, amount: string): Promise<string> {
    const config = await this.configRepo.findOne({ where: { key: "platform_fee_percentage" } });
    const percentage = config ? parseFloat(config.value) : 0;
    
    // Using BigInt for precision. Percentage is stored as a number (e.g. 2.5 for 2.5%)
    // fee = (amount * percentage) / 100
    // We multiply percentage by 100 to handle up to 2 decimal places in percentage.
    const feeAmount = (BigInt(amount) * BigInt(Math.floor(percentage * 100))) / BigInt(10000);

    await this.feeRepo.save({
      grantId,
      funderAddress,
      totalContribution: amount,
      feeAmount: feeAmount.toString(),
      feePercentage: percentage.toString(),
    });

    return feeAmount.toString();
  }

  async getTotalFeesCollected(): Promise<string> {
    const fees = await this.feeRepo.find();
    const total = fees.reduce((sum, f) => sum + BigInt(f.feeAmount), BigInt(0));
    return total.toString();
  }
}
