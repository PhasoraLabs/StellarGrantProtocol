import { DataSource, Repository } from "typeorm";
import { Activity, ActivityType } from "../entities/Activity";
import { Grant } from "../entities/Grant";
import { ReconciliationCheckpoint } from "../entities/ReconciliationCheckpoint";
import { SorobanContractClient, SorobanContractEvent } from "../soroban/types";
import { logger } from "../config/logger";
import { GrantSyncService } from "./grant-sync-service";
import { metricsService } from "./metrics-service";

/** How many ledgers to scan per reconciliation run (≈ 30 min at 5 s/ledger = 360 ledgers). */
const MAX_LEDGER_RANGE = 400;

/** Activity event types that are sourced from on-chain events. */
const ON_CHAIN_ACTIVITY_TYPES = new Set<ActivityType>([
    "grant_created",
    "grant_updated",
    "grant_funded",
    "grant_completed",
    "milestone_submitted",
    "milestone_approved",
]);

export interface ReconciliationResult {
    fromLedger: number;
    toLedger: number;
    eventsFound: number;
    gapsFound: number;
    gapsFilled: number;
    errors: string[];
    durationMs: number;
}

export class ReconciliationService {
    private readonly activityRepo: Repository<Activity>;
    private readonly grantRepo: Repository<Grant>;
    private readonly checkpointRepo: Repository<ReconciliationCheckpoint>;
    private timer: ReturnType<typeof setInterval> | null = null;

    constructor(
        private readonly dataSource: DataSource,
        private readonly sorobanClient: SorobanContractClient,
        private readonly grantSyncService: GrantSyncService,
    ) {
        this.activityRepo = dataSource.getRepository(Activity);
        this.grantRepo = dataSource.getRepository(Grant);
        this.checkpointRepo = dataSource.getRepository(ReconciliationCheckpoint);
    }

    // ---------------------------------------------------------------------------
    // Scheduler
    // ---------------------------------------------------------------------------

    /** Start the periodic reconciliation task. Default: every 30 minutes. */
    start(intervalMs = 30 * 60 * 1000): void {
        if (this.timer) return;
        logger.info("ReconciliationService: scheduler started", { intervalMs });
        // Run once immediately, then on the interval.
        this.runSafe();
        this.timer = setInterval(() => this.runSafe(), intervalMs);
    }

    stop(): void {
        if (this.timer) {
            clearInterval(this.timer);
            this.timer = null;
            logger.info("ReconciliationService: scheduler stopped");
        }
    }

    // ---------------------------------------------------------------------------
    // Core reconciliation logic
    // ---------------------------------------------------------------------------

    /** Public entry point — safe wrapper that never throws. */
    async runSafe(): Promise<ReconciliationResult | null> {
        try {
            return await this.run();
        } catch (err) {
            logger.error("ReconciliationService: unhandled error during run", { err });
            metricsService.recordReconciliationRun("failure", 0);
            return null;
        }
    }

    /** Run one reconciliation pass and return a structured result. */
    async run(): Promise<ReconciliationResult> {
        const startTime = Date.now();
        const errors: string[] = [];

        const latestLedger = await this.sorobanClient.getLatestLedger();
        const checkpoint = await this.getOrCreateCheckpoint(latestLedger);
        const fromLedger = checkpoint.lastLedger + 1;
        const toLedger = Math.min(latestLedger, fromLedger + MAX_LEDGER_RANGE - 1);

        logger.info("ReconciliationService: starting run", { fromLedger, toLedger, latestLedger });

        if (fromLedger > toLedger) {
            logger.info("ReconciliationService: already up-to-date, nothing to reconcile");
            return { fromLedger, toLedger, eventsFound: 0, gapsFound: 0, gapsFilled: 0, errors, durationMs: Date.now() - startTime };
        }

        // 1. Fetch on-chain events for the ledger range.
        const events = await this.sorobanClient.fetchEvents(fromLedger, toLedger);
        logger.info("ReconciliationService: fetched on-chain events", { count: events.length });

        // 2. Identify gaps (events with no matching Activity row).
        const gaps = await this.findGaps(events);
        logger.info("ReconciliationService: gaps identified", { gapsFound: gaps.length });

        // 3. Fill each gap.
        let gapsFilled = 0;
        const writtenEventIds = new Set<string>();
        for (const event of gaps) {
            try {
                await this.fillGap(event, writtenEventIds);
                gapsFilled++;
            } catch (err: any) {
                const msg = `Failed to fill gap for event ${event.id}: ${err?.message ?? err}`;
                logger.warn("ReconciliationService: " + msg, { event });
                errors.push(msg);
            }
        }

        // 4. Advance the checkpoint to the last ledger we processed.
        await this.advanceCheckpoint(checkpoint, toLedger);

        const result: ReconciliationResult = {
            fromLedger,
            toLedger,
            eventsFound: events.length,
            gapsFound: gaps.length,
            gapsFilled,
            errors,
            durationMs: Date.now() - startTime,
        };

        metricsService.recordReconciliationRun(errors.length > 0 ? "failure" : "success", gaps.length);
        logger.info("ReconciliationService: run complete", result);
        return result;
    }

    // ---------------------------------------------------------------------------
    // Gap detection
    // ---------------------------------------------------------------------------

    /**
     * Returns the subset of on-chain events that have no corresponding Activity
     * record in the database. Uses the event's unique `id` stored in activity.data
     * to avoid duplicates.
     */
    private async findGaps(events: SorobanContractEvent[]): Promise<SorobanContractEvent[]> {
        if (events.length === 0) return [];

        const dedupedEvents = Array.from(new Map(events.map((event) => [event.id, event])).values());

        // We store the on-chain event id in activity.data.eventId for deduplication.
        // Use a raw query for efficiency when the list is large.
        const existing = await this.activityRepo
            .createQueryBuilder("a")
            .select("a.data")
            .where("a.data IS NOT NULL")
            .getMany();

        const knownEventIds = new Set(
            existing
                .filter((a) => ON_CHAIN_ACTIVITY_TYPES.has(a.type) && a.data?.eventId)
                .map((a) => a.data!.eventId as string),
        );

        return dedupedEvents.filter((e) => !knownEventIds.has(e.id));
    }

    // ---------------------------------------------------------------------------
    // Gap filling
    // ---------------------------------------------------------------------------

    private async fillGap(event: SorobanContractEvent, writtenEventIds: Set<string>): Promise<void> {
        // Ensure the grant exists in our DB (sync it if missing/stale).
        const grant = await this.grantRepo.findOne({ where: { id: event.grantId } });
        if (!grant) {
            logger.debug("ReconciliationService: syncing missing grant", { grantId: event.grantId });
            await this.grantSyncService.syncGrant(event.grantId);
        } else if (this.isGrantStateEvent(event.type)) {
            // For state-changing events, re-sync the grant to pick up the latest status.
            await this.grantSyncService.syncGrant(event.grantId);
        }

        // Write the missing Activity record.
        await this.writeActivity(event, writtenEventIds);
    }

    private isGrantStateEvent(type: string): boolean {
        return ["grant_updated", "grant_funded", "grant_completed", "milestone_approved"].includes(type);
    }

    private async writeActivity(event: SorobanContractEvent, writtenEventIds: Set<string>): Promise<void> {
        if (writtenEventIds.has(event.id)) {
            return;
        }

        const activity = new Activity();
        activity.type = this.mapEventType(event.type);
        activity.entityType = "grant";
        activity.entityId = event.grantId;
        activity.actorAddress = event.actorAddress;
        // Merge the original event data with our deduplication key.
        activity.data = { ...event.data, eventId: event.id, ledger: event.ledger, reconciled: true };
        await this.activityRepo.save(activity);
        writtenEventIds.add(event.id);
    }

    private mapEventType(onChainType: string): ActivityType {
        const map: Record<string, ActivityType> = {
            grant_created: "grant_created",
            grant_funded: "grant_funded",
            grant_updated: "grant_updated",
            grant_completed: "grant_completed",
            milestone_submitted: "milestone_submitted",
            milestone_approved: "milestone_approved",
        };
        return map[onChainType] ?? "grant_updated";
    }

    // ---------------------------------------------------------------------------
    // Checkpoint helpers
    // ---------------------------------------------------------------------------

    private async getOrCreateCheckpoint(latestLedger: number): Promise<ReconciliationCheckpoint> {
        let checkpoint = await this.checkpointRepo.findOne({ where: { name: "main" } });
        if (!checkpoint) {
            checkpoint = new ReconciliationCheckpoint();
            checkpoint.name = "main";
            // Start from MAX_LEDGER_RANGE ledgers back so the first run catches recent history.
            checkpoint.lastLedger = Math.max(0, latestLedger - MAX_LEDGER_RANGE);
            await this.checkpointRepo.save(checkpoint);
        }
        return checkpoint;
    }

    private async advanceCheckpoint(checkpoint: ReconciliationCheckpoint, toLedger: number): Promise<void> {
        checkpoint.lastLedger = toLedger;
        await this.checkpointRepo.save(checkpoint);
        logger.debug("ReconciliationService: checkpoint advanced", { lastLedger: toLedger });
    }
}
