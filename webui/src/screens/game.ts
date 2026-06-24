import {
  init_wasm,
  reset_wasm,
  get_grid_view,
  get_multiplayer_snapshot,
  get_opponent_player_grid,
  type WebTetris,
} from '../core/wasm';
import { InputBuffer } from '../core/input_buffer';
import { is_hud_data, is_hard_drop_info, has_garbage_inserted } from '../types/predicates';
import {
  page,
  score,
  level,
  combo,
  b2b_count,
  lines,
  settings,
  is_multiplayer,
  connection_status,
  match_standings,
  type StandingRow,
} from '../state';
import { createBoardRenderer, create_opponent_panel } from '../render/board';
import { createPreviewRenderer, createNextStackRenderer } from '../render/preview';
import { create_hud_overlay } from '../render/hud';
import { setup_hidpi_canvas } from '../render/canvas';
import { get_theme_colors } from '../render/colors';
import { LineFx } from '../fx/line_fx';
import { HardDropFx } from '../fx/harddrop_fx';
import { shake_screen } from '../fx/shake';
import { run_collapse_animation } from '../fx/gameover_fx';
import { bindKeyboard, type KeyboardConfig } from '../input/keyboard';
import { Actions } from '../game/actions';
import { audio_manager } from '../core/audio';
import { create_label } from '../core/dom';
import { create_touch_overlay } from '../input/touch';
import { get_multiplayer_ws } from '../core/multiplayer';
import { OpponentReplayPlayer } from '../core/replay_player';
import type { MultiplayerEvent } from '../core/wasm';

export async function create_game_screen(root: HTMLElement): Promise<() => void> {
  root.innerHTML = '';
  root.className = 'content';

  let wasm: WebTetris;
  try {
    wasm = await init_wasm(root);
  } catch {
    root.innerHTML = '<div class="error">Failed to initialize WASM. Please reload.</div>';
    return () => {};
  }

  if (!is_multiplayer.value) {
    const seed = Date.now() >>> 0;
    wasm.reset(seed);
    // Clear any stale multiplayer connection status carried over from a prior session.
    connection_status.value = 'offline';
  }
  await audio_manager.init();
  audio_manager.set_sfx_volume(settings.value.sfx_volume);
  audio_manager.set_bgm_volume(settings.value.bgm_volume);

  if (settings.value.show_countdown) {
    await run_countdown(root);
  }

  audio_manager.start_bgm(settings.value.theme);
  root.innerHTML = '';

  const board_css_w = 360;
  const board_css_h = 720;

  const wrapper = document.createElement('div');
  wrapper.className = 'game-root';
  wrapper.style.position = 'relative';

  const layout = document.createElement('div');
  layout.className = 'game-layout';
  layout.style.cssText = 'display:flex;gap:16px;align-items:flex-start;justify-content:center;';

  const hold_col = document.createElement('div');
  hold_col.style.cssText = 'display:flex;flex-direction:column;gap:8px;';
  const hold_label = document.createElement('div');
  hold_label.textContent = 'HOLD';
  hold_label.style.cssText =
    'color:var(--color-text);font-size:var(--text-body);letter-spacing:2px;';
  const hold_canvas = document.createElement('canvas');
  setup_hidpi_canvas(hold_canvas, 140, 140);
  hold_canvas.style.border = '2px solid var(--color-panel-border)';
  hold_canvas.style.background = 'var(--color-panel)';
  hold_col.appendChild(hold_label);
  hold_col.appendChild(hold_canvas);

  const board_frame = document.createElement('div');
  board_frame.style.cssText =
    'position:relative;display:inline-block;border:3px solid var(--color-panel-border);background:var(--color-bg);box-shadow:0 0 20px var(--color-panel-shadow);';
  const board_canvas = document.createElement('canvas');
  setup_hidpi_canvas(board_canvas, board_css_w, board_css_h);
  const fx_canvas = document.createElement('canvas');
  fx_canvas.style.position = 'absolute';
  fx_canvas.style.inset = '0';
  fx_canvas.style.pointerEvents = 'none';
  setup_hidpi_canvas(fx_canvas, board_css_w, board_css_h);
  board_frame.appendChild(board_canvas);
  board_frame.appendChild(fx_canvas);

  const right_col = document.createElement('div');
  right_col.style.cssText = 'display:flex;flex-direction:column;gap:8px;';
  const next_label = document.createElement('div');
  next_label.textContent = 'NEXT';
  next_label.style.cssText =
    'color:var(--color-text);font-size:var(--text-body);letter-spacing:2px;text-align:center;background:var(--color-panel);border:1px solid var(--color-panel-border);padding:4px 0;';
  const next_canvas = document.createElement('canvas');
  setup_hidpi_canvas(next_canvas, 180, 480);
  next_canvas.style.border = '1px solid var(--color-panel-border)';
  next_canvas.style.background = 'var(--color-panel)';
  right_col.appendChild(next_label);
  right_col.appendChild(next_canvas);

  const hud = create_hud_overlay(right_col);

  // Opponent mini-grids (multiplayer only)
  const opponent_col = document.createElement('div');
  opponent_col.style.cssText = 'display:flex;flex-direction:column;gap:8px;min-width:72px;';
  const opponent_renderers: Array<{
    canvas: HTMLCanvasElement;
    render: (g: ArrayLike<number>, c: string[]) => void;
    destroy: () => void;
  }> = [];
  const opponent_status_els: HTMLElement[] = [];

  const mp_ws = is_multiplayer.value ? get_multiplayer_ws() : null;
  const opponent_grids: Map<number, Uint8Array> = new Map();
  const opponent_labels: HTMLElement[] = [];
  const opponent_slots: HTMLElement[] = [];
  const replay_players: Map<number, OpponentReplayPlayer> = new Map();
  const connection_badge = document.createElement('div');
  connection_badge.className = 'connection-badge connection-offline';

  if (mp_ws) {
    const opp_label = create_label('OTHERS');
    opponent_col.appendChild(opp_label);

    // Reserve slots for up to 3 opponents
    for (let i = 0; i < 3; i++) {
      const { slot, name_el, status_el, renderer } = create_opponent_panel(`P${i + 2}`);
      opponent_slots.push(slot);
      opponent_labels.push(name_el);
      opponent_status_els.push(status_el);
      opponent_renderers.push(renderer);
      opponent_col.appendChild(slot);
    }
  }

  layout.appendChild(hold_col);
  layout.appendChild(board_frame);
  layout.appendChild(right_col);
  if (mp_ws) layout.appendChild(opponent_col);
  wrapper.appendChild(layout);
  if (mp_ws) wrapper.appendChild(connection_badge);
  root.appendChild(wrapper);

  const renderer = createBoardRenderer(board_canvas);
  const hold_renderer = createPreviewRenderer(hold_canvas, { showGrid: true });
  const next_renderer = createNextStackRenderer(next_canvas);
  const line_fx = new LineFx(fx_canvas);
  const harddrop_fx = new HardDropFx(fx_canvas);
  const cell = 720 / 20;

  let paused = false;
  let prev_combo = 0;
  let prev_tspin = 0;
  let prev_all_clear = 0;
  let raf_id = 0;
  let last_tick = performance.now();
  let last_fx = performance.now();
  let cleanup_keyboard: (() => void) | null = null;
  let popup_el: HTMLElement | null = null;
  let edge_active = false;
  let edge_dir = 0;
  let is_tearing_down = false;
  let collapse_cancel: (() => void) | null = null;

  const input_buffer = mp_ws ? new InputBuffer(wasm) : null;

  function replay_player_for(player_id: number): OpponentReplayPlayer {
    let player = replay_players.get(player_id);
    if (!player) {
      player = new OpponentReplayPlayer(player_id);
      replay_players.set(player_id, player);
    }
    return player;
  }

  function handle_multiplayer_packet(event: MultiplayerEvent | null) {
    if (!event) return;
    if (event.kind === 'server_replay' && typeof event.source_player_id === 'number') {
      replay_player_for(event.source_player_id).push_replay(
        event.source_player_id,
        event.events ?? [],
      );
      return;
    }
    if (event.kind === 'state_hash' && typeof event.tick === 'number') {
      for (const player of replay_players.values()) player.apply_tick(event.tick);
      finish_ws_resync();
      return;
    }
    if (event.kind === 'resync_required') {
      connection_status.value = 'resyncing';
      return;
    }
    if (event.kind === 'state_snapshot') {
      finish_ws_resync();
      return;
    }
    if (event.kind === 'reconnect_ack') {
      finish_ws_resync();
      return;
    }
    if (event.kind === 'standings') {
      match_standings.value = (event.standings ?? []) as StandingRow[];
      finish_to_result();
      return;
    }
    // Multiplayer 'game_over' is informational; the authoritative result arrives
    // via the 'standings' packet broadcast immediately after. No single-player
    // gameover transition here.
  }

  function finish_ws_resync() {
    if (mp_ws && 'finish_resync' in mp_ws && typeof mp_ws.finish_resync === 'function') {
      mp_ws.finish_resync();
    } else if (connection_status.value === 'resyncing') {
      connection_status.value = 'online';
    }
  }

  if (mp_ws) {
    // Game takes over the ws: drop lobby's onopen (else a mid-game reconnect
    // would resend a spurious join_room).
    mp_ws.onopen = null;
    mp_ws.onpacket = handle_multiplayer_packet;
  }

  function queue_multiplayer_input(action: number, pressed: boolean) {
    if (!mp_ws || !input_buffer) return;
    input_buffer.push(action, pressed);
  }

  function flush_multiplayer_input_if_ready() {
    if (!mp_ws || !input_buffer) return;
    input_buffer.advance_tick();
    if (!input_buffer.should_flush()) return;
    const replay = input_buffer.flush();
    if (replay && replay.byteLength > 0) {
      mp_ws.send(replay);
    }
  }

  function edge_bump(dir: -1 | 1) {
    if (edge_active) return;
    edge_active = true;
    edge_dir = dir;
    board_frame.style.transition = 'transform 160ms ease-out';
    board_frame.style.transform = `translateX(${dir * 14}px)`;
  }

  function edge_release() {
    if (!edge_active) return;
    edge_active = false;
    edge_dir = 0;
    board_frame.style.transition = 'transform 70ms ease-in';
    board_frame.style.transform = 'translateX(0px)';
  }

  let popup_timer: ReturnType<typeof setTimeout> | null = null;

  function show_popup(text: string) {
    if (popup_el) popup_el.remove();
    if (popup_timer) clearTimeout(popup_timer);
    popup_el = document.createElement('div');
    popup_el.className = 'combo-popup';
    popup_el.textContent = text;
    board_frame.appendChild(popup_el);
    popup_timer = setTimeout(() => {
      popup_el?.remove();
      popup_el = null;
      popup_timer = null;
    }, 1000);
  }

  function cleanup_runtime() {
    if (raf_id) {
      cancelAnimationFrame(raf_id);
      raf_id = 0;
    }
    cleanup_keyboard?.();
  }

  function finish_game() {
    if (is_tearing_down) return;
    is_tearing_down = true;
    cleanup_runtime();
    audio_manager.stop_bgm();
    collapse_cancel = run_collapse_animation(board_canvas, () => {
      collapse_cancel = null;
      destroy();
      page.value = 'gameover';
    });
  }

  // Multiplayer: local player topped out but the match continues → spectate the
  // remaining players. ws stays open (main only tears it down on 'home'); the
  // spectator screen rebinds onpacket. No collapse anim (that's single-player).
  function go_spectator() {
    if (is_tearing_down) return;
    is_tearing_down = true;
    cleanup_runtime();
    audio_manager.stop_bgm();
    page.value = 'spectator';
  }

  // Multiplayer: global match end → show the standings result screen.
  function finish_to_result() {
    if (is_tearing_down) return;
    is_tearing_down = true;
    cleanup_runtime();
    audio_manager.stop_bgm();
    page.value = 'multiplayer_result';
  }

  function advance_game(now: number) {
    if (paused || wasm.is_game_over) {
      if (wasm.is_game_over) {
        finish_game();
      }
      return;
    }

    const delta_ms = now - last_tick;
    if (delta_ms < 16) {
      return;
    }

    const prev_level = level.value;
    const prev_lines = lines.value;
    const tick_result: unknown = wasm.tick(delta_ms);
    last_tick = now;

    if (has_garbage_inserted(tick_result)) {
      shake_screen(board_frame);
    }

    const hud_after: unknown = wasm.get_hud_data();
    if (is_hud_data(hud_after)) {
      if (hud_after.lines > prev_lines) {
        audio_manager.play_sfx(hud_after.tspin > 0 ? 't_spin' : 'line_clear');
      }
      if (hud_after.level > prev_level) {
        audio_manager.play_sfx('level_up');
      }
    }

    if (wasm.is_game_over) {
      finish_game();
    }
  }

  function render_all() {
    const colors = get_theme_colors();
    wasm.update_grid();
    renderer.render(get_grid_view(), colors);
    hold_renderer.render(wasm.get_hold());
    next_renderer.render(Array.from(wasm.get_next()));

    if (mp_ws) {
      const snapshot = get_multiplayer_snapshot();
      const remote_players = snapshot?.opponents ?? [];
      opponent_grids.clear();
      for (let i = 0; i < opponent_renderers.length; i++) {
        const player = remote_players[i];
        const renderer = opponent_renderers[i];
        const label = opponent_labels[i];
        const slot = opponent_slots[i];
        if (!player || label === undefined) {
          renderer.canvas.style.display = 'none';
          continue;
        }
        label.textContent = `${player.name}${player.away ? ' (AWAY)' : player.alive ? '' : ' (KO)'}`;
        if (slot) {
          const dim =
            connection_status.value === 'slow' || connection_status.value === 'disconnected';
          slot.classList.toggle('opponent-connection-dim', dim);
          if (opponent_status_els[i]) {
            opponent_status_els[i].textContent = dim ? '仅重力' : '';
            opponent_status_els[i].style.display = dim ? '' : 'none';
          }
        }
        replay_player_for(player.player_id);
        const grid = get_opponent_player_grid(player.player_id);
        opponent_grids.set(i, grid);
        renderer.canvas.style.display = '';
      }
    }

    if (mp_ws) {
      update_connection_badge(connection_badge, connection_status.value);
    }

    for (const [idx, grid] of opponent_grids) {
      if (idx < opponent_renderers.length) {
        opponent_renderers[idx].render(grid, colors);
      }
    }

    const lock_timer = wasm.get_lock_timer();
    if (lock_timer > 0) {
      const ctx = board_canvas.getContext('2d')!;
      const pct = lock_timer / 500;
      const bar_h = 4;
      const color = pct > 0.5 ? '#4f4' : pct > 0.25 ? '#ff0' : '#f44';
      ctx.fillStyle = color;
      ctx.fillRect(0, 720 - bar_h, 360 * pct, bar_h);
    }

    const clear_mask = Number(wasm.get_last_clear_mask());
    if (clear_mask) {
      for (let row = 0; row < 20; row++) {
        if (clear_mask & (1 << row)) {
          line_fx.triggerFlash(row * cell, cell);
        }
      }
    }

    const hud_raw: unknown = wasm.get_hud_data();
    if (is_hud_data(hud_raw)) {
      hud.update(hud_raw);
      score.value = hud_raw.score;
      level.value = hud_raw.level;
      lines.value = hud_raw.lines;
      combo.value = hud_raw.combo;
      b2b_count.value = hud_raw.b2b;

      if (hud_raw.combo > 1 && hud_raw.combo !== prev_combo) show_popup(`COMBO x${hud_raw.combo}`);
      if (hud_raw.tspin > 0 && hud_raw.tspin !== prev_tspin) show_popup('T-SPIN');
      if (hud_raw.all_clear > 0 && hud_raw.all_clear !== prev_all_clear) show_popup('ALL CLEAR!');
      prev_combo = hud_raw.combo;
      prev_tspin = hud_raw.tspin;
      prev_all_clear = hud_raw.all_clear;
    }
  }

  function game_loop(time: number) {
    if (paused) {
      raf_id = requestAnimationFrame(game_loop);
      return;
    }

    if (!mp_ws) {
      advance_game(time);
      if (is_tearing_down) return;
    } else if (wasm.is_game_over) {
      go_spectator();
      return;
    } else {
      flush_multiplayer_input_if_ready();
    }

    render_all();

    const dt = time - last_fx;
    last_fx = time;
    const fx_ctx = fx_canvas.getContext('2d')!;
    fx_ctx.clearRect(0, 0, fx_canvas.width, fx_canvas.height);
    line_fx.update(dt);
    line_fx.render();
    harddrop_fx.update(dt);
    harddrop_fx.render();

    raf_id = requestAnimationFrame(game_loop);
  }

  function toggle_pause() {
    if (wasm.is_game_over) return;
    paused = !paused;
    if (paused) {
      audio_manager.stop_bgm();
      show_pause_overlay();
    } else {
      audio_manager.start_bgm(settings.value.theme);
      hide_pause_overlay();
    }
  }

  let pause_overlay_el: HTMLElement | null = null;

  function show_pause_overlay() {
    if (pause_overlay_el) return;
    pause_overlay_el = document.createElement('div');
    pause_overlay_el.className = 'pause-overlay';
    pause_overlay_el.innerHTML = `
      <div class="pause-panel glass">
        <h2 class="pause-title">PAUSED</h2>
        <button class="btn" id="btn-resume">继续游戏</button>
        <button class="btn" id="btn-restart">重新开始</button>
        <button class="btn" id="btn-menu">主菜单</button>
      </div>
    `;
    pause_overlay_el.querySelector('#btn-resume')!.addEventListener('click', () => {
      paused = false;
      hide_pause_overlay();
    });
    pause_overlay_el.querySelector('#btn-restart')!.addEventListener('click', () => {
      hide_pause_overlay();
      paused = false;
      reset_wasm();
      last_tick = performance.now();
    });
    pause_overlay_el.querySelector('#btn-menu')!.addEventListener('click', () => {
      destroy();
      page.value = 'home';
    });
    wrapper.appendChild(pause_overlay_el);
  }

  function hide_pause_overlay() {
    pause_overlay_el?.remove();
    pause_overlay_el = null;
  }

  function on_visibility_change() {
    if (document.hidden && !wasm.is_game_over && !paused && !is_multiplayer.value) {
      toggle_pause();
    }
    // Auto-unpause removed: user must manually resume.
    // Prevents BGM restart when tab returns after manual pause.
  }

  let _destroy: () => void;

  function destroy() {
    collapse_cancel?.();
    collapse_cancel = null;
    cleanup_runtime();
    // Stop BGM on any exit (e.g. topbar back), not just the pause menu.
    audio_manager.stop_bgm();
    touch_overlay.destroy();
    document.removeEventListener('visibilitychange', on_visibility_change);
    if (mp_ws?.onpacket === handle_multiplayer_packet) {
      mp_ws.onpacket = null;
    }
    hud.destroy();
    renderer.destroy();
    for (const r of opponent_renderers) r.destroy();
  }
  // eslint-disable-next-line prefer-const
  _destroy = destroy;

  const kbd_config: KeyboardConfig = {
    das_ms: settings.value.das_ms,
    arr_ms: settings.value.arr_ms,
    keymap: settings.value.keymap,
  };

  function check_harddrop_fx() {
    const hd_info: unknown = wasm.get_last_hard_drop_info();
    if (is_hard_drop_info(hd_info)) {
      const colors = get_theme_colors();
      const color = colors[hd_info.piece + 3] ?? '#c8f0ff';
      harddrop_fx.trigger(hd_info.cols, hd_info.start_y * cell, hd_info.end_y * cell, color);
      shake_screen(board_frame);
    }
  }

  cleanup_keyboard = bindKeyboard(
    {
      handleAction: (action) => {
        if (action === Actions.MoveLeft && !wasm.can_move(-1)) {
          edge_bump(-1);
          return;
        }
        if (action === Actions.MoveRight && !wasm.can_move(1)) {
          edge_bump(1);
          return;
        }
        if (action === Actions.MoveLeft && edge_dir === -1) edge_release();
        if (action === Actions.MoveRight && edge_dir === 1) edge_release();

        const prev_lines = lines.value;
        queue_multiplayer_input(action, true);
        wasm.handle_action(action);

        if (action === Actions.HardDrop) {
          check_harddrop_fx();
          const hud_now: unknown = wasm.get_hud_data();
          if (is_hud_data(hud_now) && hud_now.lines > prev_lines) {
            audio_manager.play_sfx(hud_now.tspin > 0 ? 't_spin' : 'line_clear');
          } else {
            audio_manager.play_sfx('hard_drop');
          }
        } else if (action === Actions.MoveLeft || action === Actions.MoveRight)
          audio_manager.play_sfx('move');
        else if (action === Actions.SoftDrop) audio_manager.play_sfx('soft_drop');
        else if (action === Actions.RotateCW || action === Actions.RotateCCW)
          audio_manager.play_sfx('rotate');
        else if (action === Actions.Hold) audio_manager.play_sfx('hold');
      },
      isGameOver: () => wasm.is_game_over,
      render: render_all,
      onRelease: (action) => {
        queue_multiplayer_input(action, false);
        if (action === Actions.MoveLeft) edge_release();
        if (action === Actions.MoveRight) edge_release();
      },
      onPause: toggle_pause,
    },
    kbd_config,
  );

  document.addEventListener('visibilitychange', on_visibility_change);

  const touch_overlay = create_touch_overlay(root, (action_val, pressed) => {
    if (!pressed) {
      // Always forward release so the server clears the held key (touch B7-2).
      queue_multiplayer_input(action_val, false);
      return;
    }
    if (!wasm.is_game_over && !paused) {
      queue_multiplayer_input(action_val, true);
      wasm.handle_action(action_val);
    }
  });

  last_tick = performance.now();
  last_fx = performance.now();
  render_all();
  raf_id = requestAnimationFrame(game_loop);

  return _destroy;
}

function update_connection_badge(el: HTMLElement, status: string) {
  const labels: Record<string, string> = {
    online: 'ONLINE',
    slow: 'SLOW',
    reconnecting: 'RECONNECTING',
    disconnected: 'DISCONNECTED',
    resyncing: 'RESYNCING',
    connecting: 'CONNECTING',
    offline: 'OFFLINE',
  };
  el.textContent = labels[status] ?? 'OFFLINE';
  el.className = `connection-badge connection-${status}`;
}

function run_countdown(container: HTMLElement): Promise<void> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'countdown-overlay';

    const tip_el = document.createElement('div');
    tip_el.className = 'countdown-tip';

    const num_el = document.createElement('div');
    num_el.className = 'countdown-num';

    overlay.appendChild(num_el);
    overlay.appendChild(tip_el);
    container.appendChild(overlay);

    const seq = ['3', '2', '1', 'GO!'];
    const tips = ['准备!', '空格下落', '消行!', '冲冲冲!'];
    let i = 0;

    function show_next() {
      if (i >= seq.length) {
        overlay.remove();
        resolve();
        return;
      }
      num_el.textContent = seq[i];
      tip_el.textContent = tips[i];
      num_el.style.animation = 'none';
      void num_el.offsetHeight;
      num_el.style.animation = 'countdown-pop 400ms ease-out';
      i++;
      setTimeout(show_next, 500);
    }
    show_next();
  });
}
