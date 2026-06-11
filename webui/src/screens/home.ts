export function create_home_screen(
  on_solo: () => void,
  on_multi: () => void,
  on_settings: () => void,
): HTMLElement {
  const container = document.createElement('div');
  container.className = 'home';

  const multiCard = document.createElement('div');
  multiCard.className = 'menu-card menu-multi';
  multiCard.style.opacity = '0.5';
  multiCard.style.cursor = 'not-allowed';
  multiCard.innerHTML = `
    <div class="menu-left">MP</div>
    <div class="menu-body">
      <div class="menu-title">MULTIPLAYER</div>
      <div class="menu-sub" style="display:flex;justify-content:space-between;align-items:center;">
        <span>PLAY ONLINE WITH FRIENDS AND FOES</span>
        <span style="color:var(--color-accent);font-size:10px;">COMING SOON</span>
      </div>
    </div>
  `;

  const soloCard = document.createElement('div');
  soloCard.className = 'menu-card menu-solo';
  soloCard.onclick = on_solo;
  soloCard.innerHTML = `
    <div class="menu-left">SP</div>
    <div class="menu-body">
      <div class="menu-title">SOLO</div>
      <div class="menu-sub">CHALLENGE YOURSELF AND TOP THE LEADERBOARDS</div>
    </div>
  `;

  const settingsCard = document.createElement('div');
  settingsCard.className = 'menu-card menu-settings';
  settingsCard.onclick = on_settings;
  settingsCard.innerHTML = `
    <div class="menu-left">CFG</div>
    <div class="menu-body">
      <div class="menu-title">SETTINGS</div>
      <div class="menu-sub">TWEAK YOUR EXPERIENCE</div>
    </div>
  `;

  container.appendChild(soloCard);
  container.appendChild(multiCard);
  container.appendChild(settingsCard);
  return container;
}
