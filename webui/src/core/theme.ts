export type ThemeName = 'cyberpunk' | 'retro' | 'minimal';

const VALID_THEMES: ThemeName[] = ['cyberpunk', 'retro', 'minimal'];

export function get_theme_color(var_name: string): string {
  const style = getComputedStyle(document.body);
  return style.getPropertyValue(var_name).trim();
}

export function apply_theme(name: ThemeName): void {
  if (!VALID_THEMES.includes(name)) {
    name = 'cyberpunk';
  }
  document.body.setAttribute('data-theme', name);
}

export function get_current_theme(): ThemeName {
  const theme = document.body.getAttribute('data-theme') as ThemeName;
  return VALID_THEMES.includes(theme) ? theme : 'cyberpunk';
}
