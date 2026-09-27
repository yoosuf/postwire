import { useEffect, useRef, useState } from "react";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { MailIcon, PhoneIcon, PineTreeIcon, PlugIcon, RefreshIcon, SearchIcon } from "./components/Icons";
import { MessageList } from "./components/MessageList";
import { MessageView } from "./components/MessageView";
import { SetupPanel } from "./components/SetupPanel";
import { SmsList } from "./components/SmsList";
import { SmsView } from "./components/SmsView";
import { btn } from "./ui";
import { useInboxStore } from "./store";
import { useServerEvents } from "./useServerEvents";
import { useRouterSync } from "./useRouterSync";

export default function App() {
  useRouterSync();

  const activeTab = useInboxStore((s) => s.activeTab);
  const setActiveTab = useInboxStore((s) => s.setActiveTab);

  const [inputValue, setInputValue] = useState("");

  const total = useInboxStore((s) => s.total);
  const setSearch = useInboxStore((s) => s.setSearch);
  const fetchFirstPage = useInboxStore((s) => s.fetchFirstPage);
  const loading = useInboxStore((s) => s.loading);

  const smsTotal = useInboxStore((s) => s.smsTotal);
  const setSmsSearch = useInboxStore((s) => s.setSmsSearch);
  const fetchFirstSmsPage = useInboxStore((s) => s.fetchFirstSmsPage);
  const smsLoading = useInboxStore((s) => s.smsLoading);

  const applyEvent = useInboxStore((s) => s.applyEvent);
  const openSetupWithSegment = useInboxStore((s) => s.openSetupWithSegment);
  const isFirstRender = useRef(true);

  useEffect(() => {
    setInputValue("");
  }, [activeTab]);

  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false;
      void setSearch(inputValue);
      return;
    }
    const handle = setTimeout(() => {
      if (activeTab === "emails") {
        setSearch(inputValue);
      } else {
        setSmsSearch(inputValue);
      }
    }, 200);
    return () => clearTimeout(handle);
  }, [inputValue, activeTab, setSearch, setSmsSearch]);

  useServerEvents(applyEvent);

  const handleRefresh = () => {
    if (activeTab === "emails") {
      void fetchFirstPage();
    } else {
      void fetchFirstSmsPage();
    }
  };

  const isLoading = activeTab === "emails" ? loading : smsLoading;

  return (
    <div className="flex h-screen flex-col bg-zinc-950">
      <header className="flex items-center gap-3 border-b border-zinc-800/80 bg-zinc-950/95 px-4 py-3 backdrop-blur-xl">
        <div className="flex items-center gap-2 pr-2">
          <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-gradient-to-br from-indigo-500 to-violet-600 text-white shadow-sm shadow-indigo-950/50">
            <PineTreeIcon width={15} height={15} />
          </div>
          <h1 className="text-sm font-semibold tracking-tight text-zinc-100">Postwire</h1>
        </div>

        {/* Tab Toggle (Emails vs SMS) */}
        <div className="flex items-center rounded-lg bg-zinc-900/90 p-1 ring-1 ring-zinc-800">
          <button
            onClick={() => setActiveTab("emails")}
            className={`flex items-center gap-1.5 rounded-md px-3 py-1 text-xs font-medium transition-all ${
              activeTab === "emails"
                ? "bg-zinc-800 text-zinc-100 shadow-sm"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <MailIcon width={13} height={13} />
            <span>Email</span>
            <span className="ml-1 rounded-full bg-zinc-950/80 px-1.5 py-0.2 text-[10px] text-zinc-400">
              {total}
            </span>
          </button>

          <button
            onClick={() => setActiveTab("sms")}
            className={`flex items-center gap-1.5 rounded-md px-3 py-1 text-xs font-medium transition-all ${
              activeTab === "sms"
                ? "bg-zinc-800 text-zinc-100 shadow-sm"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <PhoneIcon width={13} height={13} />
            <span>SMS</span>
            <span className="ml-1 rounded-full bg-zinc-950/80 px-1.5 py-0.2 text-[10px] text-zinc-400">
              {smsTotal}
            </span>
          </button>
        </div>

        {/* Search Bar */}
        <div className="relative w-72">
          <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-600" />
          <input
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            placeholder={activeTab === "emails" ? "Search from, to, subject…" : "Search from, to, body…"}
            className="w-full rounded-lg border border-zinc-800 bg-zinc-900/70 py-1.5 pl-8 pr-3 text-sm text-zinc-200 placeholder-zinc-600 transition-colors focus:border-indigo-500/60 focus:outline-none"
          />
        </div>

        <div className="flex-1" />

        <button
          onClick={() => openSetupWithSegment(activeTab === "emails" ? "email" : "sms")}
          className={btn.secondary}
        >
          <PlugIcon />
          Setup
        </button>
        <button onClick={handleRefresh} disabled={isLoading} className={btn.secondary}>
          <RefreshIcon className={isLoading ? "animate-spin" : ""} />
          Refresh
        </button>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <div className="w-96 shrink-0 border-r border-zinc-800/80 bg-zinc-950">
          {activeTab === "emails" ? <MessageList /> : <SmsList />}
        </div>
        <div className="flex-1 overflow-hidden bg-zinc-950">
          {activeTab === "emails" ? <MessageView /> : <SmsView />}
        </div>
      </div>

      <SetupPanel />
      <ConfirmDialog />
    </div>
  );
}
