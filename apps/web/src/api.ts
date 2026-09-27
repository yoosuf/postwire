import type { MessageDetail, MessageList, ExtractedSignals, MessageAnalysis, ServerConfig, MessageSummary, SmsMessage, SmsList } from "./types";

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) {
    throw new Error(`${res.status} ${res.statusText}`);
  }
  return (await res.json()) as T;
}

export const api = {
  list(search: string, offset = 0, limit = 50): Promise<MessageList> {
    const params = new URLSearchParams({ offset: String(offset), limit: String(limit) });
    if (search) params.set("search", search);
    return fetch(`/api/messages?${params}`).then((r) => json(r));
  },

  get(id: string): Promise<MessageDetail> {
    return fetch(`/api/messages/${id}`).then((r) => json(r));
  },

  extract(id: string, regex?: string): Promise<ExtractedSignals> {
    const params = new URLSearchParams();
    if (regex) params.set("regex", regex);
    const q = params.toString();
    return fetch(`/api/messages/${id}/extract${q ? `?${q}` : ""}`).then((r) => json(r));
  },

  analysis(id: string): Promise<MessageAnalysis> {
    return fetch(`/api/messages/${id}/analysis`).then((r) => json(r));
  },

  source(id: string): Promise<string> {
    return fetch(`/api/messages/${id}/raw`).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
      return r.text();
    });
  },

  markRead(id: string, read: boolean): Promise<void> {
    return fetch(`/api/messages/${id}/read`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ read }),
    }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  delete(id: string): Promise<void> {
    return fetch(`/api/messages/${id}`, { method: "DELETE" }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  bulkDelete(ids: string[]): Promise<void> {
    return fetch(`/api/messages/bulk-delete`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ids }),
    }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  bulkMarkRead(ids: string[], read: boolean): Promise<void> {
    return fetch(`/api/messages/bulk-read`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ids, read }),
    }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  clear(): Promise<void> {
    return fetch(`/api/messages`, { method: "DELETE" }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  config(): Promise<ServerConfig> {
    return fetch(`/api/config`).then((r) => json(r));
  },

  sendTestEmail(to?: string): Promise<MessageSummary> {
    return fetch(`/api/test-email`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ to: to || undefined }),
    }).then((r) => json(r));
  },

  rawUrl(id: string): string {
    return `/api/messages/${id}/raw`;
  },

  htmlUrl(id: string): string {
    return `/api/messages/${id}/html`;
  },

  attachmentUrl(id: string, index: number): string {
    return `/api/messages/${id}/attachments/${index}`;
  },

  // --- SMS API ---

  listSms(search: string, offset = 0, limit = 50): Promise<SmsList> {
    const params = new URLSearchParams({ offset: String(offset), limit: String(limit) });
    if (search) params.set("search", search);
    return fetch(`/api/sms?${params}`).then((r) => json(r));
  },

  getSms(id: string): Promise<SmsMessage> {
    return fetch(`/api/sms/${id}`).then((r) => json(r));
  },

  extractSms(id: string, regex?: string): Promise<ExtractedSignals> {
    const params = new URLSearchParams();
    if (regex) params.set("regex", regex);
    const q = params.toString();
    return fetch(`/api/sms/${id}/extract${q ? `?${q}` : ""}`).then((r) => json(r));
  },

  markSmsRead(id: string, read: boolean): Promise<void> {
    return fetch(`/api/sms/${id}/read`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ read }),
    }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  deleteSms(id: string): Promise<void> {
    return fetch(`/api/sms/${id}`, { method: "DELETE" }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  bulkDeleteSms(ids: string[]): Promise<void> {
    return fetch(`/api/sms/bulk-delete`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ids }),
    }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  bulkMarkSmsRead(ids: string[], read: boolean): Promise<void> {
    return fetch(`/api/sms/bulk-read`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ids, read }),
    }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  clearSms(): Promise<void> {
    return fetch(`/api/sms`, { method: "DELETE" }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
    });
  },

  sendTestSms(to?: string, from?: string, body?: string): Promise<SmsMessage> {
    return fetch(`/api/test-sms`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ to, from, body }),
    }).then((r) => json(r));
  },

  replay(id: string, targetUrl: string): Promise<{ success: boolean; message: string }> {
    return fetch(`/api/messages/${id}/replay`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ target_url: targetUrl }),
    }).then((r) => json(r));
  },

  replaySms(id: string, targetUrl: string): Promise<{ success: boolean; message: string }> {
    return fetch(`/api/sms/${id}/replay`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ target_url: targetUrl }),
    }).then((r) => json(r));
  },
};

