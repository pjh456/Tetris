import './style.css';
import { effect, untracked } from '@preact/signals-core';
import { page, settings, is_multiplayer } from './state';
import { apply_theme, type ThemeName } from './core/theme';
import { init_particles, apply_theme_particles } from './core/particles';
import { init_wasm } from './core/wasm';
import { create_home_screen } from './screens/home';
import { create_game_screen } from './screens/game';
import { create_settings_screen } from './screens/settings';
import { create_gameover_screen } from './screens/gameover';
import { create_leaderboard_screen } from './screens/leaderboard';
import { create_lobby_screen } from './screens/lobby';
import { create_spectator_screen } from './screens/spectator';

const boot_theme = (settings.value.theme as ThemeName) || 'cyberpunk';
apply_theme(boot_theme);
init_particles(boot_theme);

effect(() => {
  const theme = (settings.value.theme as ThemeName) || 'cyberpunk';
  apply_theme(theme);
  apply_theme_particles(theme);
});

const app = document.getElementById('app')!;

function mount_topbar(container: HTMLElement) {
  const topbar = document.createElement('div');
  topbar.className = 'topbar';
  topbar.innerHTML = `
    <div class="topbar-left">
      <span class="back-btn" title="Back to Home">←</span>
      <div class="logo">TETRIS</div>
    </div>
    <div class="user-info">
      <div>GUEST</div>
      <div class="status">OFFLINE</div>
    </div>
  `;
  topbar.querySelector('.back-btn')!.addEventListener('click', () => {
    page.value = 'home';
  });
  container.prepend(topbar);
}

function mount_back_button(container: HTMLElement) {
  const btn = document.createElement('button');
  btn.className = 'back-float';
  btn.textContent = '← Back';
  btn.title = 'Back to Home';
  btn.addEventListener('click', () => {
    page.value = 'home';
  });
  container.appendChild(btn);
}

type CleanableElement = HTMLElement & { _cleanup?: () => void };
let active_lobby: CleanableElement | null = null;

effect(() => {
  const current = page.value;
  active_lobby?._cleanup?.();
  active_lobby = null;
  app.innerHTML = '';

  // Reset multiplayer state when leaving multiplayer screens
  if (current === 'home' || current === 'game') {
    is_multiplayer.value = false;
  }

  switch (current) {
    case 'home': {
      mount_topbar(app);
      const content = document.createElement('div');
      content.className = 'content';
      content.appendChild(
        create_home_screen(
          () => {
            page.value = 'game';
          },
          () => {},
          () => {
            page.value = 'settings';
          },
        ),
      );
      app.appendChild(content);
      break;
    }
    case 'game': {
      const content = document.createElement('div');
      content.className = 'content';
      app.appendChild(content);
      create_game_screen(content);
      mount_back_button(app);
      break;
    }
    case 'settings': {
      mount_topbar(app);
      const content = document.createElement('div');
      content.className = 'content';
      content.appendChild(create_settings_screen());
      app.appendChild(content);
      break;
    }
    case 'gameover': {
      app.appendChild(create_gameover_screen());
      mount_back_button(app);
      break;
    }
    case 'leaderboard': {
      app.appendChild(create_leaderboard_screen());
      break;
    }
    case 'lobby': {
      mount_topbar(app);
      const content = document.createElement('div');
      content.className = 'content';
      app.appendChild(content);

      void (async () => {
        try {
          await init_wasm(content);
          if (page.value !== 'lobby') {
            return;
          }
          const lobby_el = untracked(() => create_lobby_screen()) as CleanableElement;
          active_lobby = lobby_el;
          content.innerHTML = '';
          content.appendChild(lobby_el);
        } catch {
          // `init_wasm` already renders error state into `content`.
        }
      })();
      break;
    }
    case 'spectator': {
      app.appendChild(create_spectator_screen([{ id: 1, name: 'Player 1' }]));
      mount_back_button(app);
      break;
    }
  }
});
