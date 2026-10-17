import { HOUR, RateLimiter } from "@convex-dev/rate-limiter";
import { components } from "./_generated/api";

export const rateLimiter = new RateLimiter(components.rateLimiter, {
  settings: { kind: "token bucket", rate: 60, period: HOUR, capacity: 60 },
  ping: { kind: "token bucket", rate: 30, period: HOUR, capacity: 30 },
  feedbackUser: { kind: "token bucket", rate: 30, period: HOUR, capacity: 30 },
  feedbackAnonymous: { kind: "token bucket", rate: 30, period: HOUR, capacity: 30 },
});
