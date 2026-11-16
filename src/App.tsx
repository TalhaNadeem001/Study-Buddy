import { Routes, Route } from "react-router-dom";
import Layout from "./components/Layout";
import HomePage from "./pages/HomePage";
import VoicePage from "./pages/VoicePage";
import NotesPage from "./pages/NotesPage";
import SettingsPage from "./pages/SettingsPage";

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<HomePage />} />
        <Route path="voice" element={<VoicePage />} />
        <Route path="notes" element={<NotesPage />} />
        <Route path="settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}
