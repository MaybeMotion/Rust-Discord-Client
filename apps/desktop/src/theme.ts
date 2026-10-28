export interface Theme {
  version: 1;
  name: string;
  colors: { background: string; surface: string; text: string; muted: string; accent: string; border: string };
  glass: boolean;
}
export const themes: Record<string, Theme> = {
  dark: { version: 1, name: 'Midnight', colors: { background: '#11131b', surface: '#1b1e2b', text: '#eef0ff', muted: '#abb1ce', accent: '#98aaff', border: '#353b55' }, glass: false },
  light: { version: 1, name: 'Daylight', colors: { background: '#eff2fa', surface: '#ffffff', text: '#17223a', muted: '#52617b', accent: '#4b42ce', border: '#c9d0df' }, glass: false },
  neon: { version: 1, name: 'Neon glass', colors: { background: '#100d23', surface: '#231b39', text: '#f9edff', muted: '#c6aedc', accent: '#48f7da', border: '#634479' }, glass: true },
};
const colorKeys = Object.keys(themes.dark.colors) as (keyof Theme['colors'])[];
export function parseTheme(input: unknown): Theme {
  if (!input || typeof input !== 'object') throw new Error('Theme must be an object');
  const theme = input as Partial<Theme>;
  if (theme.version !== 1 || typeof theme.name !== 'string' || theme.name.length > 60 || typeof theme.glass !== 'boolean' || !theme.colors) throw new Error('Invalid theme metadata');
  const colors = {} as Theme['colors'];
  for (const key of colorKeys) {
    const value = theme.colors[key];
    // Deliberately allow colors only, never CSS URLs, rules, scripts or arbitrary properties.
    if (typeof value !== 'string' || !/^#[0-9a-f]{6}$/i.test(value)) throw new Error(`Invalid color: ${key}`);
    colors[key] = value;
  }
  return { version: 1, name: theme.name, glass: theme.glass, colors };
}
export function applyTheme(theme: Theme): void {
  for (const key of colorKeys) document.documentElement.style.setProperty(`--${key}`, theme.colors[key]);
  document.documentElement.dataset.glass = String(theme.glass);
}
