import { useEffect, useState } from "react";
import { api } from "../api";
import { useInboxStore } from "../store";
import type { ExtractedSignals, SmsMessage } from "../types";
import { btn } from "../ui";
import { CopyIcon, PhoneIcon, TrashIcon } from "./Icons";

type SnippetLang = "playwright" | "cypress" | "python" | "node";

export function SmsView() {
  const smsSelectedId = useInboxStore((s) => s.smsSelectedId);
  const setSmsSelectedId = useInboxStore((s) => s.setSmsSelectedId);
  const [sms, setSms] = useState<SmsMessage | null>(null);
  const [signals, setSignals] = useState<ExtractedSignals | null>(null);
  const [copiedText, setCopiedText] = useState<string | null>(null);
  const [regexPattern, setRegexPattern] = useState("");
  const [loading, setLoading] = useState(false);
  const [showSnippet, setShowSnippet] = useState(false);
  const [snippetLang, setSnippetLang] = useState<SnippetLang>("playwright");
  const [showReplay, setShowReplay] = useState(false);
  const [replayUrl, setReplayUrl] = useState("");
  const [replayResult, setReplayResult] = useState<{ success: boolean; message: string } | null>(null);
  const [replaying, setReplaying] = useState(false);

  useEffect(() => {
    if (!smsSelectedId) {
      setSms(null);
      setSignals(null);
      setLoading(false);
      setRegexPattern("");
      setShowSnippet(false);
      setShowReplay(false);
      setReplayResult(null);
      return;
    }

    let isMounted = true;
    setSms(null);
    setSignals(null);
    setLoading(true);
    setRegexPattern("");
    setShowSnippet(false);
    setShowReplay(false);
    setReplayResult(null);

    api
      .getSms(smsSelectedId)
      .then((data) => {
        if (!isMounted) return;
        setSms(data);
        if (!data.read) {
          void api.markSmsRead(data.id, true);
        }
      })
      .catch(() => {
        if (isMounted) setSms(null);
      })
      .finally(() => {
        if (isMounted) setLoading(false);
      });

    api
      .extractSms(smsSelectedId)
      .then((sig) => {
        if (isMounted) setSignals(sig);
      })
      .catch(() => {
        if (isMounted) setSignals(null);
      });

    return () => {
      isMounted = false;
    };
  }, [smsSelectedId]);

  const copyToClipboard = (text: string) => {
    void navigator.clipboard.writeText(text);
    setCopiedText(text);
    setTimeout(() => setCopiedText(null), 2000);
  };

  const handleDelete = async () => {
    if (!sms) return;
    await api.deleteSms(sms.id);
    setSmsSelectedId(null);
  };

  async function handleReplay() {
    if (!replayUrl.trim() || !smsSelectedId) return;
    setReplaying(true);
    setReplayResult(null);
    try {
      const res = await api.replaySms(smsSelectedId, replayUrl.trim());
      setReplayResult(res);
    } catch (e) {
      setReplayResult({ success: false, message: String(e) });
    } finally {
      setReplaying(false);
    }
  }

  function buildSnippet(lang: SnippetLang): string {
    const baseUrl = window.location.origin;
    const toPhone = sms?.to ?? "+15550001234";
    if (lang === "playwright") {
      return (
        "import { test } from '@playwright/test';\n\n" +
        "test('verify SMS OTP flow', async ({ page }) => {\n" +
        "  const since = new Date().toISOString();\n" +
        "  // 1. Trigger action that sends SMS\n" +
        "  await page.goto('https://your-app.example.com/login');\n\n" +
        "  // 2. Wait for SMS to arrive\n" +
        "  const res = await fetch(`" + baseUrl + "/api/sms/wait?to=" + encodeURIComponent(toPhone) + "&since=${since}`);\n" +
        "  const smsMsg = await res.json();\n\n" +
        "  // 3. Extract OTP code\n" +
        "  const ext = await fetch(`" + baseUrl + "/api/sms/${smsMsg.id}/extract`).then(r => r.json());\n" +
        "  const otp = ext.codes[0];\n\n" +
        "  // 4. Enter the OTP\n" +
        "  await page.fill('#otp-input', otp);\n" +
        "  await page.click('#verify-btn');\n" +
        "});"
      );
    }
    if (lang === "cypress") {
      return (
        "describe('SMS OTP flow', () => {\n" +
        "  it('receives and verifies OTP via SMS', () => {\n" +
        "    const since = new Date().toISOString();\n" +
        "    cy.visit('https://your-app.example.com/login');\n\n" +
        "    cy.request(`" + baseUrl + "/api/sms/wait?to=" + encodeURIComponent(toPhone) + "&since=${since}`)\n" +
        "      .its('body').then((smsMsg) => {\n" +
        "        cy.request(`" + baseUrl + "/api/sms/${smsMsg.id}/extract`)\n" +
        "          .its('body.codes.0').then((otp) => {\n" +
        "            cy.get('#otp-input').type(otp);\n" +
        "            cy.get('#verify-btn').click();\n" +
        "          });\n" +
        "      });\n" +
        "  });\n" +
        "});"
      );
    }
    if (lang === "python") {
      return (
        "import requests\n" +
        "from datetime import datetime, timezone\n\n" +
        'BASE = "' + baseUrl + '"\n\n' +
        "since = datetime.now(timezone.utc).isoformat()\n\n" +
        "# Trigger your app action here\n\n" +
        "# Wait for SMS\n" +
        "sms_msg = requests.get(f\"{BASE}/api/sms/wait\", params={\n" +
        '    "to": "' + toPhone + '",\n' +
        '    "since": since,\n' +
        '    "timeout_ms": 15000,\n' +
        "}).json()\n\n" +
        "# Extract OTP\n" +
        "ext = requests.get(f\"{BASE}/api/sms/{sms_msg['id']}/extract\").json()\n" +
        "otp = ext['codes'][0] if ext['codes'] else None\n" +
        "print('SMS OTP:', otp)"
      );
    }
    // node
    return (
      "const BASE = '" + baseUrl + "';\n\n" +
      "async function waitForSms() {\n" +
      "  const since = new Date().toISOString();\n\n" +
      "  // Trigger app action here first\n\n" +
      "  const smsMsg = await fetch(`${BASE}/api/sms/wait?to=" + encodeURIComponent(toPhone) + "&since=${since}`)\n" +
      "    .then(r => r.json());\n\n" +
      "  const ext = await fetch(`${BASE}/api/sms/${smsMsg.id}/extract`).then(r => r.json());\n" +
      "  console.log('SMS OTP:', ext.codes[0]);\n" +
      "  return ext;\n" +
      "}\n\n" +
      "waitForSms();"
    );
  }

  if (!smsSelectedId) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-8 text-center text-zinc-500">
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-zinc-900/80 text-zinc-600 ring-1 ring-zinc-800">
          <PhoneIcon width={24} height={24} />
        </div>
        <p className="mt-3 text-sm font-medium text-zinc-400">Select an SMS message to view details</p>
        <p className="mt-1 text-xs text-zinc-600">SMS messages captured via API or webhook will appear here.</p>
      </div>
    );
  }

  if (loading || !sms || sms.id !== smsSelectedId) {
    return (
      <div className="flex h-full flex-col bg-zinc-950 p-6 space-y-6 animate-pulse">
        <div className="flex items-center justify-between border-b border-zinc-800/80 pb-4">
          <div className="space-y-2">
            <div className="h-5 w-40 rounded-md bg-zinc-800" />
            <div className="h-3 w-28 rounded bg-zinc-800/60" />
          </div>
          <div className="h-8 w-20 rounded-lg bg-zinc-800/60" />
        </div>
        <div className="rounded-xl border border-zinc-800/60 bg-zinc-900/40 p-4 space-y-2">
          <div className="h-3 w-36 rounded bg-zinc-800/80" />
          <div className="h-7 w-24 rounded-lg bg-zinc-800" />
        </div>
        <div className="space-y-2 max-w-lg">
          <div className="h-3 w-32 rounded bg-zinc-800/60" />
          <div className="h-24 w-full rounded-2xl bg-zinc-900/80 border border-zinc-800/60" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-zinc-950">
      {/* Header Toolbar */}
      <div className="flex items-center justify-between border-b border-zinc-800/80 px-6 py-4">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-base font-semibold text-zinc-100">{sms.from}</h2>
            <span className="rounded-md bg-indigo-500/10 px-2 py-0.5 text-xs font-medium text-indigo-400 ring-1 ring-inset ring-indigo-500/20">
              SMS
            </span>
          </div>
          <p className="mt-0.5 text-xs text-zinc-400">To: {sms.to}</p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={() => { setShowSnippet((v) => !v); setShowReplay(false); }}
            className={btn.secondary}
          >
            {showSnippet ? "Hide Snippet" : "📋 E2E Test Snippet"}
          </button>
          <button
            onClick={() => { setShowReplay((v) => !v); setShowSnippet(false); setReplayResult(null); }}
            className={btn.secondary}
          >
            {showReplay ? "Hide Replay" : "↩ Replay Payload"}
          </button>
          <button onClick={() => void handleDelete()} className={`${btn.secondary} text-red-400 hover:text-red-300`}>
            <TrashIcon width={14} height={14} />
            Delete
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4">
        {/* E2E Test Snippet Generator */}
        {showSnippet && (
          <div className="rounded-xl border border-emerald-900/50 bg-emerald-950/20 p-4">
            <div className="mb-2 flex items-center justify-between">
              <span className="text-xs font-semibold text-emerald-400">E2E Test Snippet</span>
              <div className="flex gap-1">
                {(["playwright", "cypress", "python", "node"] as SnippetLang[]).map((l) => (
                  <button
                    key={l}
                    onClick={() => setSnippetLang(l)}
                    className={`rounded px-2 py-0.5 text-xs capitalize transition-colors ${
                      snippetLang === l
                        ? "bg-emerald-700 text-white"
                        : "text-zinc-400 hover:text-zinc-200"
                    }`}
                  >
                    {l === "playwright" ? "Playwright" : l === "cypress" ? "Cypress" : l === "python" ? "Python" : "Node.js"}
                  </button>
                ))}
              </div>
            </div>
            <pre className="overflow-x-auto rounded bg-zinc-900 p-3 font-mono text-[11px] text-zinc-300 leading-relaxed whitespace-pre-wrap">
              {buildSnippet(snippetLang)}
            </pre>
            <button
              onClick={() => void navigator.clipboard.writeText(buildSnippet(snippetLang))}
              className="mt-2 rounded bg-emerald-700/40 px-3 py-1 text-xs text-emerald-300 hover:bg-emerald-700/70"
            >
              Copy to clipboard
            </button>
          </div>
        )}

        {/* Replay Payload */}
        {showReplay && (
          <div className="rounded-xl border border-amber-900/50 bg-amber-950/20 p-4">
            <span className="text-xs font-semibold text-amber-400">Replay SMS Payload to Endpoint</span>
            <div className="mt-2 flex gap-2">
              <input
                type="url"
                placeholder="https://your-app.example.com/sms-webhook"
                value={replayUrl}
                onChange={(e) => setReplayUrl(e.target.value)}
                className="flex-1 rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs text-zinc-200 placeholder-zinc-600 focus:border-amber-500 focus:outline-none"
              />
              <button
                onClick={() => void handleReplay()}
                disabled={replaying || !replayUrl.trim()}
                className="rounded bg-amber-700/60 px-3 py-1 text-xs text-amber-100 hover:bg-amber-700/90 disabled:opacity-50"
              >
                {replaying ? "Sending…" : "Send"}
              </button>
            </div>
            {replayResult && (
              <p className={`mt-2 text-xs ${replayResult.success ? "text-emerald-400" : "text-red-400"}`}>
                {replayResult.message}
              </p>
            )}
          </div>
        )}

        {/* Signal Extraction Box (OTP Codes, Links, Custom Regex) */}
        <div className="rounded-xl border border-indigo-500/20 bg-indigo-950/20 p-4 backdrop-blur-sm">
          <div className="flex flex-wrap items-center justify-between gap-2 mb-2">
            <span className="text-xs font-semibold uppercase tracking-wider text-indigo-400">
              Auto-Detected Verification Signals
            </span>
            <input
              type="text"
              placeholder="Custom regex (e.g. OTP-[0-9]+)..."
              value={regexPattern}
              onChange={(e) => {
                const pat = e.target.value;
                setRegexPattern(pat);
                if (smsSelectedId) {
                  api.extractSms(smsSelectedId, pat || undefined).then(setSignals).catch(() => {});
                }
              }}
              className="w-64 rounded border border-zinc-800 bg-zinc-900 px-2 py-1 text-xs text-zinc-200 placeholder-zinc-600 focus:border-indigo-500 focus:outline-none"
            />
          </div>

          <div className="flex flex-wrap gap-2">
            {signals?.codes.map((code) => (
              <button
                key={code}
                onClick={() => copyToClipboard(code)}
                className="flex items-center gap-1.5 rounded-lg border border-indigo-500/30 bg-indigo-900/40 px-3 py-1.5 text-sm font-mono font-bold text-indigo-200 transition hover:bg-indigo-900/70"
              >
                <CopyIcon width={14} height={14} />
                <span>{code}</span>
                {copiedText === code && <span className="ml-1 text-[10px] text-green-400">Copied!</span>}
              </button>
            ))}

            {signals?.matches?.map((m) => (
              <span key={m} className="flex items-center rounded-lg border border-emerald-500/30 bg-emerald-950/40 px-3 py-1.5 text-xs font-mono font-semibold text-emerald-300">
                Match: {m}
              </span>
            ))}

            {signals?.links.map((link) => (
              <a
                key={link}
                href={link}
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1.5 truncate rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-xs text-indigo-300 transition hover:bg-zinc-800"
              >
                <span className="truncate">{link}</span>
              </a>
            ))}

            {(!signals || (signals.codes.length === 0 && (!signals.matches || signals.matches.length === 0) && signals.links.length === 0)) && (
              <span className="text-xs text-zinc-500 italic">No verification signals extracted</span>
            )}
          </div>
        </div>

        {/* Message Bubble Container */}
        <div className="flex flex-col items-start max-w-2xl">
          <div className="mb-1 text-xs text-zinc-500">{new Date(sms.received_at).toLocaleString()}</div>
          <div className="rounded-2xl rounded-tl-sm border border-zinc-800 bg-zinc-900/80 px-5 py-4 text-sm leading-relaxed text-zinc-200 shadow-md">
            {sms.body || <span className="italic text-zinc-600">(empty SMS message)</span>}
          </div>
        </div>
      </div>
    </div>
  );
}
