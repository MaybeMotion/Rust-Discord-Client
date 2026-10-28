import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { applyTheme, parseTheme, themes, type Theme } from './theme';
import './style.css';

type Message = { id: string; channel_id: string; author: { username: string }; content: string; timestamp: string };
type Channel = { id: string; guild_id?: string; name?: string; type: number };
type Snapshot = { status: string; guilds: Record<string, { id: string; name: string }>; channels: Record<string, Channel>; messages: Record<string, Message[]> };
const app = document.querySelector<HTMLDivElement>('#app')!;
app.innerHTML = `<aside><div class="brand">R / D <span>RUST DESKTOP</span></div><h1>Your spaces</h1><nav aria-label="Channels"></nav><div class="appearance"><label for="theme">Appearance</label><select id="theme"></select><label class="import" for="import">Import theme JSON</label><input id="import" type="file" accept="application/json,.json"><p id="theme-error" role="alert"></p></div></aside><main><header><div><small>WORKSPACE / MESSAGES</small><h2 id="channel-title">Welcome</h2></div><span id="status" role="status">Starting…</span></header><section id="messages" aria-label="Messages" role="log"></section><form><label for="message">Message</label><div class="composer"><textarea id="message" rows="2" maxlength="2000" placeholder="Write a message…"></textarea><button type="submit">Send ↗</button></div><p id="error" role="alert"></p></form></main>`;
const select = document.querySelector<HTMLSelectElement>('#theme')!;
for (const [key, value] of Object.entries(themes)) select.add(new Option(value.name, key));
let custom: Theme | undefined;
try { const saved = localStorage.getItem('theme'); if (saved) custom = parseTheme(JSON.parse(saved)); } catch { /* Ignore old/corrupt preferences. */ }
if (custom) { select.add(new Option(custom.name, 'custom')); select.value = 'custom'; }
applyTheme(custom ?? themes.dark);
select.onchange = () => {
  const theme = select.value === 'custom' ? custom! : themes[select.value];
  applyTheme(theme); localStorage.setItem('theme', JSON.stringify(theme));
};
document.querySelector<HTMLInputElement>('#import')!.onchange = async event => {
  const file = (event.target as HTMLInputElement).files?.[0];
  try {
    if (!file) return;
    if (file.size > 8192) throw new Error('Theme must be at most 8 KB');
    custom = parseTheme(JSON.parse(await file.text()));
    if (![...select.options].some(o => o.value === 'custom')) select.add(new Option('Custom', 'custom'));
    select.value = 'custom'; select.onchange!(new Event('change'));
    document.querySelector('#theme-error')!.textContent = '';
  } catch (error) { document.querySelector('#theme-error')!.textContent = String(error); }
};
let current = '';
let state: Snapshot;
let refreshInFlight = false;
let dirty = false;
const messageList = document.querySelector<HTMLElement>('#messages')!;
function renderMessages() {
  const nearBottom = messageList.scrollHeight - messageList.scrollTop - messageList.clientHeight < 100;
  const fragment = document.createDocumentFragment();
  for (const message of (state.messages[current] ?? []).slice(-200)) {
    const article = document.createElement('article');
    const author = document.createElement('strong'); author.textContent = message.author.username;
    const time = document.createElement('time'); time.dateTime = message.timestamp;
    time.textContent = new Date(message.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const content = document.createElement('p'); content.textContent = message.content || '[No text content]';
    article.append(author, time, content); fragment.append(article);
  }
  messageList.replaceChildren(fragment);
  if (nearBottom) messageList.scrollTop = messageList.scrollHeight;
}
async function choose(channel: Channel) {
  current = channel.id;
  document.querySelector('#channel-title')!.textContent = `# ${channel.name ?? channel.id}`;
  document.querySelector('#error')!.textContent = '';
  renderMessages();
  try {
    await invoke<Message[]>('history', { channel: channel.id });
    await refresh();
  } catch (error) { document.querySelector('#error')!.textContent = String(error); }
}
async function refresh() {
  dirty = true;
  if (refreshInFlight) return;
  refreshInFlight = true;
  try {
    while (dirty) {
      dirty = false;
      state = await invoke<Snapshot>('snapshot');
      document.querySelector('#status')!.textContent = state.status;
      const nav = document.querySelector('nav')!;
      nav.replaceChildren();
      const channels = Object.values(state.channels).filter(c => c.type === 0 || c.type === 1 || c.type === 5);
      for (const channel of channels) {
        const button = document.createElement('button');
        const guild = channel.guild_id ? state.guilds[channel.guild_id]?.name : 'Direct';
        button.textContent = `${guild ?? 'Server'} / # ${channel.name ?? channel.id}`;
        button.setAttribute('aria-current', String(current === channel.id));
        button.onclick = () => { void choose(channel); }; nav.append(button);
      }
      renderMessages();
      if (!current && channels.length) void choose(channels[0]);
    }
  } finally { refreshInFlight = false; }
}
document.querySelector('form')!.onsubmit = async event => {
  event.preventDefault();
  const input = document.querySelector<HTMLTextAreaElement>('#message')!;
  const button = document.querySelector<HTMLButtonElement>('button[type=submit]')!;
  if (!current || !input.value.trim() || button.disabled) return;
  button.disabled = true;
  try {
    await invoke('send_message', { channel: current, content: input.value });
    input.value = ''; document.querySelector('#error')!.textContent = ''; await refresh();
  } catch (error) { document.querySelector('#error')!.textContent = String(error); }
  finally { button.disabled = false; }
};
async function start() {
  await listen('state-changed', () => { void refresh().catch(error => { document.querySelector('#error')!.textContent = String(error); }); });
  await refresh();
}
void start().catch(error => { document.querySelector('#error')!.textContent = `Open via npm run tauri dev for the native backend. ${String(error)}`; });
