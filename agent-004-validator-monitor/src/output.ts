import { config } from "./config.js";
import type { OutputMessage } from "./types.js";

/**
 * Output dispatcher.
 *
 * Sends agent outputs to configured channels:
 * - Console (always)
 * - Discord webhook (if configured)
 *
 * Mirrors the AGENT-002 and AGENT-003 dispatcher shape.
 */
export async function output(msg: OutputMessage): Promise<void> {
  logToConsole(msg);

  if (config.discordWebhookUrl) {
    await postToDiscord(msg).catch((err) =>
      console.error(`  Discord post failed: ${err}`)
    );
  }
}

function logToConsole(msg: OutputMessage): void {
  const prefix =
    msg.alertLevel === "CRITICAL"
      ? "!!! CRITICAL"
      : msg.alertLevel === "HIGH"
        ? "!! HIGH"
        : "--";

  console.log(`\n${"=".repeat(72)}`);
  console.log(
    `${prefix} [${msg.workflow}] ${msg.subjectId}: ${msg.title}`
  );
  console.log(`${"=".repeat(72)}`);
  console.log(msg.content);
  console.log(`${"=".repeat(72)}\n`);
}

async function postToDiscord(msg: OutputMessage): Promise<void> {
  const maxLen = 1900;
  let content = `**[${msg.workflow}]** ${msg.subjectId}: **${msg.title}**\n\n`;
  const remaining = maxLen - content.length;
  content +=
    msg.content.length > remaining
      ? msg.content.slice(0, remaining - 20) + "\n\n*...truncated*"
      : msg.content;

  await fetch(config.discordWebhookUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      username: config.agentName,
      content,
    }),
    signal: AbortSignal.timeout(10_000),
  });
}
