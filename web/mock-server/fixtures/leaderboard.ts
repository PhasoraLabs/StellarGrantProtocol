export const leaderboard = Array.from({ length: 25 }, (_, i) => ({
  rank: i + 1,
  address: `GD...USER${i + 1}`,
  reputation: 1000 - i * 15,
  grantsCompleted: 10 - Math.floor(i / 3),
  totalEarned: 50000 - i * 1200,
  avatar: `https://api.dicebear.com/7.x/avataaars/svg?seed=user${i + 1}`
}));
