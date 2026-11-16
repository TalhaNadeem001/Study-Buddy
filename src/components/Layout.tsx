import { NavLink, Outlet } from "react-router-dom";
import { Mic, Settings, ScrollText, AudioLines } from "lucide-react";

const tabs = [
  { to: "/", label: "Listen", icon: Mic, end: true },
  { to: "/voice", label: "Voice", icon: AudioLines, end: false },
  { to: "/notes", label: "Notes", icon: ScrollText, end: false },
  { to: "/settings", label: "Settings", icon: Settings, end: false },
];

export default function Layout() {
  return (
    <div className="min-h-screen flex flex-col">
      <header className="border-b border-slate-200 bg-white">
        <div className="mx-auto max-w-3xl px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Mic className="h-5 w-5 text-indigo-600" />
            <span className="font-semibold">Study Buddy AI</span>
          </div>
          <nav className="flex gap-1">
            {tabs.map(({ to, label, icon: Icon, end }) => (
              <NavLink
                key={to}
                to={to}
                end={end}
                className={({ isActive }) =>
                  `flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition ${
                    isActive
                      ? "bg-indigo-50 text-indigo-700"
                      : "text-slate-600 hover:bg-slate-100"
                  }`
                }
              >
                <Icon className="h-4 w-4" />
                {label}
              </NavLink>
            ))}
          </nav>
        </div>
      </header>
      <main className="mx-auto w-full max-w-3xl flex-1 px-4 py-6">
        <Outlet />
      </main>
    </div>
  );
}
