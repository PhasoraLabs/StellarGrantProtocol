/**
 * Leaderboard Page
 *
 * Fully-populated contributor leaderboard ranked by reputation score.
 * Fetches paginated data from GET ${API_URL}/leaderboard with manual
 * fetch; pagination handled by the Pagination component.
 */

import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Leaderboard — StellarGrant Protocol",
  description:
    "Top contributors ranked by reputation score earned through completed milestones on the StellarGrant Protocol.",
};

export { default } from "./LeaderboardClient";
