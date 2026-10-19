// Expo inlines EXPO_PUBLIC_ variables only for literal property reads.
const values = {
  convexUrl: process.env.EXPO_PUBLIC_CONVEX_URL,
  workosClientId: process.env.EXPO_PUBLIC_WORKOS_CLIENT_ID,
  apiUrl: process.env.EXPO_PUBLIC_API_URL,
};

for (const [name, value] of Object.entries(values))
  if (!value) throw new Error(`Missing environment value: ${name}`);

export const env = values as { [K in keyof typeof values]: string };
