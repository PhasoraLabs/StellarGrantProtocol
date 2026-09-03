import { Repository } from "typeorm";
import { PlatformConfig } from "../entities/PlatformConfig";

export class ConfigService {
  constructor(private readonly configRepo: Repository<PlatformConfig>) {}

  async getFeePercentage(): Promise<number> {
    const config = await this.configRepo.findOne({ where: { key: "platform_fee_percentage" } });
    return config ? parseFloat(config.value) : 0;
  }

  async setFeePercentage(percentage: number): Promise<void> {
    let config = await this.configRepo.findOne({ where: { key: "platform_fee_percentage" } });
    if (!config) {
      config = this.configRepo.create({ key: "platform_fee_percentage" });
    }
    config.value = percentage.toString();
    await this.configRepo.save(config);
  }
}
