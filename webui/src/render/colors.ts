export function get_theme_colors(): string[] {
  const style = getComputedStyle(document.body);
  return [
    '#000000',
    style.getPropertyValue('--color-locked').trim() || '#aaaaaa',
    style.getPropertyValue('--color-ghost-border').trim() || '#444444',
    style.getPropertyValue('--color-piece-i').trim() || '#00ffff',
    style.getPropertyValue('--color-piece-o').trim() || '#ffff00',
    style.getPropertyValue('--color-piece-t').trim() || '#ff00ff',
    style.getPropertyValue('--color-piece-s').trim() || '#00ff00',
    style.getPropertyValue('--color-piece-z').trim() || '#ff0000',
    style.getPropertyValue('--color-piece-j').trim() || '#0000ff',
    style.getPropertyValue('--color-piece-l').trim() || '#ff8800',
  ];
}
