import './style.css';
import { effect } from '@preact/signals-core';
import { page, settings } from './state';
import { apply_theme, type ThemeName } from './core/theme';
import { init_particles, apply_theme_particles } from './core/particles';
import { create_home_screen } from './screens/home';
import { create_game_screen } from './screens/game';
import { create_settings_screen } from './screens/settings';
import { create_gameover_screen } from './screens/gameover';
import { create_leaderboard_screen } from './screens/leaderboard';

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
    <div class="logo">TETRIS</div>
    <div class="user-info">
      <div>GUEST</div>
      <div class="status">OFFLINE</div>
    </div>
  `;
  container.prepend(topbar);
}

effect(() => {
  const current = page.value;
  app.innerHTML = '';

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
      break;
    }
    case 'leaderboard': {
      app.appendChild(create_leaderboard_screen());
      break;
    }
  }
});
