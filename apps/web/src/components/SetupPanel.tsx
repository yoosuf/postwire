import { memo, useEffect, useMemo, useState } from "react";
import { api } from "../api";
import { useInboxStore } from "../store";
import { btn, card, input } from "../ui";
import { CheckIcon, CloseIcon, CopyIcon, SendIcon } from "./Icons";
import type { ServerConfig } from "../types";

const CopyButton = memo(function CopyButton({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(value).catch(() => {});
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
      className="inline-flex shrink-0 items-center gap-1 rounded-md border border-zinc-800 bg-zinc-900/80 px-2 py-1 text-xs font-medium text-zinc-400 transition-all hover:border-zinc-700 hover:bg-zinc-800 hover:text-zinc-100 active:scale-95"
    >
      {copied ? <CheckIcon className="text-emerald-400" /> : <CopyIcon />}
      <span className={copied ? "text-emerald-400" : ""}>{copied ? "Copied" : "Copy"}</span>
    </button>
  );
});

const ConnectionField = memo(function ConnectionField({
  label,
  value,
  method,
}: {
  label: string;
  value: string;
  method?: string;
}) {
  return (
    <div className="group flex items-center justify-between gap-3 rounded-lg border border-zinc-800/80 bg-zinc-900/40 p-3 transition-colors hover:border-zinc-700/80 hover:bg-zinc-900/70">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          {method && (
            <span className="rounded bg-indigo-500/15 px-1.5 py-0.5 text-[10px] font-semibold text-indigo-400">
              {method}
            </span>
          )}
          <span className="text-[11px] font-medium uppercase tracking-wider text-zinc-500">{label}</span>
        </div>
        <div className="mt-0.5 truncate font-mono text-xs text-zinc-100 select-all">{value}</div>
      </div>
      <CopyButton value={value} />
    </div>
  );
});

type Snippet = "node" | "python" | "env" | "curl";
type VendorSnippet = "resend" | "sendgrid" | "postmark";
type SmsSnippet = "json" | "twilio" | "python" | "node";
type WebhookSnippet = "slack" | "discord" | "generic";

function snippetFor(kind: Snippet, host: string, port: number): string {
  switch (kind) {
    case "node":
      return `import nodemailer from "nodemailer";

const transport = nodemailer.createTransport({
  host: "${host}",
  port: ${port},
  secure: false,
});

await transport.sendMail({
  from: "you@yourapp.dev",
  to: "user@example.com",
  subject: "Hello",
  text: "Sent via Postwire",
});`;
    case "python":
      return `import smtplib
from email.mime.text import MIMEText

msg = MIMEText("Sent via Postwire")
msg["Subject"] = "Hello"
msg["From"] = "you@yourapp.dev"
msg["To"] = "user@example.com"

with smtplib.SMTP("${host}", ${port}) as s:
    s.send_message(msg)`;
    case "env":
      return `SMTP_HOST=${host}
SMTP_PORT=${port}
SMTP_SECURE=false
SMTP_USER=
SMTP_PASSWORD=`;
    case "curl":
      return `curl --url 'smtp://${host}:${port}' \\
  --mail-from 'you@yourapp.dev' \\
  --mail-rcpt 'user@example.com' \\
  --upload-file - <<EOF
From: you@yourapp.dev
To: user@example.com
Subject: Test Email

Hello from Postwire cURL!
EOF`;
  }
}

function vendorSnippetFor(kind: VendorSnippet, host: string): string {
  switch (kind) {
    case "resend":
      return `// Resend API format (POST http://${host}:8025/emails)
await fetch("http://${host}:8025/emails", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    from: "onboarding@resend.dev",
    to: ["user@example.com"],
    subject: "Hello World",
    html: "<p>Your code is 482913</p>"
  })
});`;
    case "sendgrid":
      return `// SendGrid API format (POST http://${host}:8025/v3/mail/send)
await fetch("http://${host}:8025/v3/mail/send", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    personalizations: [{ to: [{ email: "user@example.com" }] }],
    from: { email: "sender@example.com" },
    subject: "SendGrid Test",
    content: [{ type: "text/html", value: "<p>Code: 482913</p>" }]
  })
});`;
    case "postmark":
      return `// Postmark API format (POST http://${host}:8025/email)
await fetch("http://${host}:8025/email", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    From: "sender@example.com",
    To: "user@example.com",
    Subject: "Postmark Test",
    HtmlBody: "<p>Code: 482913</p>"
  })
});`;
  }
}

function smsSnippetFor(kind: SmsSnippet, host: string): string {
  switch (kind) {
    case "json":
      return `curl -X POST http://${host}:8025/api/sms \\
  -H "Content-Type: application/json" \\
  -d '{
    "from": "+18005550199",
    "to": "+15550100",
    "body": "Your verification code is 849201"
  }'`;
    case "twilio":
      return `curl -X POST http://${host}:8025/api/sms/webhook \\
  -H "Content-Type: application/x-www-form-urlencoded" \\
  --data-urlencode "From=+18005550199" \\
  --data-urlencode "To=+15550100" \\
  --data-urlencode "Body=Your verification code is 849201"`;
    case "python":
      return `import requests

response = requests.post(
    "http://${host}:8025/api/sms",
    json={
        "from": "+18005550199",
        "to": "+15550100",
        "body": "Your verification code is 849201"
    }
)
print(response.json())`;
    case "node":
      return `await fetch("http://${host}:8025/api/sms", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    from: "+18005550199",
    to: "+15550100",
    body: "Your verification code is 849201",
  }),
});`;
  }
}

function webhookSnippetFor(kind: WebhookSnippet, host: string): string {
  switch (kind) {
    case "slack":
      return `curl -X POST http://${host}:8025/api/webhooks/slack \\
  -H "Content-Type: application/json" \\
  -d '{"text": "Alert: Verification code is 884422", "username": "AlertBot"}'`;
    case "discord":
      return `curl -X POST http://${host}:8025/api/webhooks/discord \\
  -H "Content-Type: application/json" \\
  -d '{"content": "Alert: OTP code is 332211", "username": "DiscordBot"}'`;
    case "generic":
      return `curl -X POST http://${host}:8025/api/webhooks/generic \\
  -H "Content-Type: application/json" \\
  -d '{"from": "auth-service", "to": "alerts", "body": "Custom notification payload 991122"}'`;
  }
}

const SNIPPET_TABS: [Snippet, string][] = [
  ["node", "Node.js"],
  ["python", "Python"],
  ["env", ".env"],
  ["curl", "cURL"],
];

const VENDOR_SNIPPET_TABS: [VendorSnippet, string][] = [
  ["resend", "Resend API"],
  ["sendgrid", "SendGrid API"],
  ["postmark", "Postmark API"],
];

const SMS_SNIPPET_TABS: [SmsSnippet, string][] = [
  ["json", "cURL (JSON)"],
  ["twilio", "cURL (Twilio)"],
  ["python", "Python"],
  ["node", "Node.js"],
];

const WEBHOOK_SNIPPET_TABS: [WebhookSnippet, string][] = [
  ["slack", "Slack Webhook"],
  ["discord", "Discord Webhook"],
  ["generic", "Generic JSON"],
];

export const SetupPanel = memo(function SetupPanel({ onClose }: { onClose?: () => void }) {
  const setupOpen = useInboxStore((s) => s.setupOpen);
  const setSetupOpen = useInboxStore((s) => s.setSetupOpen);
  const activeSegment = useInboxStore((s) => s.setupSegment);
  const setActiveSegment = useInboxStore((s) => s.setSetupSegment);

  const handleClose = () => {
    setSetupOpen(false);
    onClose?.();
  };

  const [config, setConfig] = useState<ServerConfig | null>(null);
  const [snippet, setSnippet] = useState<Snippet>("node");
  const [vendorSnippet, setVendorSnippet] = useState<VendorSnippet>("resend");
  const [smsSnippet, setSmsSnippet] = useState<SmsSnippet>("json");
  const [webhookSnippet, setWebhookSnippet] = useState<WebhookSnippet>("slack");

  const [testTo, setTestTo] = useState("");
  const [sendState, setSendState] = useState<"idle" | "sending" | "sent" | "error">("idle");

  const [smsTestTo, setSmsTestTo] = useState("");
  const [smsSendState, setSmsSendState] = useState<"idle" | "sending" | "sent" | "error">("idle");

  useEffect(() => {
    if (!setupOpen) return;
    api.config().then(setConfig).catch(() => {});
  }, [setupOpen]);

  const host = window.location.hostname || "localhost";
  const port = config?.smtp_port ?? 1025;

  const currentEmailSnippet = useMemo(() => snippetFor(snippet, host, port), [snippet, host, port]);
  const currentVendorSnippet = useMemo(() => vendorSnippetFor(vendorSnippet, host), [vendorSnippet, host]);
  const currentSmsSnippet = useMemo(() => smsSnippetFor(smsSnippet, host), [smsSnippet, host]);
  const currentWebhookSnippet = useMemo(() => webhookSnippetFor(webhookSnippet, host), [webhookSnippet, host]);

  async function handleSendTest() {
    setSendState("sending");
    try {
      await api.sendTestEmail(testTo);
      setSendState("sent");
      setTimeout(() => setSendState("idle"), 2500);
    } catch {
      setSendState("error");
    }
  }

  async function handleSendSmsTest() {
    setSmsSendState("sending");
    try {
      await api.sendTestSms(smsTestTo || undefined);
      setSmsSendState("sent");
      setTimeout(() => setSmsSendState("idle"), 2500);
    } catch {
      setSmsSendState("error");
    }
  }

  if (!setupOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-md transition-opacity animate-in fade-in duration-200"
      onClick={handleClose}
    >
      <div
        className="flex h-full w-full max-w-lg flex-col overflow-y-auto border-l border-zinc-800/80 bg-zinc-950/95 shadow-2xl backdrop-blur-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-zinc-800/80 px-6 py-4">
          <div className="flex items-center gap-3">
            <h2 className="text-base font-semibold tracking-tight text-zinc-100">Connect your app</h2>
            <div className="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-medium text-emerald-400">
              <span className="relative flex h-2 w-2">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75"></span>
                <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-500"></span>
              </span>
              Connected
            </div>
          </div>
          <button
            onClick={handleClose}
            className={`${btn.ghost} rounded-full p-1.5 hover:bg-zinc-800`}
            aria-label="Close setup panel"
          >
            <CloseIcon />
          </button>
        </div>

        {/* Segmented Navigation Control */}
        <div className="px-6 pt-5">
          <div className="flex rounded-xl border border-zinc-800/80 bg-zinc-900/60 p-1 shadow-inner">
            <button
              onClick={() => setActiveSegment("email")}
              className={`flex flex-1 items-center justify-center gap-2 rounded-lg py-2 text-xs font-semibold transition-all duration-150 ${
                activeSegment === "email"
                  ? "border border-indigo-500/30 bg-zinc-800 text-zinc-100 shadow-md shadow-indigo-950/40"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              <span className="text-sm">✉️</span>
              <span>Email (SMTP)</span>
            </button>
            <button
              onClick={() => setActiveSegment("sms")}
              className={`flex flex-1 items-center justify-center gap-2 rounded-lg py-2 text-xs font-semibold transition-all duration-150 ${
                activeSegment === "sms"
                  ? "border border-emerald-500/30 bg-zinc-800 text-zinc-100 shadow-md shadow-emerald-950/40"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              <span className="text-sm">💬</span>
              <span>SMS API & Webhooks</span>
            </button>
          </div>
        </div>

        {/* Panel Content */}
        <div className="flex-1 space-y-6 p-6">
          {activeSegment === "email" ? (
            <>
              {/* SMTP Connection details */}
              <section className="space-y-3">
                <div className="flex items-center justify-between">
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                    SMTP Server Configuration
                  </h3>
                  <span className="text-[11px] text-zinc-500">No TLS / No Auth</span>
                </div>
                <div className="grid gap-2">
                  <ConnectionField label="Host" value={host} />
                  <ConnectionField label="SMTP Port" value={String(port)} />
                  <ConnectionField label="Authentication" value="None (dev environment)" />
                </div>
              </section>

              {/* Mailer Code Snippets */}
              <section className="space-y-3">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                  Integration Code Examples
                </h3>
                <div className={card}>
                  <div className="flex items-center gap-1 border-b border-zinc-800/80 bg-zinc-900/60 px-3 pt-2">
                    {SNIPPET_TABS.map(([key, label]) => (
                      <button
                        key={key}
                        onClick={() => setSnippet(key)}
                        className={`rounded-t-lg px-3 py-1.5 text-xs font-medium transition-all ${
                          snippet === key
                            ? "border-b-2 border-indigo-500 bg-zinc-800/50 text-zinc-100"
                            : "text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                    <div className="ml-auto pb-1">
                      <CopyButton value={currentEmailSnippet} />
                    </div>
                  </div>
                  <div className="relative">
                    <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed text-zinc-300">
                      {currentEmailSnippet}
                    </pre>
                  </div>
                </div>
              </section>

              {/* Vendor Email API Emulators Snippets */}
              <section className="space-y-3">
                <div className="flex items-center justify-between">
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                    Vendor Email API Emulators
                  </h3>
                  <span className="text-[11px] font-mono text-indigo-400">HTTP REST</span>
                </div>
                <div className={card}>
                  <div className="flex items-center gap-1 border-b border-zinc-800/80 bg-zinc-900/60 px-3 pt-2">
                    {VENDOR_SNIPPET_TABS.map(([key, label]) => (
                      <button
                        key={key}
                        onClick={() => setVendorSnippet(key)}
                        className={`rounded-t-lg px-3 py-1.5 text-xs font-medium transition-all ${
                          vendorSnippet === key
                            ? "border-b-2 border-indigo-500 bg-zinc-800/50 text-zinc-100"
                            : "text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                    <div className="ml-auto pb-1">
                      <CopyButton value={currentVendorSnippet} />
                    </div>
                  </div>
                  <div className="relative">
                    <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed text-zinc-300">
                      {currentVendorSnippet}
                    </pre>
                  </div>
                </div>
              </section>

              {/* Send Test Email */}
              <section className="rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4 space-y-3">
                <div>
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-300">
                    Deliver Test Email
                  </h3>
                  <p className="mt-1 text-xs text-zinc-500">
                    Instantly deliver a test MIME email to verify Postwire SMTP interception.
                  </p>
                </div>
                <div className="flex gap-2">
                  <input
                    value={testTo}
                    onChange={(e) => setTestTo(e.target.value)}
                    placeholder="you@example.com (optional)"
                    className={input}
                  />
                  <button
                    onClick={handleSendTest}
                    disabled={sendState === "sending"}
                    className={`${btn.primary} shrink-0 px-4`}
                  >
                    <SendIcon />
                    {sendState === "sending" ? "Sending…" : sendState === "sent" ? "Sent! ✅" : "Send Email"}
                  </button>
                </div>
                {sendState === "error" && (
                  <p className="text-xs text-red-400">Failed to deliver test email. Check server log.</p>
                )}
              </section>
            </>
          ) : (
            <>
              {/* SMS Endpoints */}
              <section className="space-y-3">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                  SMS API & Webhook Endpoints
                </h3>
                <div className="grid gap-2">
                  <ConnectionField method="POST" label="REST Ingest API" value={`http://${host}:8025/api/sms`} />
                  <ConnectionField method="POST" label="Twilio Webhook URL" value={`http://${host}:8025/api/sms/webhook`} />
                </div>
              </section>

              {/* Multi-Channel Webhook Catchers */}
              <section className="space-y-3">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                  Slack, Discord & Generic Webhooks
                </h3>
                <div className={card}>
                  <div className="flex items-center gap-1 border-b border-zinc-800/80 bg-zinc-900/60 px-3 pt-2">
                    {WEBHOOK_SNIPPET_TABS.map(([key, label]) => (
                      <button
                        key={key}
                        onClick={() => setWebhookSnippet(key)}
                        className={`rounded-t-lg px-3 py-1.5 text-xs font-medium transition-all ${
                          webhookSnippet === key
                            ? "border-b-2 border-emerald-500 bg-zinc-800/50 text-zinc-100"
                            : "text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                    <div className="ml-auto pb-1">
                      <CopyButton value={currentWebhookSnippet} />
                    </div>
                  </div>
                  <div className="relative">
                    <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed text-zinc-300">
                      {currentWebhookSnippet}
                    </pre>
                  </div>
                </div>
              </section>

              {/* SMS Code Snippets */}
              <section className="space-y-3">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                  SMS Integration Snippets
                </h3>
                <div className={card}>
                  <div className="flex items-center gap-1 border-b border-zinc-800/80 bg-zinc-900/60 px-3 pt-2">
                    {SMS_SNIPPET_TABS.map(([key, label]) => (
                      <button
                        key={key}
                        onClick={() => setSmsSnippet(key)}
                        className={`rounded-t-lg px-3 py-1.5 text-xs font-medium transition-all ${
                          smsSnippet === key
                            ? "border-b-2 border-emerald-500 bg-zinc-800/50 text-zinc-100"
                            : "text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                    <div className="ml-auto pb-1">
                      <CopyButton value={currentSmsSnippet} />
                    </div>
                  </div>
                  <div className="relative">
                    <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed text-zinc-300">
                      {currentSmsSnippet}
                    </pre>
                  </div>
                </div>
              </section>

              {/* Send Test SMS */}
              <section className="rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4 space-y-3">
                <div>
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-300">
                    Deliver Test SMS
                  </h3>
                  <p className="mt-1 text-xs text-zinc-500">
                    Synthesize and inject a verification SMS directly into your Postwire SMS inbox.
                  </p>
                </div>
                <div className="flex gap-2">
                  <input
                    value={smsTestTo}
                    onChange={(e) => setSmsTestTo(e.target.value)}
                    placeholder="+15550100 (optional)"
                    className={input}
                  />
                  <button
                    onClick={handleSendSmsTest}
                    disabled={smsSendState === "sending"}
                    className={`${btn.primary} shrink-0 px-4 !bg-emerald-600 hover:!bg-emerald-500`}
                  >
                    <SendIcon />
                    {smsSendState === "sending" ? "Sending…" : smsSendState === "sent" ? "Sent! ✅" : "Send SMS"}
                  </button>
                </div>
                {smsSendState === "error" && (
                  <p className="text-xs text-red-400">Failed to deliver test SMS. Check server log.</p>
                )}
              </section>
            </>
          )}
        </div>

        {/* Footer */}
        <footer className="border-t border-zinc-800/80 px-6 py-4 text-xs text-zinc-500">
          Built by{" "}
          <a
            href="https://yoosuf.me/"
            target="_blank"
            rel="noreferrer"
            className="font-medium text-indigo-400 hover:underline"
          >
            Yoosuf
          </a>
          , who also offers{" "}
          <a
            href="https://yoosuf.me/services/"
            target="_blank"
            rel="noreferrer"
            className="font-medium text-indigo-400 hover:underline"
          >
            fractional CTO services
          </a>
          .
        </footer>
      </div>
    </div>
  );
});

