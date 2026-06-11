export function create_home_screen(
  on_solo: () => void,
  _on_multi: () => void,
  on_settings: () => void,
): HTMLElement {
  const container = document.createElement('div');
  container.className = 'home';

  const cards: HTMLElement[] = [];

  cards.push(
    create_card({
      left_label: 'SP',
      title: 'SOLO',
      subtitle: 'CHALLENGE YOURSELF AND TOP THE LEADERBOARDS',
      css_class: 'menu-solo',
      onclick: on_solo,
    }),
  );

  cards.push(
    create_card({
      left_label: 'MP',
      title: 'MULTIPLAYER',
      subtitle: 'PLAY ONLINE WITH FRIENDS AND FOES',
      css_class: 'menu-multi',
      disabled: true,
      badge: 'COMING SOON',
    }),
  );

  cards.push(
    create_card({
      left_label: 'CFG',
      title: 'SETTINGS',
      subtitle: 'TWEAK YOUR EXPERIENCE',
      css_class: 'menu-settings',
      onclick: on_settings,
    }),
  );

  for (const card of cards) {
    container.appendChild(card);
  }

  let focused_idx = 0;
  cards[0]?.focus();

  container.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      focused_idx = (focused_idx - 1 + cards.length) % cards.length;
      cards[focused_idx].focus();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      focused_idx = (focused_idx + 1) % cards.length;
      cards[focused_idx].focus();
    }
  });

  return container;
}

function create_card(config: {
  left_label: string;
  title: string;
  subtitle: string;
  css_class: string;
  disabled?: boolean;
  badge?: string;
  onclick?: () => void;
}): HTMLElement {
  const card = document.createElement('div');
  card.className = `menu-card glass ${config.css_class}`;
  card.setAttribute('tabindex', '0');
  card.style.position = 'relative';

  card.innerHTML = `
    <div class="menu-left">${config.left_label}</div>
    <div class="menu-body">
      <div class="menu-title">${config.title}</div>
      <div class="menu-sub">${config.subtitle}</div>
    </div>
  `;

  if (config.badge) {
    const badge = document.createElement('div');
    badge.className = 'coming-soon-badge';
    badge.textContent = config.badge;
    card.appendChild(badge);
  }

  if (config.disabled) {
    card.classList.add('menu-disabled');
    card.setAttribute('aria-disabled', 'true');
    card.removeAttribute('tabindex');
  } else if (config.onclick) {
    card.addEventListener('click', config.onclick);
    card.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        config.onclick!();
      }
    });
  }

  return card;
}
