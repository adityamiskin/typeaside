export function error(status: number, type: string, message: string, extra = {}) {
  return { status, error: { type, message, ...extra } };
}
