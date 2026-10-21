const SECONDS_PER_MINUTE = 60

/** The local time zone, in seconds ahead of UTC: the Rust core words every time with it. */
export const readUtcOffsetSeconds = (): number => -new Date().getTimezoneOffset() * SECONDS_PER_MINUTE
