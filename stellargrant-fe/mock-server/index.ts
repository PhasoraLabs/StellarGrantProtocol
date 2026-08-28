import express from "express";
import cors from "cors";
import grantsRouter from "./routes/grants";
import leaderboardRouter from "./routes/leaderboard";
import statsRouter from "./routes/stats";
import contributorsRouter from "./routes/contributors";
import milestonesRouter from "./routes/milestones";
import dashboardRouter from "./routes/dashboard";

const app = express();
const PORT = 4001;

// Configuration
const MOCK_DELAY = parseInt(process.env.MOCK_DELAY ?? '200');

// Middleware
app.use(cors({ origin: "http://localhost:3000" }));
app.use(express.json());

// Latency simulation
app.use((req, res, next) => {
  setTimeout(next, MOCK_DELAY);
});

// Routes
app.use("/grants", grantsRouter);
app.use("/leaderboard", leaderboardRouter);
app.use("/stats", statsRouter);
app.use("/contributors", contributorsRouter);
app.use("/milestones", milestonesRouter);
app.use("/dashboard", dashboardRouter);

// Root
app.get("/", (req, res) => {
  res.json({ message: "StellarGrant Mock API is running on :4001" });
});

app.listen(PORT, () => {
  console.log(`\x1b[32m%s\x1b[0m`, `🚀 Mock API server ready at http://localhost:${PORT}`);
  console.log(`\x1b[34m%s\x1b[0m`, `   Simulating ${MOCK_DELAY}ms latency`);
});
