/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as billing from "../billing.js";
import type * as desktop from "../desktop.js";
import type * as feedback from "../feedback.js";
import type * as http from "../http.js";
import type * as lib from "../lib.js";
import type * as pings from "../pings.js";
import type * as proxy from "../proxy.js";
import type * as rateLimits from "../rateLimits.js";
import type * as rollups from "../rollups.js";
import type * as settings from "../settings.js";
import type * as stats from "../stats.js";
import type * as viewer from "../viewer.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";

declare const fullApi: ApiFromModules<{
  billing: typeof billing;
  desktop: typeof desktop;
  feedback: typeof feedback;
  http: typeof http;
  lib: typeof lib;
  pings: typeof pings;
  proxy: typeof proxy;
  rateLimits: typeof rateLimits;
  rollups: typeof rollups;
  settings: typeof settings;
  stats: typeof stats;
  viewer: typeof viewer;
}>;

/**
 * A utility for referencing Convex functions in your app's public API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
export declare const api: FilterApi<
  typeof fullApi,
  FunctionReference<any, "public">
>;

/**
 * A utility for referencing Convex functions in your app's internal API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = internal.myModule.myFunction;
 * ```
 */
export declare const internal: FilterApi<
  typeof fullApi,
  FunctionReference<any, "internal">
>;

export declare const components: {
  dodopayments: import("@dodopayments/convex/_generated/component.js").ComponentApi<"dodopayments">;
  rateLimiter: import("@convex-dev/rate-limiter/_generated/component.js").ComponentApi<"rateLimiter">;
};
