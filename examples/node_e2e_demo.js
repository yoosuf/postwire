#!/usr/bin/env node
/**
 * End-to-end demo using Node.js native fetch (Node 18+) to interact with Postwire REST API.
 * Demonstrates long-polling wait, signal extraction (OTP/links), and cleanup for Email & SMS.
 *
 * Usage:
 *   node examples/node_e2e_demo.js [POSTWIRE_URL]
 */

const BASE_URL = process.argv[2] || process.env.POSTWIRE_URL || "http://127.0.0.1:8025";

async function main() {
  console.log(`Connecting to Postwire server at ${BASE_URL}...\n`);

  // --- 1. EMAIL E2E FLOW ---
  const emailRecipient = "node-agent@example.com";
  // Always capture timestamp BEFORE triggering the email action
  const emailSince = new Date().toISOString();

  console.log("--- EMAIL FLOW ---");
  console.log(`1. Injecting test email to ${emailRecipient}...`);
  const injectEmailRes = await fetch(`${BASE_URL}/api/test-email`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ to: emailRecipient }),
  });
  const injectedEmail = await injectEmailRes.json();
  console.log(`   Injected email ID: ${injectedEmail.id}`);

  console.log("2. Long-polling /api/wait for incoming email...");
  const waitEmailParams = new URLSearchParams({
    to: emailRecipient,
    since: emailSince,
    timeout_ms: "10000",
  });
  const waitEmailRes = await fetch(`${BASE_URL}/api/wait?${waitEmailParams}`);
  const emailDetail = await waitEmailRes.json();
  console.log(`   Received: "${emailDetail.subject}" from ${emailDetail.from_addr}`);

  console.log("3. Extracting OTP codes and links from email...");
  const extractEmailRes = await fetch(`${BASE_URL}/api/messages/${emailDetail.id}/extract`);
  const emailSignals = await extractEmailRes.json();
  console.log(`   Codes: ${JSON.stringify(emailSignals.codes)} | Links: ${JSON.stringify(emailSignals.links)}`);

  console.log("4. Cleaning up email...");
  await fetch(`${BASE_URL}/api/messages/${emailDetail.id}`, { method: "DELETE" });
  console.log("   Email deleted.");

  // --- 2. SMS E2E FLOW ---
  const smsTo = "+15550199";
  const smsFrom = "+18005550123";
  const smsBody = "Your verification code is 849201.";
  const smsSince = new Date().toISOString();

  console.log("\n--- SMS FLOW ---");
  console.log(`5. Ingesting test SMS via POST /api/sms to ${smsTo}...`);
  const injectSmsRes = await fetch(`${BASE_URL}/api/sms`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ from: smsFrom, to: smsTo, body: smsBody }),
  });
  const injectedSms = await injectSmsRes.json();
  console.log(`   Ingested SMS ID: ${injectedSms.id}`);

  console.log("6. Long-polling /api/sms/wait for incoming SMS...");
  const waitSmsParams = new URLSearchParams({
    to: smsTo,
    since: smsSince,
    timeout_ms: "10000",
  });
  const waitSmsRes = await fetch(`${BASE_URL}/api/sms/wait?${waitSmsParams}`);
  const smsDetail = await waitSmsRes.json();
  console.log(`   Received SMS from ${smsDetail.from_phone}: "${smsDetail.body}"`);

  console.log("7. Extracting OTP codes from SMS...");
  const extractSmsRes = await fetch(`${BASE_URL}/api/sms/${smsDetail.id}/extract`);
  const smsSignals = await extractSmsRes.json();
  console.log(`   Extracted SMS Codes: ${JSON.stringify(smsSignals.codes)}`);

  console.log("8. Cleaning up SMS...");
  await fetch(`${BASE_URL}/api/sms/${smsDetail.id}`, { method: "DELETE" });
  console.log("   SMS deleted. Demo complete!");
}

main().catch((err) => {
  console.error("Demo failed:", err);
  process.exit(1);
});
