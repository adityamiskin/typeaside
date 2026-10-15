import { defineApp } from "convex/server";
import dodopayments from "@dodopayments/convex/convex.config";
import rateLimiter from "@convex-dev/rate-limiter/convex.config";
const app = defineApp();
app.use(dodopayments);
app.use(rateLimiter);
export default app;
