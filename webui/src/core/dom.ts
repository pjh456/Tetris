export function create_button(
  text: string,
  opts: { className?: string; ariaLabel?: string; onClick?: () => void } = {},
): HTMLButtonElement {
  const btn = document.createElement('button');
  btn.className = opts.className ?? 'btn';
  btn.textContent = text;
  if (opts.ariaLabel) btn.setAttribute('aria-label', opts.ariaLabel);
  if (opts.onClick) btn.onclick = opts.onClick;
  return btn;
}

export function create_label(text: string, className = 'lobby-label'): HTMLElement {
  const el = document.createElement('div');
  el.className = className;
  el.textContent = text;
  return el;
}
