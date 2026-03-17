import { useEffect, useState } from "react";
import type { ThemeListResponse } from "@cmux-win/protocol";
import { getThemes, setActiveTheme } from "../lib/desktopClient";

export function ThemeSelector() {
  const [themeData, setThemeData] = useState<ThemeListResponse | null>(null);

  useEffect(() => {
    getThemes()
      .then(setThemeData)
      .catch(() => {});
  }, []);

  const handleChange = (themeId: string) => {
    setActiveTheme(themeId)
      .then(() => getThemes())
      .then(setThemeData)
      .catch(() => {});
  };

  if (!themeData) return null;

  return (
    <div className="theme-selector">
      <label>
        <span>Theme</span>
        <select
          value={themeData.activeThemeId}
          onChange={(e) => handleChange(e.target.value)}
          aria-label="Theme"
        >
          {themeData.themes.map((theme) => (
            <option key={theme.id} value={theme.id}>
              {theme.name}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
